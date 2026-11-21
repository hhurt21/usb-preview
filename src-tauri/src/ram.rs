//! RAM の使用状況・解放 (Microsoft PC Manager のブースト相当) と、
//! 定期自動解放 (スマートブースト)・一時ファイル (%TEMP%) のお掃除。

use crate::store;
use serde::Serialize;
use std::ffi::c_void;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, SetProcessWorkingSetSize,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA,
};

#[link(name = "ntdll")]
extern "system" {
    fn NtSetSystemInformation(info_class: u32, info: *mut c_void, len: u32) -> i32;
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct RamStatus {
    pub total_mb: u64,
    pub used_mb: u64,
    pub avail_mb: u64,
    pub used_pct: u32,
    pub last_boost_time: Option<i64>,
    pub last_freed_mb: u64,
    pub auto_boost_count: usize,
    pub temp_mb: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct BoostResult {
    pub before_used_mb: u64,
    pub after_used_mb: u64,
    pub before_pct: u32,
    pub after_pct: u32,
    pub freed_mb: u64,
    pub trimmed_procs: usize,
    pub purged_standby: bool,
    pub temp_freed_mb: u64,
}

#[derive(Default)]
struct RamHistory {
    last_boost_time: Option<i64>,
    last_freed_mb: u64,
    auto_boost_count: usize,
}

static HIST: OnceLock<Mutex<RamHistory>> = OnceLock::new();
fn hist() -> &'static Mutex<RamHistory> {
    HIST.get_or_init(|| Mutex::new(RamHistory::default()))
}

fn read_mem() -> (u64, u64, u64, u32) {
    let mut m = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe {
        if GlobalMemoryStatusEx(&mut m).is_err() {
            return (0, 0, 0, 0);
        }
    }
    let mb = |b: u64| b / (1024 * 1024);
    let total = mb(m.ullTotalPhys);
    let avail = mb(m.ullAvailPhys);
    let used = total.saturating_sub(avail);
    (total, used, avail, m.dwMemoryLoad)
}

fn dir_size_limited(dir: &Path, depth: usize, max_entries: &mut usize) -> u64 {
    if depth > 4 || *max_entries == 0 {
        return 0;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut total = 0u64;
    for ent in rd.flatten() {
        if *max_entries == 0 {
            break;
        }
        *max_entries -= 1;
        let Ok(md) = ent.metadata() else { continue };
        if md.is_file() {
            total = total.saturating_add(md.len());
        } else if md.is_dir() {
            total = total.saturating_add(dir_size_limited(&ent.path(), depth + 1, max_entries));
        }
    }
    total
}

fn temp_dir_mb() -> u64 {
    let mut budget = 4000usize;
    let bytes = dir_size_limited(&std::env::temp_dir(), 0, &mut budget);
    bytes / (1024 * 1024)
}

pub fn status(with_temp: bool) -> RamStatus {
    let (total_mb, used_mb, avail_mb, used_pct) = read_mem();
    let h = hist().lock().unwrap();
    RamStatus {
        total_mb,
        used_mb,
        avail_mb,
        used_pct,
        last_boost_time: h.last_boost_time,
        last_freed_mb: h.last_freed_mb,
        auto_boost_count: h.auto_boost_count,
        temp_mb: if with_temp { temp_dir_mb() } else { 0 },
    }
}

unsafe fn enable_privilege(name: &str) -> bool {
    let mut token = HANDLE::default();
    if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token).is_err() {
        return false;
    }
    let wname: Vec<u16> = name.encode_utf16().chain([0]).collect();
    let mut luid = LUID::default();
    let ok = LookupPrivilegeValueW(None, PCWSTR(wname.as_ptr()), &mut luid).is_ok() && {
        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None).is_ok()
    };
    let _ = CloseHandle(token);
    ok
}

/// システムのスタンバイリスト / ワーキングセットを整理 (管理者権限のときのみ成功)
fn purge_system_memory_lists() -> bool {
    const SYSTEM_MEMORY_LIST_INFORMATION: u32 = 80;
    const MEMORY_EMPTY_WORKING_SETS: u32 = 2;
    const MEMORY_PURGE_STANDBY_LIST: u32 = 4;
    unsafe {
        if !enable_privilege("SeProfileSingleProcessPrivilege") {
            return false;
        }
        let mut cmd1 = MEMORY_EMPTY_WORKING_SETS;
        let r1 = NtSetSystemInformation(
            SYSTEM_MEMORY_LIST_INFORMATION,
            &mut cmd1 as *mut u32 as *mut c_void,
            std::mem::size_of::<u32>() as u32,
        );
        let mut cmd2 = MEMORY_PURGE_STANDBY_LIST;
        let r2 = NtSetSystemInformation(
            SYSTEM_MEMORY_LIST_INFORMATION,
            &mut cmd2 as *mut u32 as *mut c_void,
            std::mem::size_of::<u32>() as u32,
        );
        r1 == 0 || r2 == 0
    }
}

