//! 全体図のための情報: PC (CPU・メモリ)、つながっている全部の機器、画面、ディスクとネットワークの詳細。

use crate::devices::{self, Dev};
use crate::display::{self, MonitorInfo};
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;

#[derive(Serialize)]
pub struct PcInfo {
    pub cpu: Option<String>,
    pub threads: usize,
    pub memory_gb: Option<f64>,
}

#[derive(Serialize)]
pub struct Overview {
    pub pc: PcInfo,
    /// つながっている全部の機器 (USB 以外も)
    pub devices: Vec<Dev>,
    pub monitors: Vec<MonitorInfo>,
}

fn cpu_name() -> Option<String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    let k = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0")
        .ok()?;
    let s: String = k.get_value("ProcessorNameString").ok()?;
    Some(s.trim().to_string())
}

fn memory_gb() -> Option<f64> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    unsafe { GlobalMemoryStatusEx(&mut m).ok()? };
    Some((m.ullTotalPhys as f64 / 1024f64.powi(3) * 10.0).round() / 10.0)
}

pub fn get() -> Overview {
    Overview {
        pc: PcInfo {
            cpu: cpu_name(),
            threads: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
            memory_gb: memory_gb(),
        },
        devices: devices::list_all_present(),
        monitors: display::list(),
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct DiskExtra {
    pub id: String,
    pub size: Option<i64>,
    pub iface: Option<String>,
    pub media: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct NetExtra {
    pub id: String,
    pub status: Option<String>,
    pub speed: Option<String>,
    pub kind: Option<String>,
    pub physical: Option<bool>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Extra {
    #[serde(default)]
    pub disks: Vec<DiskExtra>,
    #[serde(default)]
    pub nets: Vec<NetExtra>,
}

/// ディスクの容量・つなぎ方と、ネットワークのつながり具合・速さ (少し時間がかかるので別に取る)
pub fn extra() -> Result<Extra, String> {
    let script = r#"
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$d = @(Get-CimInstance Win32_DiskDrive | ForEach-Object {
  [pscustomobject]@{ id = [string]$_.PNPDeviceID; size = [int64]$_.Size; iface = [string]$_.InterfaceType; media = [string]$_.MediaType } })
$n = @(Get-NetAdapter | ForEach-Object {
  [pscustomobject]@{ id = [string]$_.PnPDeviceID; status = [string]$_.Status; speed = [string]$_.LinkSpeed; kind = [string]$_.MediaType; physical = [bool]$_.HardwareInterface } })
[pscustomobject]@{ disks = $d; nets = $n } | ConvertTo-Json -Depth 4 -Compress
"#;
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let s = s.trim().trim_start_matches('\u{feff}');
    serde_json::from_str(s).map_err(|e| format!("読めませんでした: {e}"))
}
