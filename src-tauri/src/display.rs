//! 画面 (モニター) の情報。Windows の画面設定と同じ QueryDisplayConfig から取る。

use serde::Serialize;
use windows::Win32::Devices::Display::*;
use windows::Win32::Foundation::ERROR_SUCCESS;

#[derive(Serialize, Clone, Debug)]
pub struct MonitorInfo {
    /// モニターが名乗った名前 (例: ROG PG258Q)
    pub name: String,
    /// モニターの機器 ID (DISPLAY\...)
    pub monitor_id: Option<String>,
    /// つながっている GPU の機器 ID (PCI\...)
    pub adapter_id: Option<String>,
    /// つなぎ方 (DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY の値)
    pub connection: i32,
    pub width: u32,
    pub height: u32,
    /// 画面設定での位置 (いちばん左上が 0,0 とは限らない)
    pub x: i32,
    pub y: i32,
    pub hz: f64,
    pub primary: bool,
}

fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// "\\?\DISPLAY#DELA1E6#5&abc&0&UID4352#{guid}" → "DISPLAY\DELA1E6\5&abc&0&UID4352"
pub fn device_path_to_id(path: &str) -> Option<String> {
    let p = path.strip_prefix(r"\\?\")?;
    let p = match p.rfind("#{") {
        Some(i) => &p[..i],
        None => p,
    };
    if p.is_empty() {
        return None;
    }
    Some(p.replace('#', "\\").to_ascii_uppercase())
}

pub fn list() -> Vec<MonitorInfo> {
    let mut out = Vec::new();
    unsafe {
        let mut np = 0u32;
        let mut nm = 0u32;
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) != ERROR_SUCCESS {
            return out;
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); np as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); nm as usize];
        if QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut np, paths.as_mut_ptr(), &mut nm, modes.as_mut_ptr(), None)
            != ERROR_SUCCESS
        {
            return out;
        }
        paths.truncate(np as usize);
        modes.truncate(nm as usize);

        for p in &paths {
            let mut tn = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            tn.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            tn.header.size = std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
            tn.header.adapterId = p.targetInfo.adapterId;
            tn.header.id = p.targetInfo.id;
            let (name, monitor_id, connection) = if DisplayConfigGetDeviceInfo(&mut tn.header) == 0 {
                (
                    from_wide(&tn.monitorFriendlyDeviceName),
                    device_path_to_id(&from_wide(&tn.monitorDevicePath)),
                    tn.outputTechnology.0,
                )
            } else {
                (String::new(), None, -1)
            };

            let mut an = DISPLAYCONFIG_ADAPTER_NAME::default();
            an.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADAPTER_NAME;
            an.header.size = std::mem::size_of::<DISPLAYCONFIG_ADAPTER_NAME>() as u32;
            an.header.adapterId = p.targetInfo.adapterId;
            let adapter_id = if DisplayConfigGetDeviceInfo(&mut an.header) == 0 {
                device_path_to_id(&from_wide(&an.adapterDevicePath))
            } else {
                None
            };

            let idx = p.sourceInfo.Anonymous.modeInfoIdx as usize;
            let (mut width, mut height, mut x, mut y) = (0, 0, 0, 0);
            if let Some(m) = modes.get(idx) {
                if m.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
                    let s = m.Anonymous.sourceMode;
                    width = s.width;
                    height = s.height;
                    x = s.position.x;
                    y = s.position.y;
                }
            }
            let r = p.targetInfo.refreshRate;
            let hz = if r.Denominator != 0 { r.Numerator as f64 / r.Denominator as f64 } else { 0.0 };

            out.push(MonitorInfo {
                name,
                monitor_id,
                adapter_id,
                connection,
                width,
                height,
                x,
                y,
                hz: (hz * 100.0).round() / 100.0,
                primary: x == 0 && y == 0,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_path_becomes_instance_id() {
        assert_eq!(
            device_path_to_id(r"\\?\DISPLAY#DELA1E6#5&abc&0&UID4352#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}").as_deref(),
            Some(r"DISPLAY\DELA1E6\5&ABC&0&UID4352")
        );
        assert_eq!(
            device_path_to_id(r"\\?\PCI#VEN_1002&DEV_73BF&SUBSYS_1#4&1#{5b45201d-f2f2-4f3b-85bb-30ff1f953599}").as_deref(),
            Some(r"PCI\VEN_1002&DEV_73BF&SUBSYS_1\4&1")
        );
        assert_eq!(device_path_to_id("no-prefix"), None);
    }
}
