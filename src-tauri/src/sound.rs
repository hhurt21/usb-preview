//! Windows の「機器をつないだ・外した・失敗した」の音 (HKCU のサウンド設定)。

use std::collections::HashMap;
use winreg::enums::*;
use winreg::RegKey;

const EVENTS: [(&str, &str); 3] = [
    ("DeviceConnect", r"C:\Windows\media\Windows Hardware Insert.wav"),
    ("DeviceDisconnect", r"C:\Windows\media\Windows Hardware Remove.wav"),
    ("DeviceFail", r"C:\Windows\media\Windows Hardware Fail.wav"),
];

fn backup_path() -> std::path::PathBuf {
    crate::store::data_dir().join("sound_backup.json")
}

fn open(ev: &str, write: bool) -> Option<RegKey> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let p = format!(r"AppEvents\Schemes\Apps\.Default\{ev}\.Current");
    if write {
        hkcu.create_subkey(&p).ok().map(|(k, _)| k)
    } else {
        hkcu.open_subkey(&p).ok()
    }
}

/// 3 つの音それぞれ鳴るかどうか
pub fn get() -> HashMap<String, bool> {
    EVENTS
        .iter()
        .map(|(ev, _)| {
            let v: String = open(ev, false).and_then(|k| k.get_value("").ok()).unwrap_or_default();
            (ev.to_string(), !v.trim().is_empty())
        })
        .collect()
}

pub fn set(on: bool) -> Result<(), String> {
    if on {
        // 消す前に控えたもの → 前にこの PC で控えたもの → Windows の標準 の順で戻す
        let backup: HashMap<String, String> = std::fs::read_to_string(backup_path())
            .ok()
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .and_then(|p| std::fs::read_to_string(std::path::Path::new(&p).join("usb_sound_backup.json")).ok())
            })
            .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default();
        for (ev, def) in EVENTS {
            let v = backup.get(ev).filter(|s| !s.trim().is_empty()).cloned().unwrap_or_else(|| def.to_string());
            open(ev, true).ok_or("設定を開けません")?.set_value("", &v).map_err(|e| e.to_string())?;
        }
    } else {
        let mut cur: HashMap<String, String> = HashMap::new();
        for (ev, _) in EVENTS {
            let v: String = open(ev, false).and_then(|k| k.get_value("").ok()).unwrap_or_default();
            if !v.trim().is_empty() {
                cur.insert(ev.to_string(), v);
            }
        }
        if !cur.is_empty() {
            let _ = std::fs::write(backup_path(), serde_json::to_string_pretty(&cur).unwrap_or_default());
        }
        for (ev, _) in EVENTS {
            open(ev, true).ok_or("設定を開けません")?.set_value("", &"").map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
