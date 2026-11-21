//! USB ハブから各ポートの接続速度 (USB 1.1 / 2.0 / 3.0 / 3.1+) と消費電力 (mA) を読む。
//! 読むだけで管理者権限は不要。

use serde::Serialize;
use std::collections::HashMap;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::*;
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;

const GUID_DEVINTERFACE_USB_HUB: GUID = GUID::from_u128(0xf18a0e88_c30c_11d0_8815_00a0c906bed8);
const IOCTL_USB_GET_NODE_INFORMATION: u32 = 0x0022_0408;
const IOCTL_USB_GET_DESCRIPTOR_FROM_NODE_CONNECTION: u32 = 0x0022_0410;
const IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX: u32 = 0x0022_0448;
const IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX_V2: u32 = 0x0022_045C;

#[derive(Serialize, Clone, Debug, Default)]
pub struct UsbPortInfo {
    /// low / full / high / super / super_plus
    pub speed: String,
    /// 例: "USB 3.0 (5 Gbps)"
    pub speed_label: String,
    /// 機器が要求する最大電流 (mA)
    pub max_power_ma: Option<u32>,
    /// デバイス記述子の bcdUSB (例: 0x0300, 0x0210)
    pub bcd_usb: u16,
    /// USB 3.0 以上に対応しているのに USB 2.0 以下でつながっている
    pub slow_port: bool,
    pub is_hub: bool,
}

fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// 速度コード + V2 フラグ + bcdUSB から表示用の速度・刺し間違い判定を作る
pub fn classify_speed(speed_code: u8, bcd_usb: u16, v2_flags: Option<u32>) -> (String, String, bool) {
    let super_plus = v2_flags.map(|f| (f & 0x04) != 0).unwrap_or(false);
    let super_op = speed_code >= 3 || v2_flags.map(|f| (f & 0x01) != 0).unwrap_or(false);
    // USB 3 に対応しているか。Windows のくわしい答え (V2) があればそれだけを使う。
    // bcdUSB 0x0210 (USB 2.1) は省電力 (LPM) に対応した USB 2 の機器もよく名乗るので、USB 3 の印にしない
    let super_cap = match v2_flags {
        Some(f) => (f & 0x02) != 0,
        None => bcd_usb >= 0x0300,
    };

    let (speed, label) = if super_plus {
        ("super_plus", "USB 3.1+ (10 Gbps)")
    } else if super_op {
        ("super", "USB 3.0 (5 Gbps)")
    } else {
        match speed_code {
            0 => ("low", "USB 1.1 (1.5 Mbps)"),
            1 => ("full", "USB 1.1 (12 Mbps)"),
            2 => ("high", "USB 2.0 (480 Mbps)"),
            _ => ("unknown", "不明"),
        }
    };
    // USB 3.0 対応機器 (bcdUSB >= 0x0210 または V2 フラグ) なのに USB 2.0 以下で動いている
    let slow_port = super_cap && !super_op;
    (speed.to_string(), label.to_string(), slow_port)
}

unsafe fn open_hub(path: &[u16]) -> Option<HANDLE> {
    for access in [GENERIC_WRITE.0, GENERIC_READ.0, 0u32] {
        if let Ok(h) = CreateFileW(
            PCWSTR(path.as_ptr()),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        ) {
            if !h.is_invalid() {
                return Some(h);
            }
        }
    }
    None
}

unsafe fn query_hub_ports(h: HANDLE, hub_id: &str, out: &mut HashMap<String, UsbPortInfo>) {
    let mut node_info = [0u8; 76];
    let mut ret = 0u32;
    if DeviceIoControl(
        h,
        IOCTL_USB_GET_NODE_INFORMATION,
        Some(node_info.as_ptr() as *const _),
        node_info.len() as u32,
        Some(node_info.as_mut_ptr() as *mut _),
        node_info.len() as u32,
        Some(&mut ret),
        None,
    )
    .is_err()
    {
        return;
    }
    let num_ports = node_info[6] as u32;
    if num_ports == 0 || num_ports > 64 {
        return;
    }

    let hub_up = hub_id.to_ascii_uppercase();
    let mut conn_buf = vec![0u8; 1024];

    for port in 1..=num_ports {
        conn_buf.fill(0);
        conn_buf[0..4].copy_from_slice(&port.to_le_bytes());
        let mut cret = 0u32;
        if DeviceIoControl(
            h,
            IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX,
            Some(conn_buf.as_ptr() as *const _),
            conn_buf.len() as u32,
            Some(conn_buf.as_mut_ptr() as *mut _),
            conn_buf.len() as u32,
            Some(&mut cret),
            None,
        )
        .is_err()
        {
            continue;
        }
        if cret < 35 {
            continue;
        }
        // ConnectionStatus == 1 (DeviceConnected)
        let status = u32::from_le_bytes(conn_buf[31..35].try_into().unwrap());
        if status != 1 {
            continue;
        }
        let bcd_usb = u16::from_le_bytes([conn_buf[6], conn_buf[7]]);
        let speed_code = conn_buf[23];
        let is_hub = conn_buf[24] != 0;

        // V2 で 10Gbps や SuperSpeed 対応可否も調べる (対応していないコントローラーでも無視して続行)
        let mut v2 = [0u8; 16];
        v2[0..4].copy_from_slice(&port.to_le_bytes());
        v2[4..8].copy_from_slice(&16u32.to_le_bytes());
        v2[8..12].copy_from_slice(&0x07u32.to_le_bytes());
        let mut v2ret = 0u32;
        let v2_flags = if DeviceIoControl(
            h,
            IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX_V2,
            Some(v2.as_ptr() as *const _),
            16,
            Some(v2.as_mut_ptr() as *mut _),
            16,
            Some(&mut v2ret),
            None,
        )
        .is_ok()
            && v2ret >= 16
        {
            Some(u32::from_le_bytes(v2[12..16].try_into().unwrap()))
        } else {
            None
        };

        let (speed, speed_label, slow_port) = classify_speed(speed_code, bcd_usb, v2_flags);

        // Configuration Descriptor から最大消費電力 (bMaxPower) を読む
        let mut desc = [0u8; 12 + 9];
        desc[0..4].copy_from_slice(&port.to_le_bytes());
        desc[4] = 0x80;
        desc[5] = 0x06;
        desc[6..8].copy_from_slice(&0x0200u16.to_le_bytes());
        desc[10..12].copy_from_slice(&9u16.to_le_bytes());
        let mut dret = 0u32;
        let max_power_ma = if DeviceIoControl(
            h,
            IOCTL_USB_GET_DESCRIPTOR_FROM_NODE_CONNECTION,
            Some(desc.as_ptr() as *const _),
            desc.len() as u32,
            Some(desc.as_mut_ptr() as *mut _),
            desc.len() as u32,
            Some(&mut dret),
            None,
        )
        .is_ok()
            && dret >= 21
            && desc[13] == 2
        {
            let raw = desc[20] as u32;
            let unit = if speed == "super" || speed == "super_plus" { 8 } else { 2 };
            Some(raw * unit)
        } else {
            None
        };

        out.insert(
            format!("{hub_up}|{port}"),
            UsbPortInfo {
                speed,
                speed_label,
                max_power_ma,
                bcd_usb,
                slow_port: slow_port && !is_hub,
                is_hub,
            },
        );
    }
}

