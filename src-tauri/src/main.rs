#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod browser;
mod devices;
mod diag;
mod diskrate;
mod display;
mod overview;
mod power;
mod ram;
mod smart;
mod sound;
mod spec;
mod store;
mod usbinfo;
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

#[tauri::command]
fn get_disk_rates() -> Result<Vec<diskrate::DiskRate>, String> {
    diskrate::rates()
}

#[tauri::command]
async fn get_cache_policy() -> Result<Vec<browser::BrowserCache>, String> {
    tauri::async_runtime::spawn_blocking(browser::list).await.map_err(|e| e.to_string())
}

/// ブラウザーを閉じて、タブごと開き直す (管理者で動いているときは閉じるだけ)
#[tauri::command]
async fn restart_browser(id: String) -> Result<String, String> {
    let admin = is_admin();
    tauri::async_runtime::spawn_blocking(move || browser::restart(&id, admin)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_cache_policy(id: String, small: bool) -> Result<Vec<browser::BrowserCache>, String> {
    tauri::async_runtime::spawn_blocking(move || browser::set(&id, small)).await.map_err(|e| e.to_string())?
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

#[tauri::command]
async fn get_usb_ports() -> Result<HashMap<String, usbinfo::UsbPortInfo>, String> {
    tauri::async_runtime::spawn_blocking(usbinfo::query_all).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_proc_io_rates() -> Result<Vec<diskrate::ProcIoRate>, String> {
    tauri::async_runtime::spawn_blocking(diskrate::proc_io_rates)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn reset_proc_io_totals() {
    diskrate::reset_proc_totals();
}

#[tauri::command]
async fn set_monitor_brightness(gdi_name: String, value: u32) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || display::set_brightness(&gdi_name, value))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_all_monitor_brightness(pct: u32) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || display::set_all_brightness(pct))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_wake_info() -> Result<power::WakeInfo, String> {
    tauri::async_runtime::spawn_blocking(power::get_wake_info)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn disable_device_wake(device_name: String) -> Result<(), String> {
    need_admin()?;
    tauri::async_runtime::spawn_blocking(move || power::disable_device_wake(&device_name))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_audio_mute(id: String, muted: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || audio::set_endpoint_mute(&id, muted))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_audio_volume(id: String, volume: f32) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || audio::set_endpoint_volume(&id, volume))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_audio_session_volume(endpoint_id: String, pid: u32, volume: f32, muted: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || audio::set_session_volume(&endpoint_id, pid, volume, muted))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_default_audio(id: String, role: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || audio::set_default_endpoint(&id, &role))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn get_app_settings() -> store::AppSettings {
    store::load_settings()
}

#[tauri::command]
fn save_app_settings(settings: store::AppSettings) -> Result<store::AppSettings, String> {
    store::save_settings(&settings)
}

#[tauri::command]
fn get_autostart() -> bool {
    diag::get_autostart()
}

#[tauri::command]
fn set_autostart(on: bool) -> Result<bool, String> {
    diag::set_autostart(on)?;
    Ok(diag::get_autostart())
}

#[tauri::command]
fn test_notification() {
    diag::notify_toast("USB 見張り", "通知のテストです。異常があればこのようにお知らせします。");
}

#[tauri::command]
fn get_baseline() -> Option<serde_json::Value> {
    store::load_baseline()
}

#[tauri::command]
fn save_baseline(snapshot: serde_json::Value) -> Result<(), String> {
    store::save_baseline(&snapshot)
}

#[tauri::command]
async fn get_system_events() -> Result<Vec<diag::SysEvent>, String> {
    tauri::async_runtime::spawn_blocking(diag::get_system_events)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_startup_apps() -> Result<Vec<diag::StartupApp>, String> {
    let admin = is_admin();
    tauri::async_runtime::spawn_blocking(move || diag::get_startup_apps(admin))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_startup_enabled(scope: String, name: String, enabled: bool) -> Result<(), String> {
    let admin = is_admin();
    tauri::async_runtime::spawn_blocking(move || diag::set_startup_enabled(&scope, &name, enabled, admin))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn eject_device(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || devices::eject_device(&id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remove_ghost_device(id: String) -> Result<(), String> {
    need_admin()?;
    tauri::async_runtime::spawn_blocking(move || devices::remove_ghost_device(&id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_ram_status(with_temp: bool) -> Result<ram::RamStatus, String> {
    tauri::async_runtime::spawn_blocking(move || ram::status(with_temp))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn boost_ram(clean_temp: bool) -> Result<ram::BoostResult, String> {
    tauri::async_runtime::spawn_blocking(move || ram::boost(clean_temp, false))
        .await
        .map_err(|e| e.to_string())
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
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW};
    unsafe {
        for title in [w!("USB 見張り"), w!("USB 見張り（管理者）")] {
            if let Ok(h) = FindWindowW(None, title) {
                if !h.is_invalid() {
                    let _ = ShowWindow(h, SW_SHOW);
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
            ram::start_auto_worker(app.handle().clone());
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
            let start_minimized = std::env::args().any(|a| a == "--minimized");
            let built = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                .title(title)
                .inner_size(1180.0, 800.0)
                .min_inner_size(760.0, 520.0)
                .visible(!start_minimized)
                .additional_browser_args(&args)
                .data_directory(data_dir)
                .build();
            if let Err(e) = built {
                show_error(&format!(
                    "画面を開けませんでした。\n\nUSB 見張りがもう開いていないか確かめて、閉じてからもう一度開いてください。\n\n詳しい理由: {e}"
                ));
                std::process::exit(1);
            }
            // タスクトレイのアイコン (閉じてトレイにしまう設定のときや、いつでも呼び戻せるように)
            if let (Ok(open_i), Ok(boost_i), Ok(quit_i)) = (
                tauri::menu::MenuItem::with_id(app, "open", "画面を開く", true, None::<&str>),
                tauri::menu::MenuItem::with_id(app, "boost", "今すぐ RAM を解放（ブースト）", true, None::<&str>),
                tauri::menu::MenuItem::with_id(app, "quit", "終了する", true, None::<&str>),
            ) {
                if let Ok(menu) = tauri::menu::Menu::with_items(app, &[&open_i, &boost_i, &quit_i]) {
                    let mut tb = tauri::tray::TrayIconBuilder::with_id("main-tray")
                        .tooltip(title)
                        .menu(&menu)
                        .on_menu_event(|app, ev| match ev.id.as_ref() {
                            "open" => {
                                if let Some(w) = app.get_webview_window("main") {
                                    let _ = w.show();
                                    let _ = w.unminimize();
                                    let _ = w.set_focus();
                                }
                            }
                            "boost" => {
                                std::thread::spawn(|| {
                                    let r = ram::boost(false, false);
                                    diag::notify_toast(
                                        "USB 見張り：RAM を解放しました",
                                        &format!("{} MB 解放（使用率 {}% → {}%）", r.freed_mb, r.before_pct, r.after_pct),
                                    );
                                });
                            }
                            "quit" => app.exit(0),
                            _ => {}
                        })
                        .on_tray_icon_event(|tray, ev| {
                            if let tauri::tray::TrayIconEvent::Click {
                                button: tauri::tray::MouseButton::Left,
                                button_state: tauri::tray::MouseButtonState::Up,
                                ..
                            } = ev
                            {
                                if let Some(w) = tray.app_handle().get_webview_window("main") {
                                    let _ = w.show();
                                    let _ = w.unminimize();
                                    let _ = w.set_focus();
                                }
                            }
                        });
                    if let Some(ic) = app.default_window_icon().cloned() {
                        tb = tb.icon(ic);
                    }
                    let _ = tb.build(app);
                }
            }
            Ok(())
        })
        .on_window_event(|w, ev| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = ev {
                if store::load_settings().minimize_to_tray {
                    api.prevent_close();
                    let _ = w.hide();
                }
            }
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
            get_disk_rates,
            get_cache_policy,
            set_cache_policy,
            restart_browser,
            get_usb_ports,
            get_proc_io_rates,
            reset_proc_io_totals,
            set_monitor_brightness,
            set_all_monitor_brightness,
            get_wake_info,
            disable_device_wake,
            set_audio_mute,
            set_audio_volume,
            set_audio_session_volume,
            set_default_audio,
            get_app_settings,
            save_app_settings,
            get_autostart,
            set_autostart,
            test_notification,
            get_baseline,
            save_baseline,
            get_system_events,
            get_startup_apps,
            set_startup_enabled,
            eject_device,
            remove_ghost_device,
            get_ram_status,
            boost_ram,
        ])
        .run(tauri::generate_context!())
        .expect("起動できませんでした");
}
