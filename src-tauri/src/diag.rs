//! PC の診断・お手入れ機能:
//! - Windows のイベントログから急な電源落ち・ブルースクリーン・画面ドライバー落ち・USB エラーを取得
//! - Windows 起動時に自動で立ち上がるアプリ (スタートアップ) の一覧と ON/OFF
//! - USB 見張り自身の自動起動 (HKCU\...\Run)
//! - Windows トースト通知

use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use winreg::enums::*;
use winreg::{RegKey, HKEY};

const NO_WINDOW: u32 = 0x0800_0000;
const RUN_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED_RUN_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SysEvent {
    /// UNIX ミリ秒
    pub time: i64,
    pub id: u32,
    pub provider: String,
    /// power / bsod / gpu / usb / disk
    pub kind: String,
    pub title: String,
    pub message: String,
}

/// 過去 30 日の重要なシステムイベント (急な電源落ち・BSOD・GPU リセット・USB/ディスク不調)
pub fn get_system_events() -> Result<Vec<SysEvent>, String> {
    let script = r#"
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$ErrorActionPreference = 'SilentlyContinue'
$since = (Get-Date).AddDays(-30)
$ev = @(Get-WinEvent -FilterHashtable @{
  LogName = 'System'
  StartTime = $since
  Id = @(41, 1001, 4101, 219, 129, 153, 51, 15, 20001)
} -MaxEvents 60)
$out = @()
foreach ($e in $ev) {
  $p = [string]$e.ProviderName
  $id = [int]$e.Id
  $kind = ''
  $title = ''
  if ($id -eq 41 -and $p -like '*Kernel-Power*') {
    $kind = 'power'; $title = '急な電源落ち・強制再起動 (Kernel-Power 41)'
  } elseif ($id -eq 1001 -and ($p -like '*BugCheck*' -or $p -like '*WER*')) {
    $kind = 'bsod'; $title = 'ブルースクリーン・強制終了 (BugCheck)'
  } elseif ($id -eq 4101 -and $p -eq 'Display') {
    $kind = 'gpu'; $title = '画面ドライバーが一時停止して回復 (Display 4101)'
  } elseif ($p -match 'USB|xhci|hub' -or $id -eq 219) {
    $kind = 'usb'; $title = "USB・機器のエラー (ID $id)"
  } elseif ($p -match 'disk|stornvme|storahci|ntfs' -and $id -in @(51, 129, 153, 15)) {
    $kind = 'disk'; $title = "ディスクの警告・応答遅れ (ID $id)"
  } else {
    continue
  }
  $msg = ([string]$e.Message -replace "\r?\n+", ' ').Trim()
  if ($msg.Length -gt 220) { $msg = $msg.Substring(0, 220) + '…' }
  $out += [pscustomobject]@{
    time = [int64]([DateTimeOffset]$e.TimeCreated).ToUnixTimeMilliseconds()
    id = $id
    provider = $p
    kind = $kind
    title = $title
    message = $msg
  }
}
$out | ConvertTo-Json -Depth 3 -Compress
"#;
    let out = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    let s = String::from_utf8_lossy(&out.stdout);
    let s = s.trim().trim_start_matches('\u{feff}');
    if s.is_empty() || s == "null" {
        return Ok(Vec::new());
    }
    if s.starts_with('{') {
        let one: SysEvent = serde_json::from_str(s).map_err(|e| e.to_string())?;
        return Ok(vec![one]);
    }
    serde_json::from_str(s).map_err(|e| format!("イベントログを読めませんでした: {e}"))
}

#[derive(Serialize, Clone, Debug)]
pub struct StartupApp {
    pub name: String,
    pub command: String,
    /// hkcu / hklm / folder
    pub scope: String,
    pub enabled: bool,
    pub can_toggle: bool,
}

fn is_approved(root: HKEY, name: &str) -> bool {
    let Ok(k) = RegKey::predef(root).open_subkey(APPROVED_RUN_PATH) else {
        return true;
    };
    let Ok(val) = k.get_raw_value(name) else {
        return true;
    };
    // 先頭バイトが偶数 (0x02 など) なら有効、奇数 (0x03 など) なら無効 (タスクマネージャーと同じ規則)
    val.bytes.first().map(|&b| b % 2 == 0).unwrap_or(true)
}