/// すべての USB ハブの接続ポート情報を返す。キーは「ハブの ID (大文字)|ポート番号」。
pub fn query_all() -> HashMap<String, UsbPortInfo> {
    let mut out = HashMap::new();
    unsafe {
        let Ok(h) = SetupDiGetClassDevsW(
            Some(&GUID_DEVINTERFACE_USB_HUB),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        ) else {
            return out;
        };
        let mut i = 0u32;
        loop {
            let mut iface = SP_DEVICE_INTERFACE_DATA {
                cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };
            if SetupDiEnumDeviceInterfaces(h, None, &GUID_DEVINTERFACE_USB_HUB, i, &mut iface).is_err() {
                break;
            }
            i += 1;

            let mut req = 0u32;
            let _ = SetupDiGetDeviceInterfaceDetailW(h, &iface, None, 0, Some(&mut req), None);
            if req < 8 {
                continue;
            }
            let mut buf = vec![0u8; req as usize];
            let detail = buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            // 64bit では cbSize = 8, 32bit では 6
            (*detail).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            let mut devinfo = SP_DEVINFO_DATA {
                cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
                ..Default::default()
            };
            if SetupDiGetDeviceInterfaceDetailW(
                h,
                &iface,
                Some(detail),
                req,
                None,
                Some(&mut devinfo),
            )
            .is_err()
            {
                continue;
            }
            let mut idbuf = [0u16; 512];
            if CM_Get_Device_IDW(devinfo.DevInst, &mut idbuf, 0) != CR_SUCCESS {
                continue;
            }
            let hub_id = from_wide(&idbuf);
            let path_ptr = std::ptr::addr_of!((*detail).DevicePath) as *const u16;
            let mut plen = 0usize;
            while *path_ptr.add(plen) != 0 {
                plen += 1;
            }
            let path: Vec<u16> = std::slice::from_raw_parts(path_ptr, plen + 1).to_vec();
            if let Some(hub_h) = open_hub(&path) {
                query_hub_ports(hub_h, &hub_id, &mut out);
                let _ = CloseHandle(hub_h);
            }
        }
        let _ = SetupDiDestroyDeviceInfoList(h);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_usb3_device_on_usb2_port() {
        let (sp, lbl, slow) = classify_speed(2, 0x0210, Some(0x02));
        assert_eq!(sp, "high");
        assert!(lbl.contains("2.0"));
        assert!(slow);
    }

    #[test]
    fn usb21_device_is_not_flagged_as_usb3() {
        // USB 2.1 を名乗る USB 2 の機器 (キーボードなど) は、刺し間違いにしない
        let (_, _, slow_v2) = classify_speed(2, 0x0210, Some(0x00));
        assert!(!slow_v2);
        let (_, _, slow_no_v2) = classify_speed(2, 0x0210, None);
        assert!(!slow_no_v2);
        // V2 が無くても USB 3.0 を名乗っていれば刺し間違い
        let (_, _, slow30) = classify_speed(2, 0x0300, None);
        assert!(slow30);
    }

    #[test]
    fn normal_usb3_and_usb2_no_warning() {
        let (sp3, _, slow3) = classify_speed(3, 0x0300, Some(0x03));
        assert_eq!(sp3, "super");
        assert!(!slow3);

        let (sp2, _, slow2) = classify_speed(2, 0x0200, Some(0x00));
        assert_eq!(sp2, "high");
        assert!(!slow2);

        let (sp31, lbl31, slow31) = classify_speed(3, 0x0310, Some(0x0F));
        assert_eq!(sp31, "super_plus");
        assert!(lbl31.contains("10 Gbps"));
        assert!(!slow31);
    }
}
