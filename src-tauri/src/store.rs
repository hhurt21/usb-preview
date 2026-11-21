//! 自分でつけた名前と履歴の保存先: %APPDATA%\usb-watch

use crate::watch::Change;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Mutex;

static FILE_LOCK: Mutex<()> = Mutex::new(());

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let d = base.join("usb-watch");
    let _ = std::fs::create_dir_all(&d);
    d
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Names {
    /// 機器の ID → 自分でつけた名前
    #[serde(default)]
    pub devices: HashMap<String, String>,
    /// 「親の ID|ポート番号」→ 自分でつけた名前
    #[serde(default)]
    pub ports: HashMap<String, String>,
}

fn names_path() -> PathBuf {
    data_dir().join("names.json")
}

pub fn load_names() -> Names {
    std::fs::read_to_string(names_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_names(n: &Names) -> Result<(), String> {
    let s = serde_json::to_string_pretty(n).map_err(|e| e.to_string())?;
    let tmp = names_path().with_extension("json.tmp");
    std::fs::write(&tmp, s).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, names_path()).map_err(|e| e.to_string())
}

fn key(s: &str) -> String {
    s.to_ascii_uppercase()
}

pub fn set_device_name(id: &str, name: &str) -> Result<Names, String> {
    let _g = FILE_LOCK.lock().unwrap();
    let mut n = load_names();
    let name = name.trim();
    if name.is_empty() {
        n.devices.remove(&key(id));
    } else {
        n.devices.insert(key(id), name.to_string());
    }
    save_names(&n)?;
    Ok(n)
}

pub fn set_port_name(port_key: &str, name: &str) -> Result<Names, String> {
    let _g = FILE_LOCK.lock().unwrap();
    let mut n = load_names();
    let name = name.trim();
    if name.is_empty() {
        n.ports.remove(&key(port_key));
    } else {
        n.ports.insert(key(port_key), name.to_string());
    }
    save_names(&n)?;
    Ok(n)
}

fn history_path() -> PathBuf {
    data_dir().join("history.jsonl")
}

pub fn append_history(ch: &Change) {
    let _g = FILE_LOCK.lock().unwrap();
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(history_path()) {
        if let Ok(s) = serde_json::to_string(ch) {
            let _ = writeln!(f, "{s}");
        }
    }
}

/// 新しい順に最大 limit 件
pub fn read_history(limit: usize) -> Vec<serde_json::Value> {
    let _g = FILE_LOCK.lock().unwrap();
    let Ok(f) = std::fs::File::open(history_path()) else {
        return Vec::new();
    };
    let lines: Vec<String> = std::io::BufReader::new(f).lines().map_while(Result::ok).collect();
    lines
        .iter()
        .rev()
        .take(limit)
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// アプリの動作設定 (トレイ常駐・通知・RAM 自動解放など)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    /// 窓を閉じてもタスクトレイに残して裏で見張る
    #[serde(default)]
    pub minimize_to_tray: bool,
    /// 10 分で 3 回以上外れたときに Windows 通知を出す
    #[serde(default = "default_true")]
    pub notify_on_flap: bool,
    /// 外れたらすぐ通知する機器の ID (大文字)
    #[serde(default)]
    pub watched_ids: Vec<String>,
    /// 定期的に RAM を自動で解放する (Microsoft PC Manager のスマートブースト相当)
    #[serde(default)]
    pub auto_ram_boost: bool,
    /// RAM 使用率がこの % 以上になったら自動解放 (0 なら毎回)
    #[serde(default = "default_ram_threshold")]
    pub auto_ram_threshold_pct: u32,
    /// 自動解放をチェック・実行する間隔 (分)
    #[serde(default = "default_ram_interval")]
    pub auto_ram_interval_min: u32,
}

fn default_true() -> bool {
    true
}

fn default_ram_threshold() -> u32 {
    80
}

fn default_ram_interval() -> u32 {
    15
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            minimize_to_tray: false,
            notify_on_flap: true,
            watched_ids: Vec::new(),
            auto_ram_boost: false,
            auto_ram_threshold_pct: 80,
            auto_ram_interval_min: 15,
        }
    }
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn load_settings() -> AppSettings {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(s: &AppSettings) -> Result<AppSettings, String> {
    let _g = FILE_LOCK.lock().unwrap();
    let mut clean = s.clone();
    clean.watched_ids = clean
        .watched_ids
        .into_iter()
        .map(|id| id.to_ascii_uppercase())
        .collect();
    clean.watched_ids.sort();
    clean.watched_ids.dedup();
    let txt = serde_json::to_string_pretty(&clean).map_err(|e| e.to_string())?;
    let tmp = settings_path().with_extension("json.tmp");
    std::fs::write(&tmp, txt).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, settings_path()).map_err(|e| e.to_string())?;
    Ok(clean)
}

fn baseline_path() -> PathBuf {
    data_dir().join("baseline.json")
}

pub fn load_baseline() -> Option<serde_json::Value> {
    let _g = FILE_LOCK.lock().unwrap();
    std::fs::read_to_string(baseline_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

pub fn save_baseline(v: &serde_json::Value) -> Result<(), String> {
    let _g = FILE_LOCK.lock().unwrap();
    let txt = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    let tmp = baseline_path().with_extension("json.tmp");
    std::fs::write(&tmp, txt).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, baseline_path()).map_err(|e| e.to_string())
}
