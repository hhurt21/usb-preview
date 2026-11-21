//! NVMe SSD の健康記録 (SMART / Health Information ログページ) を直接読む。読むだけで何も書き込まない。
//! 管理者の権限が要る (\\.\PhysicalDriveN を開くため)。NVMe 以外 (SATA・USB の箱など) は読めないことがある。

use serde::Serialize;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE};
use windows::Win32::Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
use windows::Win32::System::IO::DeviceIoControl;

const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D_1400;
const STORAGE_DEVICE_PROTOCOL_SPECIFIC_PROPERTY: u32 = 50;
const PROTOCOL_TYPE_NVME: u32 = 3;
const NVME_DATA_TYPE_LOG_PAGE: u32 = 2;
const NVME_LOG_PAGE_HEALTH_INFO: u32 = 2;
const LOG_LEN: usize = 512;
/// STORAGE_PROTOCOL_SPECIFIC_DATA の大きさ (u32 が 10 個)
const SPEC_LEN: usize = 40;

#[derive(Serialize, Clone, Debug, Default)]
pub struct NvmeHealth {
    /// \\.\PhysicalDriveN の N (Get-PhysicalDisk の DeviceId と同じ)
    pub number: u32,
    pub power_on_hours: u64,
    pub power_cycles: u64,
    pub unsafe_shutdowns: u64,
    pub media_errors: u64,
    /// 消耗度 (%)。100 を超えることもある
    pub percent_used: u8,
    pub available_spare: u8,
    pub temp_c: i32,
    /// 書き込んだ量・読んだ量 (バイト)
    pub written_bytes: u64,
    pub read_bytes: u64,
    pub critical_warning: u8,
}

fn u64_at(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

/// 健康記録 512 バイトを読みやすい形に (NVMe の仕様の位置)
pub fn parse_health(number: u32, log: &[u8]) -> Option<NvmeHealth> {
    if log.len() < LOG_LEN {
        return None;
    }
    let kelvin = u16::from_le_bytes([log[1], log[2]]) as i32;
    Some(NvmeHealth {
        number,
        critical_warning: log[0],
        temp_c: if kelvin > 0 { kelvin - 273 } else { 0 },
        available_spare: log[3],
        percent_used: log[5],
        // データ単位は 1000 × 512 バイト
        read_bytes: u64_at(log, 32).saturating_mul(512_000),
        written_bytes: u64_at(log, 48).saturating_mul(512_000),
        power_cycles: u64_at(log, 112),
        power_on_hours: u64_at(log, 128),
        unsafe_shutdowns: u64_at(log, 144),
        media_errors: u64_at(log, 160),
    })
}

fn read_one(number: u32) -> Option<NvmeHealth> {
    let path: Vec<u16> = format!(r"\\.\PhysicalDrive{number}").encode_utf16().chain([0]).collect();
    unsafe {
        let h = CreateFileW(
            PCWSTR(path.as_ptr()),
            (GENERIC_READ | GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
        .ok()?;
        // 問い合わせ: STORAGE_PROPERTY_QUERY (PropertyId, QueryType) の後ろに STORAGE_PROTOCOL_SPECIFIC_DATA
        let mut buf = vec![0u8; 8 + SPEC_LEN + LOG_LEN];
        let put = |b: &mut [u8], off: usize, v: u32| b[off..off + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut buf, 0, STORAGE_DEVICE_PROTOCOL_SPECIFIC_PROPERTY);
        put(&mut buf, 4, 0); // PropertyStandardQuery
        put(&mut buf, 8, PROTOCOL_TYPE_NVME);
        put(&mut buf, 12, NVME_DATA_TYPE_LOG_PAGE);
        put(&mut buf, 16, NVME_LOG_PAGE_HEALTH_INFO);
        put(&mut buf, 20, 0); // ProtocolDataRequestSubValue
        put(&mut buf, 24, SPEC_LEN as u32); // ProtocolDataOffset (この構造体の頭から)
        put(&mut buf, 28, LOG_LEN as u32); // ProtocolDataLength
        let mut ret = 0u32;
        let len = buf.len() as u32;
        let ok = DeviceIoControl(
            h,
            IOCTL_STORAGE_QUERY_PROPERTY,
            Some(buf.as_ptr() as *const _),
            len,
            Some(buf.as_mut_ptr() as *mut _),
            len,
            Some(&mut ret),
            None,
        )
        .is_ok();
        let _ = CloseHandle(h);
        if !ok {
            return None;
        }
        // 返事: STORAGE_PROTOCOL_DATA_DESCRIPTOR (Version, Size) の後ろに同じ構造体。データはその頭から Offset の位置
        let off = u32::from_le_bytes(buf[8 + 16..8 + 20].try_into().ok()?) as usize;
        let start = 8 + off;
        parse_health(number, buf.get(start..start + LOG_LEN)?)
    }
}

/// つながっているディスクを順に読む (読めた NVMe だけ返す)
pub fn read_all() -> Vec<NvmeHealth> {
    (0..16).filter_map(read_one).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nvme_health_log() {
        let mut log = vec![0u8; 512];
        log[1..3].copy_from_slice(&(318u16).to_le_bytes()); // 318K = 45℃
        log[3] = 100;
        log[5] = 3;
        log[48..56].copy_from_slice(&(20_000_000u64).to_le_bytes());
        log[112..120].copy_from_slice(&(1_234u64).to_le_bytes());
        log[128..136].copy_from_slice(&(10_805u64).to_le_bytes());
        log[144..152].copy_from_slice(&(56u64).to_le_bytes());
        let h = parse_health(1, &log).unwrap();
        assert_eq!(h.temp_c, 45);
        assert_eq!(h.percent_used, 3);
        assert_eq!(h.power_cycles, 1234);
        assert_eq!(h.power_on_hours, 10805);
        assert_eq!(h.unsafe_shutdowns, 56);
        assert_eq!(h.written_bytes, 20_000_000 * 512_000);
        assert!(parse_health(0, &log[..100]).is_none());
    }
}
