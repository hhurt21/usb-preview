//! 今使っている音の機器 + 音量・ミュート・既定の機器の切り替え。
//! - 既定の機器 (いつも使う / 通話で使う) を、鳴らす側と録る側それぞれ
//! - 機器ごとの今の音の大きさ (0〜1)・音量・ミュート
//! - その機器で鳴らしている (録っている) アプリごとの音量・ミュート

use serde::Serialize;
use windows::core::{GUID, HRESULT, Interface, PCWSTR, PWSTR};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioMeterInformation};
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

#[derive(Serialize, Clone, Debug, Default)]
pub struct AudioSession {
    pub pid: u32,
    pub name: String,
    pub volume: f32,
    pub muted: bool,
}

#[derive(Serialize, Default)]
pub struct EndpointState {
    /// 機器の ID (SWD\MMDEVAPI\{...}.{...}、大文字)
    pub id: String,
    /// 今の音の大きさ 0〜1
    pub peak: f32,
    /// 機器の音量 0〜1
    pub volume: f32,
    /// 機器のミュート状態
    pub muted: bool,
    /// 鳴らしている / 録っているアプリ (互換用)
    pub apps: Vec<String>,
    /// アプリごとの音量・ミュート
    pub sessions: Vec<AudioSession>,
}

#[derive(Serialize, Default)]
pub struct AudioState {
    pub render: Option<String>,
    pub render_comm: Option<String>,
    pub capture: Option<String>,
    pub capture_comm: Option<String>,
    pub endpoints: Vec<EndpointState>,
}

/// "{0.0.0.00000000}.{guid}" → "SWD\MMDEVAPI\{0.0.0.00000000}.{GUID}"
fn to_instance_id(endpoint_id: &str) -> String {
    format!("SWD\\MMDEVAPI\\{}", endpoint_id).to_ascii_uppercase()
}

unsafe fn raw_endpoint_id(d: &IMMDevice) -> Option<String> {
    let p: PWSTR = d.GetId().ok()?;
    let s = p.to_string().ok();
    CoTaskMemFree(Some(p.0 as *const _));
    s
}

unsafe fn device_id(d: &IMMDevice) -> Option<String> {
    raw_endpoint_id(d).map(|s| to_instance_id(&s))
}

unsafe fn default_id(en: &IMMDeviceEnumerator, flow: EDataFlow, role: ERole) -> Option<String> {
    let d = en.GetDefaultAudioEndpoint(flow, role).ok()?;
    device_id(&d)
}

unsafe fn process_name(pid: u32) -> String {
    if pid == 0 {
        return "Windows の音".into();
    }
    let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
        return snapshot_name(pid).unwrap_or_else(|| format!("アプリ ({pid})"));
    };
    let mut buf = [0u16; 512];
    let mut len = buf.len() as u32;
    let r = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
    let _ = CloseHandle(h);
    if r.is_err() {
        return snapshot_name(pid).unwrap_or_else(|| format!("アプリ ({pid})"));
    }
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    std::path::Path::new(&path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or(path)
}

/// 権限が足りずに開けないアプリ (管理者で動いている物など) は、プロセスの一覧から名前だけ探す
unsafe fn snapshot_name(pid: u32) -> Option<String> {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
    let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
    let mut found = None;
    if Process32FirstW(snap, &mut e).is_ok() {
        loop {
            if e.th32ProcessID == pid {
                let end = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                let name = String::from_utf16_lossy(&e.szExeFile[..end]);
                found = Some(name.trim_end_matches(".exe").trim_end_matches(".EXE").to_string());
                break;
            }
            if Process32NextW(snap, &mut e).is_err() {
                break;
            }
        }
    }
    let _ = CloseHandle(snap);
    found
}

