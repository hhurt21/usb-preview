//! つないだ・外れたの見張り。
//! Windows の知らせ (CM_Register_Notification) を合図に一覧を取り直して前回と比べる。
//! 知らせが来ない機器のために 1.5 秒ごとにも比べる。

use crate::devices::{self, Dev};
use crate::store;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use windows::Win32::Devices::DeviceAndDriverInstallation::*;

static TX: OnceLock<Sender<()>> = OnceLock::new();

#[derive(Default)]
pub struct Counters {
    /// 外れた時刻 (ms) の記録。アプリを開いてから。
    pub removals: HashMap<String, Vec<i64>>,
    pub arrivals: HashMap<String, Vec<i64>>,
}

pub type Shared = Arc<Mutex<Counters>>;

#[derive(Serialize, Clone)]
pub struct ChangeItem {
    pub id: String,
    pub name: String,
    pub bus_name: Option<String>,
    pub parent: Option<String>,
    pub port: Option<u32>,
    pub status: String,
    /// 一緒に増減した部品の数
    pub parts: usize,
}

#[derive(Serialize, Clone)]
pub struct Change {
    pub time: i64,
    /// arrived / removed / status
    pub kind: String,
    pub items: Vec<ChangeItem>,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

unsafe extern "system" fn on_notify(
    _h: HCMNOTIFICATION,
    _ctx: *const core::ffi::c_void,
    _action: CM_NOTIFY_ACTION,
    _data: *const CM_NOTIFY_EVENT_DATA,
    _size: u32,
) -> u32 {
    if let Some(tx) = TX.get() {
        let _ = tx.send(());
    }
    0
}

fn register() {
    unsafe {
        let mut filter: CM_NOTIFY_FILTER = std::mem::zeroed();
        filter.cbSize = std::mem::size_of::<CM_NOTIFY_FILTER>() as u32;
        filter.Flags = CM_NOTIFY_FILTER_FLAG_ALL_DEVICE_INSTANCES;
        filter.FilterType = CM_NOTIFY_FILTER_TYPE_DEVICEINSTANCE;
        let mut handle = HCMNOTIFICATION::default();
        let r = CM_Register_Notification(&filter, None, Some(on_notify), &mut handle);
        if r != CR_SUCCESS {
            eprintln!("CM_Register_Notification failed: {:?}", r);
        }
        // handle は解除しない (アプリが終わるまで見張り続ける)
    }
}

/// 一緒に消えた/増えた中で、親が同じ組に入っていないもの (= 実際に抜き差しした機器) だけ残す
fn top_level(ids: &HashSet<String>, src: &HashMap<String, Dev>) -> Vec<ChangeItem> {
    let mut tops: Vec<ChangeItem> = Vec::new();
    for id in ids {
        let d = &src[id];
        let parent_in = d
            .parent
            .as_ref()
            .map(|p| ids.iter().any(|x| x.eq_ignore_ascii_case(p)))
            .unwrap_or(false);
        if parent_in {
            continue;
        }
        let parts = count_desc(id, ids, src);
        tops.push(ChangeItem {
            id: d.id.clone(),
            name: d.name.clone(),
            bus_name: d.bus_name.clone(),
            parent: d.parent.clone(),
            port: d.port,
            status: d.status.clone(),
            parts,
        });
    }
    tops.sort_by(|a, b| a.id.cmp(&b.id));
    tops
}

fn count_desc(root: &str, ids: &HashSet<String>, src: &HashMap<String, Dev>) -> usize {
    ids.iter()
        .filter(|x| {
            let mut cur = src[*x].parent.clone();
            let mut depth = 0;
            while let Some(p) = cur {
                if p.eq_ignore_ascii_case(root) {
                    return true;
                }
                depth += 1;
                if depth > 16 {
                    break;
                }
                cur = ids
                    .iter()
                    .find(|y| y.eq_ignore_ascii_case(&p))
                    .and_then(|y| src[y].parent.clone());
            }
            false
        })
        .count()
}

fn snapshot() -> HashMap<String, Dev> {
    devices::list(true).into_iter().map(|d| (d.id.clone(), d)).collect()
}

pub fn start(app: AppHandle, shared: Shared) {
    let (tx, rx) = channel::<()>();
    let _ = TX.set(tx);
    register();
    std::thread::spawn(move || {
        let mut prev = snapshot();
        loop {
            match rx.recv_timeout(Duration::from_millis(1500)) {
                Ok(()) => {
                    // 1 台の抜き差しで知らせが何十個も来るので少し待ってまとめる
                    std::thread::sleep(Duration::from_millis(150));
                    while rx.try_recv().is_ok() {}
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            let cur = snapshot();
            let t = now_ms();

            let removed: HashSet<String> = prev.keys().filter(|k| !cur.contains_key(*k)).cloned().collect();
            let arrived: HashSet<String> = cur.keys().filter(|k| !prev.contains_key(*k)).cloned().collect();
            let changed: HashSet<String> = cur
                .iter()
                .filter(|(k, d)| prev.get(*k).map(|p| p.status != d.status).unwrap_or(false))
                .map(|(k, _)| k.clone())
                .collect();

            if !removed.is_empty() || !arrived.is_empty() || !changed.is_empty() {
                {
                    let mut c = shared.lock().unwrap();
                    for id in &removed {
                        c.removals.entry(id.clone()).or_default().push(t);
                    }
                    for id in &arrived {
                        c.arrivals.entry(id.clone()).or_default().push(t);
                    }
                }
                let mut changes = Vec::new();
                if !removed.is_empty() {
                    changes.push(Change { time: t, kind: "removed".into(), items: top_level(&removed, &prev) });
                }
                if !arrived.is_empty() {
                    changes.push(Change { time: t, kind: "arrived".into(), items: top_level(&arrived, &cur) });
                }
                if !changed.is_empty() {
                    changes.push(Change { time: t, kind: "status".into(), items: top_level(&changed, &cur) });
                }
                for ch in &changes {
                    store::append_history(ch);
                    let _ = app.emit("usb-change", ch);
                }
            }
            prev = cur;
        }
    });
}