/// 実行中の各プロセスの未使用ワーキングセットを解放する (管理者権限がなくても自分のアプリは全部効く)
fn trim_all_working_sets() -> usize {
    let mut trimmed = 0usize;
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return 0;
        };
        let mut e = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                let pid = e.th32ProcessID;
                if pid > 4 {
                    if let Ok(h) = OpenProcess(PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                        if SetProcessWorkingSetSize(h, usize::MAX, usize::MAX).is_ok() {
                            trimmed += 1;
                        }
                        let _ = CloseHandle(h);
                    }
                }
                if Process32NextW(snap, &mut e).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    trimmed
}

/// これより新しい一時ファイルは消さない (インストールの途中や、動いているアプリが使っている物を守るため。
/// Windows のストレージ センサーと同じ考え方)
const TEMP_MIN_AGE: Duration = Duration::from_secs(24 * 3600);

fn old_enough(md: &std::fs::Metadata) -> bool {
    md.modified()
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|age| age >= TEMP_MIN_AGE)
        .unwrap_or(false)
}

/// フォルダーの中を 1 つずつ見て、古いファイルだけ消し、空になったフォルダーだけ消す。消した量 (バイト) を返す
fn clean_dir(dir: &Path, depth: usize) -> u64 {
    if depth > 12 {
        return 0;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    let mut freed = 0u64;
    for ent in rd.flatten() {
        let p = ent.path();
        // シンボリック リンクやジャンクションの先はたどらない (ほかの場所を消さないため)
        let Ok(md) = std::fs::symlink_metadata(&p) else { continue };
        if md.file_type().is_symlink() {
            continue;
        }
        if md.is_file() {
            if old_enough(&md) && std::fs::remove_file(&p).is_ok() {
                freed = freed.saturating_add(md.len());
            }
        } else if md.is_dir() {
            freed = freed.saturating_add(clean_dir(&p, depth + 1));
            // 中身が空になって、フォルダー自体も古ければ消す (使用中・中身が残っていれば失敗してそのまま)
            if old_enough(&md) {
                let _ = std::fs::remove_dir(&p);
            }
        }
    }
    freed
}

/// %TEMP% の中で、24 時間以上さわられていない一時ファイルだけを消す (新しい物・使用中の物はそのまま残す)
pub fn clean_temp_files() -> u64 {
    clean_dir(&std::env::temp_dir(), 0) / (1024 * 1024)
}

pub fn boost(clean_temp: bool, is_auto: bool) -> BoostResult {
    let (_total, before_used_mb, _avail, before_pct) = read_mem();
    let trimmed_procs = trim_all_working_sets();
    let purged_standby = purge_system_memory_lists();
    let temp_freed_mb = if clean_temp { clean_temp_files() } else { 0 };
    std::thread::sleep(Duration::from_millis(120));
    let (_total2, after_used_mb, _avail2, after_pct) = read_mem();
    let freed_mb = before_used_mb.saturating_sub(after_used_mb);

    {
        let mut h = hist().lock().unwrap();
        h.last_boost_time = Some(crate::watch::now_ms());
        h.last_freed_mb = freed_mb;
        if is_auto {
            h.auto_boost_count += 1;
        }
    }

    BoostResult {
        before_used_mb,
        after_used_mb,
        before_pct,
        after_pct,
        freed_mb,
        trimmed_procs,
        purged_standby,
        temp_freed_mb,
    }
}

/// 設定が ON のとき、指定間隔ごとに RAM 使用率を見て自動で解放する裏スレッド
pub fn start_auto_worker(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_run = Instant::now() - Duration::from_secs(3600);
        loop {
            std::thread::sleep(Duration::from_secs(20));
            let s = store::load_settings();
            if !s.auto_ram_boost {
                continue;
            }
            let interval = Duration::from_secs(u64::from(s.auto_ram_interval_min.max(1)) * 60);
            if last_run.elapsed() < interval {
                continue;
            }
            let (_, _, _, pct) = read_mem();
            if s.auto_ram_threshold_pct > 0 && pct < s.auto_ram_threshold_pct {
                continue;
            }
            last_run = Instant::now();
            let res = boost(false, true);
            let _ = app.emit("ram-boosted", &res);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_clean_keeps_new_files() {
        let dir = std::env::temp_dir().join(format!("usbwatch_test_{}", std::process::id()));
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(dir.join("new.txt"), b"keep").unwrap();
        std::fs::write(sub.join("new2.txt"), b"keep").unwrap();
        // 作ったばかり (24 時間たっていない) なので、何も消えない
        assert_eq!(clean_dir(&dir, 0), 0);
        assert!(dir.join("new.txt").exists());
        assert!(sub.join("new2.txt").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_memory_status() {
        let st = status(false);
        assert!(st.total_mb > 0);
        assert!(st.used_pct <= 100);
    }
}