unsafe fn active_sessions(d: &IMMDevice) -> (Vec<String>, Vec<AudioSession>) {
    let mut apps: Vec<String> = Vec::new();
    let mut sessions: Vec<AudioSession> = Vec::new();
    let Ok(sm) = d.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else {
        return (apps, sessions);
    };
    let Ok(se) = sm.GetSessionEnumerator() else {
        return (apps, sessions);
    };
    let n = se.GetCount().unwrap_or(0);
    for i in 0..n {
        let Ok(ctl) = se.GetSession(i) else { continue };
        if ctl.GetState().ok() != Some(AudioSessionStateActive) {
            continue;
        }
        let Ok(c2) = ctl.cast::<IAudioSessionControl2>() else { continue };
        let pid = c2.GetProcessId().unwrap_or(0);
        let name = process_name(pid);
        if !apps.contains(&name) {
            apps.push(name.clone());
        }
        if !sessions.iter().any(|s| s.pid == pid) {
            let (volume, muted) = ctl
                .cast::<ISimpleAudioVolume>()
                .map(|sv| {
                    let v = sv.GetMasterVolume().unwrap_or(1.0);
                    let m = sv.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                    (v, m)
                })
                .unwrap_or((1.0, false));
            sessions.push(AudioSession { pid, name, volume, muted });
        }
    }
    (apps, sessions)
}

unsafe fn with_com<T, F: FnOnce() -> Result<T, String>>(f: F) -> Result<T, String> {
    let inited = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
    let res = f();
    if inited {
        CoUninitialize();
    }
    res
}

unsafe fn find_endpoint(en: &IMMDeviceEnumerator, target_id: &str) -> Result<(IMMDevice, String), String> {
    let target_up = target_id.to_ascii_uppercase();
    let col = en.EnumAudioEndpoints(eAll, DEVICE_STATE_ACTIVE).map_err(|e| e.to_string())?;
    let n = col.GetCount().map_err(|e| e.to_string())?;
    for i in 0..n {
        let Ok(d) = col.Item(i) else { continue };
        let Some(raw) = raw_endpoint_id(&d) else { continue };
        if to_instance_id(&raw) == target_up || raw.eq_ignore_ascii_case(target_id) {
            return Ok((d, raw));
        }
    }
    Err("音の機器が見つかりませんでした".into())
}

pub fn state() -> Result<AudioState, String> {
    unsafe {
        with_com(|| {
            let en: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let mut st = AudioState {
                render: default_id(&en, eRender, eMultimedia),
                render_comm: default_id(&en, eRender, eCommunications),
                capture: default_id(&en, eCapture, eMultimedia),
                capture_comm: default_id(&en, eCapture, eCommunications),
                endpoints: Vec::new(),
            };
            let col = en.EnumAudioEndpoints(eAll, DEVICE_STATE_ACTIVE).map_err(|e| e.to_string())?;
            let n = col.GetCount().map_err(|e| e.to_string())?;
            for i in 0..n {
                let Ok(d) = col.Item(i) else { continue };
                let Some(id) = device_id(&d) else { continue };
                let peak = d
                    .Activate::<IAudioMeterInformation>(CLSCTX_ALL, None)
                    .and_then(|m| m.GetPeakValue())
                    .unwrap_or(0.0);
                let (volume, muted) = d
                    .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                    .map(|v| {
                        let vol = v.GetMasterVolumeLevelScalar().unwrap_or(1.0);
                        let mut_b = v.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                        (vol, mut_b)
                    })
                    .unwrap_or((1.0, false));
                let (apps, sessions) = active_sessions(&d);
                st.endpoints.push(EndpointState { id, peak, volume, muted, apps, sessions });
            }
            Ok(st)
        })
    }
}

pub fn set_endpoint_mute(id: &str, muted: bool) -> Result<(), String> {
    unsafe {
        with_com(|| {
            let en: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let (d, _) = find_endpoint(&en, id)?;
            let ev = d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
            ev.SetMute(muted, std::ptr::null()).map_err(|e| e.to_string())
        })
    }
}

