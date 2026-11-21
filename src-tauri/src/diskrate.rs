//! 今のディスクへの書き込みの速さ (Windows のパフォーマンス カウンター。タスク マネージャーと同じ値)。
//! 速さは「前に読んだときからの差」で出るので、1 回目は 0 になる。画面から 2 秒ごとに呼ぶ。

use serde::Serialize;
use std::sync::Mutex;
use windows::core::w;
use windows::Win32::System::Performance::*;

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
