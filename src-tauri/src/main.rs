#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod devices;
mod display;
mod overview;
mod power;
mod smart;
mod sound;
mod spec;
mod store;
mod watch;

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tauri::State;
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
            w!("--relaunched"),
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

fn show_error(msg: &str) {
    use windows::core::{w, PCWSTR};
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let wmsg: Vec<u16> = msg.encode_utf16().chain([0]).collect();
    unsafe {
        MessageBoxW(None, PCWSTR(wmsg.as_ptr()), w!("USB 見張り"), MB_OK | MB_ICONERROR);
    }
}

#[tauri::command]
async fn get_audio_state() -> Result<audio::AudioState, String> {
    tauri::async_runtime::spawn_blocking(audio::state).await.map_err(|e| e.to_string())?
}

/// NVMe SSD の健康記録 (管理者のときだけ。普通の権限なら空)
#[tauri::command]
async fn get_nvme_health() -> Result<Vec<smart::NvmeHealth>, String> {
    if !is_admin() {
        return Ok(Vec::new());
    }
    tauri::async_runtime::spawn_blocking(smart::read_all).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_monitor_usage() -> Result<Vec<display::MonitorUsage>, String> {
    tauri::async_runtime::spawn_blocking(display::usage).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_spec() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(spec::get).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_overview() -> Result<overview::Overview, String> {
    tauri::async_runtime::spawn_blocking(overview::get).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_overview_extra() -> Result<overview::Extra, String> {
    tauri::async_runtime::spawn_blocking(overview::extra).await.map_err(|e| e.to_string())?
}

/// 1 つだけ開くための目印 (名前付きミューテックス)。取れたら true。
/// 取れたハンドルはアプリが終わるまで持ち続ける。
fn take_single_instance() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    unsafe {
        match CreateMutexW(None, false, w!("Local\\usb-watch-single-instance")) {
            Ok(h) => {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    // 自分のハンドルは閉じる (持ったままだと、先の窓が閉じても目印が残る)
                    let _ = CloseHandle(h);
                    false
                } else {
                    true
                }
            }
            // 管理者の窓が作った目印は、普通の権限からは開けない (= もう開いている)
            Err(_) => false,
        }
    }
}

/// すでに開いている窓を前に出す。見つからなければ false。
fn focus_existing() -> bool {
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE};
    unsafe {
        for title in [w!("USB 見張り"), w!("USB 見張り（管理者）")] {
            if let Ok(h) = FindWindowW(None, title) {
                if !h.is_invalid() {
                    let _ = ShowWindow(h, SW_RESTORE);
                    let _ = SetForegroundWindow(h);
                    return true;
                }
            }
        }
    }
    false
}

fn main() {
    // 2 つ同時に動くと、両方が履歴に書き込んで記録が二重になる
    let relaunched = std::env::args().any(|a| a == "--relaunched");
    let mut got = take_single_instance();
    if !got && relaunched {
        // 「管理者で開き直す」のときは、前の窓が閉じるのを最大 10 秒待つ
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(200));
            got = take_single_instance();
            if got {
                break;
            }
        }
    }
    if !got {
        if !focus_existing() {
            show_error("USB 見張りはもう開いています。");
        }
        return;
    }

    let shared: Shared = Arc::new(Mutex::new(watch::Counters::default()));
    tauri::Builder::default()
        .manage(shared.clone())
        .setup(move |app| {
            watch::start(app.handle().clone(), shared.clone());
            let title = if is_admin() { "USB 見張り（管理者）" } else { "USB 見張り" };
            // 窓はここで作る (試験用のビルドだけ、外から画面を操作する口 9333 番を開くため)
            let mut args = String::from("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection");
            if cfg!(debug_assertions) {
                args.push_str(" --remote-debugging-port=9333");
            }
            // 画面の部品 (WebView2) の作業フォルダーは、普通・管理者・試験用で分ける。
            // 同じフォルダーを権限や設定の違う窓が同時に使うと、後から開いた方が開けない
            // (「管理者で開き直す」で前の窓の部品がまだ残っているときなど)。
            let mut folder = String::from("webview");
            if is_admin() {
                folder.push_str("-admin");
            }
            if cfg!(debug_assertions) {
                folder.push_str("-debug");
            }
            let data_dir = std::env::var_os("LOCALAPPDATA")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join("usb-watch")
                .join(folder);
            let built = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                .title(title)
                .inner_size(1180.0, 800.0)
                .min_inner_size(760.0, 520.0)
                .additional_browser_args(&args)
                .data_directory(data_dir)
                .build();
            if let Err(e) = built {
                show_error(&format!(
                    "画面を開けませんでした。\n\nUSB 見張りがもう開いていないか確かめて、閉じてからもう一度開いてください。\n\n詳しい理由: {e}"
                ));
                std::process::exit(1);
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
            get_overview,
            get_overview_extra,
            get_audio_state,
            get_spec,
            get_monitor_usage,
            get_nvme_health,
        ])
        .run(tauri::generate_context!())
        .expect("起動できませんでした");
}