fn set_approved(root: HKEY, name: &str, enabled: bool) -> Result<(), String> {
    let (k, _) = RegKey::predef(root)
        .create_subkey(APPROVED_RUN_PATH)
        .map_err(|e| format!("設定を開けませんでした: {e}"))?;
    let mut bytes = [0u8; 12];
    bytes[0] = if enabled { 0x02 } else { 0x03 };
    let val = winreg::RegValue { bytes: bytes.to_vec(), vtype: REG_BINARY };
    k.set_raw_value(name, &val)
        .map_err(|e| format!("書き込めませんでした: {e}"))
}

pub fn get_startup_apps(is_admin: bool) -> Vec<StartupApp> {
    let mut out = Vec::new();
    for (root, scope, can_toggle) in [
        (HKEY_CURRENT_USER, "hkcu", true),
        (HKEY_LOCAL_MACHINE, "hklm", is_admin),
    ] {
        if let Ok(k) = RegKey::predef(root).open_subkey(RUN_PATH) {
            for item in k.enum_values().flatten() {
                let (name, val) = item;
                let cmd = val.to_string();
                out.push(StartupApp {
                    enabled: is_approved(root, &name),
                    name,
                    command: cmd,
                    scope: scope.into(),
                    can_toggle,
                });
            }
        }
    }
    // スタートアップ フォルダー (.lnk 等)
    if let Some(app) = std::env::var_os("APPDATA") {
        let dir = PathBuf::from(app).join(r"Microsoft\Windows\Start Menu\Programs\Startup");
        if let Ok(rd) = std::fs::read_dir(dir) {
            for ent in rd.flatten() {
                let p = ent.path();
                let nm = ent.file_name().to_string_lossy().to_string();
                if nm.eq_ignore_ascii_case("desktop.ini") {
                    continue;
                }
                out.push(StartupApp {
                    name: p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or(nm),
                    command: p.to_string_lossy().to_string(),
                    scope: "folder".into(),
                    enabled: true,
                    can_toggle: false,
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

pub fn set_startup_enabled(scope: &str, name: &str, enabled: bool, is_admin: bool) -> Result<(), String> {
    match scope {
        "hkcu" => set_approved(HKEY_CURRENT_USER, name, enabled),
        "hklm" => {
            if !is_admin {
                return Err("PC 全体の自動起動を変えるには、管理者の権限が必要です".into());
            }
            set_approved(HKEY_LOCAL_MACHINE, name, enabled)
        }
        _ => Err("スタートアップ フォルダーの項目はフォルダーから管理してください".into()),
    }
}

/// USB 見張り自身を Windows 起動時に自動起動するか (HKCU\...\Run\UsbWatch)
pub fn get_autostart() -> bool {
    let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(RUN_PATH) else {
        return false;
    };
    k.get_value::<String, _>("UsbWatch").is_ok() && is_approved(HKEY_CURRENT_USER, "UsbWatch")
}

pub fn set_autostart(on: bool) -> Result<(), String> {
    let (k, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(RUN_PATH)
        .map_err(|e| format!("設定を開けませんでした: {e}"))?;
    if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let val = format!("\"{}\" --minimized", exe.display());
        k.set_value("UsbWatch", &val).map_err(|e| e.to_string())?;
        let _ = set_approved(HKEY_CURRENT_USER, "UsbWatch", true);
    } else {
        let _ = k.delete_value("UsbWatch");
    }
    Ok(())
}

/// Windows 通知センターにトースト通知を出す (裏スレッドで非同期実行、失敗しても無視)
pub fn notify_toast(title: &str, body: &str) {
    let t = title.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\'', "''");
    let b = body.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\'', "''");
    std::thread::spawn(move || {
        let script = format!(
            r#"
$ErrorActionPreference = 'SilentlyContinue'
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml('<toast><visual><binding template="ToastText02"><text id="1">{t}</text><text id="2">{b}</text></binding></visual></toast>')
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Windows.SystemToast.SecurityAndMaintenance').Show($toast)
"#
        );
        let _ = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(NO_WINDOW)
            .output();
    });
}
