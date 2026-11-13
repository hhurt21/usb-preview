#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod devices;
mod power;
mod sound;
mod store;
mod watch;

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use watch::{now_ms, Shared};

#[derive(Serialize)]
struct DevRow {
    #[serde(flatten)]
    dev: devices::Dev,
    /// アプリを開いてから外れた回数
    removed_total: usize,
    /// 直近 10 分で外れた回数
    removed_10min: usize,
    last_removed_seen: Option<i64>,
    last_arrived_seen: Option<i64>,
}

#[derive(Serialize)]
struct DevicesReply {
    devices: Vec<DevRow>,
    /// USB ではない親 (コントローラー)。つながり方の木の根っこ
    controllers: Vec<devices::Dev>,
    names: store::Names,
    time: i64,
}

#[tauri::command]
fn get_devices(shared: State<Shared>) -> DevicesReply {
    let all = devices::list(false);
    let t = now_ms();
    let c = shared.lock().unwrap();
    let ids: HashSet<String> = all.iter().map(|d| d.id.to_ascii_uppercase()).collect();
    let mut ctrl_ids: Vec<String> = all
        .iter()
        .filter(|d| d.present)
        .filter_map(|d| d.parent.clone())
        .filter(|p| !ids.contains(&p.to_ascii_uppercase()))
        .collect();
    ctrl_ids.sort();
    ctrl_ids.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    let controllers = ctrl_ids.iter().filter_map(|id| devices::get(id)).collect();

    let devices = all
        .into_iter()
        .map(|d| {
            let rem = c.removals.get(&d.id).cloned().unwrap_or_default();
            let arr = c.arrivals.get(&d.id).cloned().unwrap_or_default();
            DevRow {
                removed_total: rem.len(),
                removed_10min: rem.iter().filter(|&&x| t - x < 10 * 60 * 1000).count(),
                last_removed_seen: rem.last().copied(),
                last_arrived_seen: arr.last().copied(),
                dev: d,
            }
        })
        .collect();
    DevicesReply { devices, controllers, names: store::load_names(), time: t }
}

fn is_admin() -> bool {
    unsafe { windows::Win32::UI::Shell::IsUserAnAdmin().as_bool() }
}

#[tauri::command]
fn check_admin() -> bool {
    is_admin()
}

#[tauri::command]
fn relaunch_admin(app: tauri::AppHandle) -> Result<(), String> {
    use windows::core::{w, PCWSTR};
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let wexe: Vec<u16> = exe.as_os_str().to_string_lossy().encode_utf16().chain([0]).collect();
    let r = unsafe {
        windows::Win32::UI::Shell::ShellExecuteW(
            None,
            w!("runas"),
            PCWSTR(wexe.as_ptr()),
            None,
            None,
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };
    if (r.0 as isize) <= 32 {
        return Err("管理者で開けませんでした (確認画面で「いいえ」を押したかもしれません)".into());
    }
    app.exit(0);
    Ok(())
}

fn need_admin() -> Result<(), String> {
    if is_admin() {
        Ok(())
    } else {
        Err("管理者の権限が必要です。上の「管理者で開き直す」を押してください".into())
    }
}

#[tauri::command]
async fn set_enabled(id: String, enabled: bool) -> Result<(), String> {
    need_admin()?;
    tauri::async_runtime::spawn_blocking(move || devices::set_enabled(&id, enabled))
        .await
        .map_err(|e| e.to_string())?
}

/// 無効にしてすぐ有効に戻す (つなぎ直し)
#[tauri::command]
async fn restart_device(id: String) -> Result<(), String> {
    need_admin()?;
    tauri::async_runtime::spawn_blocking(move || {
        devices::set_enabled(&id, false)?;
        std::thread::sleep(std::time::Duration::from_millis(800));
        devices::set_enabled(&id, true)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn set_device_name(id: String, name: String) -> Result<store::Names, String> {
    store::set_device_name(&id, &name)
}

#[tauri::command]
fn set_port_name(key: String, name: String) -> Result<store::Names, String> {
    store::set_port_name(&key, &name)
}

#[tauri::command]
fn get_history(limit: usize) -> Vec<serde_json::Value> {
    store::read_history(limit)
}

#[tauri::command]
fn open_data_folder() -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(store::data_dir())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_sound() -> HashMap<String, bool> {
    sound::get()
}

#[tauri::command]
fn set_sound(on: bool) -> Result<HashMap<String, bool>, String> {
    sound::set(on)?;
    Ok(sound::get())
}

#[tauri::command]
async fn get_power_states() -> Result<HashMap<String, bool>, String> {
    tauri::async_runtime::spawn_blocking(power::get_devices).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_power(id: String, allow: bool) -> Result<(), String> {
    need_admin()?;
    tauri::async_runtime::spawn_blocking(move || power::set_device(&id, allow))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
struct GlobalSuspend {
    ac: bool,
    dc: bool,
}

#[tauri::command]
async fn get_global_suspend() -> Result<GlobalSuspend, String> {
    tauri::async_runtime::spawn_blocking(|| power::get_global().map(|(ac, dc)| GlobalSuspend { ac, dc }))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_global_suspend(on: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || power::set_global(on)).await.map_err(|e| e.to_string())?
}

fn main() {
    let shared: Shared = Arc::new(Mutex::new(watch::Counters::default()));
    tauri::Builder::default()
        .manage(shared.clone())
        .setup(move |app| {
            watch::start(app.handle().clone(), shared.clone());
            if let Some(w) = app.get_webview_window("main") {
                let title = if is_admin() { "USB 見張り（管理者）" } else { "USB 見張り" };
                let _ = w.set_title(title);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_devices,
            check_admin,
            relaunch_admin,
            set_enabled,
            restart_device,
            set_device_name,
            set_port_name,
            get_history,
            open_data_folder,
            get_sound,
            set_sound,
            get_power_states,
            set_power,
            get_global_suspend,
            set_global_suspend,
        ])
        .run(tauri::generate_context!())
        .expect("起動できませんでした");
}
