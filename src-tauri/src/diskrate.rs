//! 今のディスクへの書き込みの速さ (Windows のパフォーマンス カウンター。タスク マネージャーと同じ値)。
//! 速さは「前に読んだときからの差」で出るので、1 回目は 0 になる。画面から 2 秒ごとに呼ぶ。

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Performance::*;
use windows::Win32::System::Threading::{
    GetProcessIoCounters, OpenProcess, IO_COUNTERS, PROCESS_QUERY_LIMITED_INFORMATION,
};

#[derive(Serialize, Clone, Debug)]
pub struct DiskRate {
    /// "0 C:" のようなディスクの名前
    pub name: String,
    pub write_bytes_per_sec: f64,
    pub read_bytes_per_sec: f64,
}

struct Query {
    q: isize,
    write: isize,
    read: isize,
}
// PDH のハンドルはこのモジュールの中でだけ、Mutex を通して使う
unsafe impl Send for Query {}

static QUERY: Mutex<Option<Query>> = Mutex::new(None);

unsafe fn open() -> Option<Query> {
    let mut q = 0isize;
    if PdhOpenQueryW(windows::core::PCWSTR::null(), 0, &mut q) != 0 {
        return None;
    }
    let mut write = 0isize;
    let mut read = 0isize;
    if PdhAddEnglishCounterW(q, w!("\\PhysicalDisk(*)\\Disk Write Bytes/sec"), 0, &mut write) != 0
        || PdhAddEnglishCounterW(q, w!("\\PhysicalDisk(*)\\Disk Read Bytes/sec"), 0, &mut read) != 0
    {
        let _ = PdhCloseQuery(q);
        return None;
    }
    let _ = PdhCollectQueryData(q);
    Some(Query { q, write, read })
}

unsafe fn values(counter: isize) -> Vec<(String, f64)> {
    let mut size = 0u32;
    let mut count = 0u32;
    let _ = PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, None);
    if size == 0 {
        return Vec::new();
    }
    // 名前の文字列も同じ領域に入るので、バイト数で取る
    let mut buf = vec![0u8; size as usize];
    let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
    if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, Some(items)) != 0 {
        return Vec::new();
    }
    (0..count as usize)
        .filter_map(|i| {
            let it = &*items.add(i);
            let name = it.szName.to_string().ok()?;
            if name == "_Total" {
                return None;
            }
            Some((name, it.FmtValue.Anonymous.doubleValue))
        })
        .collect()
}

pub fn rates() -> Result<Vec<DiskRate>, String> {
    let mut guard = QUERY.lock().map_err(|e| e.to_string())?;
    unsafe {
        if guard.is_none() {
            *guard = open();
        }
        let Some(q) = guard.as_ref() else {
            return Err("パフォーマンス カウンターを開けませんでした".into());
        };
        if PdhCollectQueryData(q.q) != 0 {
            return Err("読めませんでした".into());
        }
        let reads = values(q.read);
        let mut out: Vec<DiskRate> = values(q.write)
            .into_iter()
            .map(|(name, w)| DiskRate {
                read_bytes_per_sec: reads.iter().find(|(n, _)| *n == name).map(|r| r.1).unwrap_or(0.0),
                write_bytes_per_sec: w,
                name,
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct ProcIoRate {
    pub name: String,
    pub pids: usize,
    pub write_bytes_per_sec: f64,
    pub read_bytes_per_sec: f64,
    /// 計測開始からの合計書き込み量 (バイト)
    pub total_written: u64,
}

#[derive(Default)]
struct ProcTracker {
    last_time: Option<Instant>,
    last_pid: HashMap<u32, (String, u64, u64)>,
    totals: HashMap<String, u64>,
}

static PROC_TRACKER: Mutex<Option<ProcTracker>> = Mutex::new(None);

pub fn reset_proc_totals() {
    if let Ok(mut g) = PROC_TRACKER.lock() {
        if let Some(t) = g.as_mut() {
            t.totals.clear();
        }
    }
}

/// アプリごとの今の書き込み・読み込みの速さ (上位 12 アプリ)。
/// Chrome のようにプロセスがたくさんあるアプリは名前ごとに合算する。
pub fn proc_io_rates() -> Result<Vec<ProcIoRate>, String> {
    let mut cur: HashMap<u32, (String, u64, u64)> = HashMap::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(|e| e.to_string())?;
        let mut e = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                let pid = e.th32ProcessID;
                if pid > 4 {
                    let end = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                    let raw = String::from_utf16_lossy(&e.szExeFile[..end]);
                    let name = raw
                        .trim_end_matches(".exe")
                        .trim_end_matches(".EXE")
                        .to_string();
                    if let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                        let mut io = IO_COUNTERS::default();
                        if GetProcessIoCounters(h, &mut io).is_ok() {
                            cur.insert(pid, (name, io.WriteTransferCount, io.ReadTransferCount));
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

    let now = Instant::now();
    let mut guard = PROC_TRACKER.lock().map_err(|e| e.to_string())?;
    let tr = guard.get_or_insert_with(ProcTracker::default);
    let dt = tr
        .last_time
        .map(|t| now.duration_since(t).as_secs_f64())
        .unwrap_or(0.0);

    // 名前ごとに合算: (pids, dw, dr)
    let mut agg: HashMap<String, (usize, u64, u64)> = HashMap::new();
    for (pid, (name, w, r)) in &cur {
        let entry = agg.entry(name.clone()).or_insert((0, 0, 0));
        entry.0 += 1;
        if dt > 0.1 {
            if let Some((prev_name, pw, pr)) = tr.last_pid.get(pid) {
                if prev_name == name {
                    let dw = w.saturating_sub(*pw);
                    let dr = r.saturating_sub(*pr);
                    entry.1 = entry.1.saturating_add(dw);
                    entry.2 = entry.2.saturating_add(dr);
                    if dw > 0 {
                        *tr.totals.entry(name.clone()).or_insert(0) += dw;
                    }
                }
            }
        }
    }

    tr.last_time = Some(now);
    tr.last_pid = cur;

    let mut out: Vec<ProcIoRate> = agg
        .into_iter()
        .filter_map(|(name, (pids, dw, dr))| {
            let total_written = *tr.totals.get(&name).unwrap_or(&0);
            if dw == 0 && total_written == 0 {
                return None;
            }
            Some(ProcIoRate {
                name,
                pids,
                write_bytes_per_sec: if dt > 0.1 { dw as f64 / dt } else { 0.0 },
                read_bytes_per_sec: if dt > 0.1 { dr as f64 / dt } else { 0.0 },
                total_written,
            })
        })
        .collect();
    out.sort_by(|a, b| {
        b.write_bytes_per_sec
            .partial_cmp(&a.write_bytes_per_sec)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total_written.cmp(&a.total_written))
    });
    out.truncate(12);
    Ok(out)
}
