//! つないだ・外れたの見張り。
//! Windows の知らせ (CM_Register_Notification) を合図に一覧を取り直して前回と比べる。
//! 知らせが来ない機器のために 1.5 秒ごとにも比べる。
//! 外れて 1 秒もしないうちに戻った機器は、一覧を比べても前後とも「ある」に見えるので、
//! 知らせに入っている「外れた機器の ID」を使って「外れてすぐ戻った」として記録する。

use crate::devices::{self, Dev};
use crate::store;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use windows::Win32::Devices::DeviceAndDriverInstallation::*;

/// None = 何か変わった合図、Some(ID 大文字) = その機器が外れたという知らせ
static TX: OnceLock<Sender<Option<String>>> = OnceLock::new();

#[derive(Default)]
pub struct Counters {
    /// 外れた時刻 (ms) の記録。アプリを開いてから。
    pub removals: HashMap<String, Vec<i64>>,
    pub arrivals: HashMap<String, Vec<i64>>,
}

pub type Shared = Arc<Mutex<Counters>>;

#[derive(Serialize, Clone, Debug)]
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

#[derive(Serialize, Clone, Debug)]
pub struct Change {
    pub time: i64,
    /// arrived / removed / blink (外れてすぐ戻った) / status
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
    action: CM_NOTIFY_ACTION,
    data: *const CM_NOTIFY_EVENT_DATA,
    size: u32,
) -> u32 {
    let Some(tx) = TX.get() else { return 0 };
    let mut removed_id = None;
    if action == CM_NOTIFY_ACTION_DEVICEINSTANCEREMOVED && !data.is_null() {
        // InstanceId は構造体の後ろに続く可変長の文字列 (size の範囲で 0 終端まで読む)
        let base = data as *const u8;
        let start = std::ptr::addr_of!((*data).u.DeviceInstance.InstanceId) as *const u16;
        let offset = (start as *const u8).offset_from(base) as usize;
        let max = (size as usize).saturating_sub(offset) / 2;
        let mut len = 0;
        while len < max && *start.add(len) != 0 {
            len += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(start, len));
        #[cfg(debug_assertions)]
        eprintln!("notify removed: {s}");
        if s.to_ascii_uppercase().starts_with("USB\\") {
            removed_id = Some(s.to_ascii_uppercase());
        }
    }
    let _ = tx.send(removed_id);
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
        tops.push(ChangeItem {
            id: d.id.clone(),
            name: d.name.clone(),
            bus_name: d.bus_name.clone(),
            parent: d.parent.clone(),
            port: d.port,
            status: d.status.clone(),
            parts: count_desc(id, ids, src),
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

/// 前回と今回の一覧、その間に届いた「外れた」知らせから、変化を作る
pub struct Diff {
    pub removed: HashSet<String>,
    pub arrived: HashSet<String>,
    pub blinked: HashSet<String>,
    pub changes: Vec<Change>,
}

pub fn diff(prev: &HashMap<String, Dev>, cur: &HashMap<String, Dev>, notified_removed: &HashSet<String>, t: i64) -> Diff {
    let removed: HashSet<String> = prev.keys().filter(|k| !cur.contains_key(*k)).cloned().collect();
    let arrived: HashSet<String> = cur.keys().filter(|k| !prev.contains_key(*k)).cloned().collect();
    let blinked: HashSet<String> = cur
        .keys()
        .filter(|k| prev.contains_key(*k) && notified_removed.contains(&k.to_ascii_uppercase()))
        .cloned()
        .collect();
    let changed: HashSet<String> = cur
        .iter()
        .filter(|(k, d)| !blinked.contains(*k) && prev.get(*k).map(|p| p.status != d.status).unwrap_or(false))
        .map(|(k, _)| k.clone())
        .collect();

    let mut changes = Vec::new();
    if !removed.is_empty() {
        changes.push(Change { time: t, kind: "removed".into(), items: top_level(&removed, prev) });
    }
    if !blinked.is_empty() {
        changes.push(Change { time: t, kind: "blink".into(), items: top_level(&blinked, cur) });
    }
    if !arrived.is_empty() {
        changes.push(Change { time: t, kind: "arrived".into(), items: top_level(&arrived, cur) });
    }
    if !changed.is_empty() {
        changes.push(Change { time: t, kind: "status".into(), items: top_level(&changed, cur) });
    }
    Diff { removed, arrived, blinked, changes }
}

fn snapshot() -> HashMap<String, Dev> {
    devices::list(true).into_iter().map(|d| (d.id.clone(), d)).collect()
}

pub fn start(app: AppHandle, shared: Shared) {
    let (tx, rx) = channel::<Option<String>>();
    let _ = TX.set(tx);
    register();
    std::thread::spawn(move || {
        let mut prev = snapshot();
        loop {
            let mut notified: HashSet<String> = HashSet::new();
            match rx.recv_timeout(Duration::from_millis(1500)) {
                Ok(first) => {
                    notified.extend(first);
                    // 1 台の抜き差しで知らせが何十個も来るので少し待ってまとめる
                    std::thread::sleep(Duration::from_millis(150));
                    while let Ok(x) = rx.try_recv() {
                        notified.extend(x);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            let cur = snapshot();
            let t = now_ms();
            let d = diff(&prev, &cur, &notified, t);
            if !d.changes.is_empty() {
                {
                    let mut c = shared.lock().unwrap();
                    for id in d.removed.iter().chain(&d.blinked) {
                        c.removals.entry(id.clone()).or_default().push(t);
                    }
                    for id in d.arrived.iter().chain(&d.blinked) {
                        c.arrivals.entry(id.clone()).or_default().push(t);
                    }
                }
                for ch in &d.changes {
                    store::append_history(ch);
                    let _ = app.emit("usb-change", ch);
                }
            }
            prev = cur;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(id: &str, parent: &str, status: &str) -> Dev {
        Dev {
            id: id.into(),
            name: id.into(),
            bus_name: None,
            manufacturer: None,
            class: None,
            parent: Some(parent.into()),
            location: None,
            port: Some(1),
            present: true,
            status: status.into(),
            problem: 0,
            last_arrival: None,
            last_removal: None,
            vid: None,
            pid: None,
            is_interface: false,
            is_hub: false,
        }
    }

    fn map(ds: Vec<Dev>) -> HashMap<String, Dev> {
        ds.into_iter().map(|d| (d.id.clone(), d)).collect()
    }

    #[test]
    fn composite_removal_reports_only_the_plugged_device() {
        let prev = map(vec![
            dev("USB\\HUB", "PCI\\X", "ok"),
            dev("USB\\KB", "USB\\HUB", "ok"),
            dev("USB\\KB&MI_00", "USB\\KB", "ok"),
            dev("USB\\KB&MI_01", "USB\\KB", "ok"),
        ]);
        let cur = map(vec![dev("USB\\HUB", "PCI\\X", "ok")]);
        let d = diff(&prev, &cur, &HashSet::new(), 0);
        assert_eq!(d.changes.len(), 1);
        assert_eq!(d.changes[0].kind, "removed");
        assert_eq!(d.changes[0].items.len(), 1);
        assert_eq!(d.changes[0].items[0].id, "USB\\KB");
        assert_eq!(d.changes[0].items[0].parts, 2);
        assert_eq!(d.removed.len(), 3);
    }

    #[test]
    fn quick_blink_is_caught_from_notification() {
        let prev = map(vec![dev("USB\\VID_0000&PID_0002\\6&1", "USB\\ROOT", "disabled")]);
        let cur = prev.clone();
        // 知らせが無ければ変化なし
        assert!(diff(&prev, &cur, &HashSet::new(), 0).changes.is_empty());
        // 「外れた」知らせがあれば、外れてすぐ戻ったとして数える
        let n: HashSet<String> = ["USB\\VID_0000&PID_0002\\6&1".to_string()].into();
        let d = diff(&prev, &cur, &n, 0);
        assert_eq!(d.changes.len(), 1);
        assert_eq!(d.changes[0].kind, "blink");
        assert_eq!(d.blinked.len(), 1);
    }

    #[test]
    fn notification_for_unknown_device_is_ignored() {
        let prev = map(vec![dev("USB\\A", "USB\\ROOT", "ok")]);
        let cur = prev.clone();
        let n: HashSet<String> = ["USB\\OTHER".to_string()].into();
        assert!(diff(&prev, &cur, &n, 0).changes.is_empty());
    }

    #[test]
    fn status_change_is_reported() {
        let prev = map(vec![dev("USB\\A", "USB\\ROOT", "ok")]);
        let cur = map(vec![dev("USB\\A", "USB\\ROOT", "disabled")]);
        let d = diff(&prev, &cur, &HashSet::new(), 0);
        assert_eq!(d.changes.len(), 1);
        assert_eq!(d.changes[0].kind, "status");
    }
}