pub fn set_endpoint_volume(id: &str, volume: f32) -> Result<(), String> {
    let v = volume.clamp(0.0, 1.0);
    unsafe {
        with_com(|| {
            let en: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let (d, _) = find_endpoint(&en, id)?;
            let ev = d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
            ev.SetMasterVolumeLevelScalar(v, std::ptr::null()).map_err(|e| e.to_string())
        })
    }
}

pub fn set_session_volume(endpoint_id: &str, pid: u32, volume: f32, muted: bool) -> Result<(), String> {
    let v = volume.clamp(0.0, 1.0);
    unsafe {
        with_com(|| {
            let en: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let (d, _) = find_endpoint(&en, endpoint_id)?;
            let sm = d.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
            let se = sm.GetSessionEnumerator().map_err(|e| e.to_string())?;
            let n = se.GetCount().unwrap_or(0);
            let mut hit = false;
            for i in 0..n {
                let Ok(ctl) = se.GetSession(i) else { continue };
                let Ok(c2) = ctl.cast::<IAudioSessionControl2>() else { continue };
                if c2.GetProcessId().unwrap_or(u32::MAX) != pid {
                    continue;
                }
                if let Ok(sv) = ctl.cast::<ISimpleAudioVolume>() {
                    let _ = sv.SetMasterVolume(v, std::ptr::null());
                    let _ = sv.SetMute(muted, std::ptr::null());
                    hit = true;
                }
            }
            if hit {
                Ok(())
            } else {
                Err("そのアプリの音が見つかりませんでした".into())
            }
        })
    }
}

/// Windows の既定の音声機器を切り替える (IPolicyConfig COM インターフェース)。
/// role: "main" (いつも使う = eConsole + eMultimedia) / "comm" (通話で使う = eCommunications) / "all"
pub fn set_default_endpoint(id: &str, role: &str) -> Result<(), String> {
    const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
    const IID_POLICY_CONFIG: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);

    #[repr(C)]
    struct PolicyConfigVtbl {
        base: [usize; 13],
        set_default_endpoint: unsafe extern "system" fn(*mut std::ffi::c_void, PCWSTR, ERole) -> HRESULT,
    }
    #[repr(C)]
    struct PolicyConfigObj {
        vtbl: *const PolicyConfigVtbl,
    }

    unsafe {
        with_com(|| {
            let en: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let (_, raw_id) = find_endpoint(&en, id)?;
            let wid: Vec<u16> = raw_id.encode_utf16().chain([0]).collect();

            let unk: windows::core::IUnknown =
                CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL).map_err(|e| format!("音声切替の初期化に失敗: {e}"))?;
            let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            let hr = (unk.vtable().QueryInterface)(unk.as_raw(), &IID_POLICY_CONFIG, &mut ptr);
            if hr.is_err() || ptr.is_null() {
                return Err("この Windows では既定の機器の切り替えに対応していません".into());
            }
            let obj = ptr as *mut PolicyConfigObj;
            let set_fn = (*(*obj).vtbl).set_default_endpoint;
            let roles: &[ERole] = match role {
                "comm" => &[eCommunications],
                "all" => &[eConsole, eMultimedia, eCommunications],
                _ => &[eConsole, eMultimedia],
            };
            let mut err = None;
            for &r in roles {
                let res = set_fn(ptr, PCWSTR(wid.as_ptr()), r);
                if res.is_err() {
                    err = Some(format!("切り替えに失敗しました ({:?})", res));
                }
            }
            // Release
            let release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32 =
                std::mem::transmute((*(*obj).vtbl).base[2]);
            release(ptr);
            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn endpoint_id_becomes_instance_id() {
        assert_eq!(
            super::to_instance_id("{0.0.0.00000000}.{07146e51-a211-4543-9f0b-123456789abc}"),
            r"SWD\MMDEVAPI\{0.0.0.00000000}.{07146E51-A211-4543-9F0B-123456789ABC}"
        );
    }
}
