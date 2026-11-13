//! 省電力で USB が切れる設定。
//! 機器ごと: デバイスマネージャーの「電力の節約のために、コンピューターでこのデバイスの電源をオフにできるようにする」
//!           (WMI の MSPower_DeviceEnable)。
//! 全体:     電源プランの「USB のセレクティブ サスペンドの設定」(powercfg)。

use std::collections::HashMap;
use std::os::windows::process::CommandExt;
use std::process::Command;

const NO_WINDOW: u32 = 0x0800_0000;
const SUB_USB: &str = "2a737441-1930-4402-8d77-b2bebba308a3";
const SET_SUSPEND: &str = "48e6b7a6-50f5-4782-a5d4-53bb8f07e226";

fn ps(script: &str) -> Result<String, String> {
    let out = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// 機器の ID (大文字) → 省電力で切ってよいか
pub fn get_devices() -> Result<HashMap<String, bool>, String> {
    let s = ps("Get-CimInstance -Namespace root\\wmi -ClassName MSPower_DeviceEnable | ForEach-Object { $_.InstanceName + '|' + $_.Enable }")?;
    let mut m = HashMap::new();
    for line in s.lines() {
        let Some((name, en)) = line.trim().rsplit_once('|') else { continue };
        // "USB\VID_xxxx\...._0" の最後の "_0" を外す
        let id = match name.rfind('_') {
            Some(i) if name[i + 1..].chars().all(|c| c.is_ascii_digit()) => &name[..i],
            _ => name,
        };
        m.insert(id.to_ascii_uppercase(), en.eq_ignore_ascii_case("true"));
    }
    Ok(m)
}

pub fn set_device(id: &str, allow: bool) -> Result<(), String> {
    if id.contains('\'') {
        return Err("ID に使えない文字があります".into());
    }
    let script = format!(
        "$n = 0; Get-CimInstance -Namespace root\\wmi -ClassName MSPower_DeviceEnable | Where-Object {{ $_.InstanceName -like '{id}_*' }} | ForEach-Object {{ $_.Enable = ${allow}; Set-CimInstance -InputObject $_ -ErrorAction Stop; $n++ }}; \"$n\"",
        id = id.replace('[', "`[").replace(']', "`]"),
        allow = if allow { "true" } else { "false" }
    );
    let out = ps(&script)?;
    if out.trim() == "0" {
        return Err("この機器には省電力の設定がありません".into());
    }
    Ok(())
}

fn powercfg(args: &[&str]) -> Result<String, String> {
    let out = Command::new("powercfg.exe")
        .args(args)
        .creation_flags(NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// (電源につないだとき, 電池のとき) のセレクティブサスペンドが有効か
pub fn get_global() -> Result<(bool, bool), String> {
    let s = powercfg(&["/q", "SCHEME_CURRENT", SUB_USB, SET_SUSPEND])?;
    // 言語によって文字が変わるので、最後の 2 つの 0x 値 (AC, DC の順) を読む
    let vals: Vec<u32> = s
        .lines()
        .filter_map(|l| l.rsplit_once("0x").and_then(|(_, h)| u32::from_str_radix(h.trim(), 16).ok()))
        .collect();
    if vals.len() < 2 {
        return Err("読めませんでした".into());
    }
    let n = vals.len();
    Ok((vals[n - 2] != 0, vals[n - 1] != 0))
}

pub fn set_global(on: bool) -> Result<(), String> {
    let v = if on { "1" } else { "0" };
    powercfg(&["/setacvalueindex", "SCHEME_CURRENT", SUB_USB, SET_SUSPEND, v])?;
    powercfg(&["/setdcvalueindex", "SCHEME_CURRENT", SUB_USB, SET_SUSPEND, v])?;
    powercfg(&["/setactive", "SCHEME_CURRENT"])?;
    Ok(())
}
