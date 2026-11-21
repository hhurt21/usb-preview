//! 今使っている音の機器。
//! - 既定の機器 (いつも使う / 通話で使う) を、鳴らす側と録る側それぞれ
//! - 機器ごとの今の音の大きさ (0〜1)
//! - その機器で鳴らしている (録っている) アプリ

use serde::Serialize;
use windows::core::{Interface, PWSTR};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

#[derive(Serialize, Default)]
pub struct EndpointState {
    /// 機器の ID (SWD\MMDEVAPI\{...}.{...}、大文字)
    pub id: String,
    /// 今の音の大きさ 0〜1
    pub peak: f32,
    /// 鳴らしている / 録っているアプリ
    pub apps: Vec<String>,
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

unsafe fn device_id(d: &IMMDevice) -> Option<String> {
    let p: PWSTR = d.GetId().ok()?;
    let s = p.to_string().ok();
    CoTaskMemFree(Some(p.0 as *const _));
    s.map(|s| to_instance_id(&s))
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

unsafe fn active_apps(d: &IMMDevice) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Ok(sm) = d.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else { return out };
    let Ok(se) = sm.GetSessionEnumerator() else { return out };
    let n = se.GetCount().unwrap_or(0);
    for i in 0..n {
        let Ok(ctl) = se.GetSession(i) else { continue };
        if ctl.GetState().ok() != Some(AudioSessionStateActive) {
            continue;
        }
        let Ok(c2) = ctl.cast::<IAudioSessionControl2>() else { continue };
        let pid = c2.GetProcessId().unwrap_or(0);
        let name = process_name(pid);
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

pub fn state() -> Result<AudioState, String> {
    unsafe {
        // 別の形で初期化済みでも続けてよい (そのときは失敗が返るが使える)
        let inited = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        let result = (|| -> Result<AudioState, String> {
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
                st.endpoints.push(EndpointState { id, peak, apps: active_apps(&d) });
            }
            Ok(st)
        })();
        if inited {
            CoUninitialize();
        }
        result
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
