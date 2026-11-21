//! USB 機器の一覧と、有効・無効の切り替え (SetupAPI / CfgMgr32)。

use serde::Serialize;
use windows::core::{w, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::*;
use windows::Win32::Devices::Properties::*;

#[derive(Serialize, Clone, Debug)]
pub struct Dev {
    pub id: String,
    /// Windows が付けた名前 (フレンドリ名 → 説明 の順)
    pub name: String,
    /// 機器が自分で名乗った名前 (無いことも多い)
    pub bus_name: Option<String>,
    pub manufacturer: Option<String>,
    pub class: Option<String>,
    pub parent: Option<String>,
    pub location: Option<String>,
    pub port: Option<u32>,
    pub present: bool,
    /// ok / problem / disabled / absent
    pub status: String,
    pub problem: u32,
    pub last_arrival: Option<i64>,
    pub last_removal: Option<i64>,
    pub vid: Option<String>,
    pub pid: Option<String>,
    /// 複合機器の部品 (&MI_xx)
    pub is_interface: bool,
    pub is_hub: bool,
    /// 安全に取り外せる機器か (CM_DEVCAP_REMOVABLE)
    pub removable: bool,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

unsafe fn prop_raw(h: HDEVINFO, d: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<(u32, Vec<u8>)> {
    let mut ty = DEVPROPTYPE(0);
    let mut need = 0u32;
    let _ = SetupDiGetDevicePropertyW(h, d, key, &mut ty, None, Some(&mut need), 0);
    if need == 0 {
        return None;
    }
    let mut buf = vec![0u8; need as usize];
    SetupDiGetDevicePropertyW(h, d, key, &mut ty, Some(&mut buf), None, 0).ok()?;
    Some((ty.0, buf))
}

unsafe fn prop_str(h: HDEVINFO, d: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<String> {
    let (_, b) = prop_raw(h, d, key)?;
    let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let s = from_wide(&u);
    let s = s.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// FILETIME → UNIX ミリ秒
unsafe fn prop_time(h: HDEVINFO, d: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<i64> {
    let (_, b) = prop_raw(h, d, key)?;
    if b.len() < 8 {
        return None;
    }
    let ft = u64::from_le_bytes(b[..8].try_into().ok()?) as i64;
    if ft <= 116_444_736_000_000_000 {
        return None;
    }
    Some((ft - 116_444_736_000_000_000) / 10_000)
}

fn parse_port(loc: &str) -> Option<u32> {
    // "Port_#0001.Hub_#0003"
    let i = loc.find("Port_#")? + 6;
    let digits: String = loc[i..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn parse_hex_after(id: &str, tag: &str) -> Option<String> {
    let up = id.to_ascii_uppercase();
    let i = up.find(tag)? + tag.len();
    Some(up[i..].chars().take(4).collect())
}

unsafe fn read_dev(h: HDEVINFO, d: &SP_DEVINFO_DATA) -> Option<Dev> {
    let mut idbuf = [0u16; 512];
    if CM_Get_Device_IDW(d.DevInst, &mut idbuf, 0) != CR_SUCCESS {
        return None;
    }
    let id = from_wide(&idbuf);

    let mut st = CM_DEVNODE_STATUS_FLAGS(0);
    let mut prob = CM_PROB(0);
    let present = CM_Get_DevNode_Status(&mut st, &mut prob, d.DevInst, 0) == CR_SUCCESS;
    let has_problem = present && (st.0 & DN_HAS_PROBLEM.0) != 0;
    let status = if !present {
        "absent"
    } else if has_problem && prob == CM_PROB_DISABLED {
        "disabled"
    } else if has_problem {
        "problem"
    } else {
        "ok"
    };

    let friendly = prop_str(h, d, &DEVPKEY_Device_FriendlyName);
    let desc = prop_str(h, d, &DEVPKEY_Device_DeviceDesc);
    let name = friendly.or(desc).unwrap_or_else(|| id.clone());
    let location = prop_str(h, d, &DEVPKEY_Device_LocationInfo);
    let port = location.as_deref().and_then(parse_port);
    let up = id.to_ascii_uppercase();
    let class = prop_str(h, d, &DEVPKEY_Device_Class);
    let caps = prop_raw(h, d, &DEVPKEY_Device_Capabilities)
        .and_then(|(_, b)| if b.len() >= 4 { Some(u32::from_le_bytes(b[..4].try_into().ok()?)) } else { None })
        .unwrap_or(0);
    let removable = (caps & 0x0000_0004) != 0; // CM_DEVCAP_REMOVABLE

    Some(Dev {
        bus_name: prop_str(h, d, &DEVPKEY_Device_BusReportedDeviceDesc),
        manufacturer: prop_str(h, d, &DEVPKEY_Device_Manufacturer),
        parent: prop_str(h, d, &DEVPKEY_Device_Parent),
        location,
        port,
        present,
        status: status.to_string(),
        problem: if has_problem { prob.0 } else { 0 },
        last_arrival: prop_time(h, d, &DEVPKEY_Device_LastArrivalDate),
        last_removal: prop_time(h, d, &DEVPKEY_Device_LastRemovalDate),
        vid: parse_hex_after(&id, "VID_"),
        pid: parse_hex_after(&id, "PID_"),
        is_interface: up.contains("&MI_"),
        is_hub: up.contains("ROOT_HUB") || name.contains("ハブ") || name.to_ascii_lowercase().contains("hub"),
        removable,
        class,
        name,
        id,
    })
}

/// USB 列挙子の機器を全部。present_only=false なら過去につないだ機器も含む。
pub fn list(present_only: bool) -> Vec<Dev> {
    let mut out = Vec::new();
    unsafe {
        let mut flags = DIGCF_ALLCLASSES;
        if present_only {
            flags |= DIGCF_PRESENT;
        }
        let Ok(h) = SetupDiGetClassDevsW(None, w!("USB"), None, flags) else {
            return out;
        };
        let mut i = 0u32;
        loop {
            let mut d = SP_DEVINFO_DATA {
                cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
                ..Default::default()
            };
            if SetupDiEnumDeviceInfo(h, i, &mut d).is_err() {
                break;
            }
            if let Some(dev) = read_dev(h, &d) {
                if !present_only || dev.present {
                    out.push(dev);
                }
            }
            i += 1;
        }
        let _ = SetupDiDestroyDeviceInfoList(h);
    }
    out
}

/// USB に限らず、今つながっている全部の機器
pub fn list_all_present() -> Vec<Dev> {
    let mut out = Vec::new();
    unsafe {
        let Ok(h) = SetupDiGetClassDevsW(None, PCWSTR::null(), None, DIGCF_ALLCLASSES | DIGCF_PRESENT) else {
            return out;
        };
        let mut i = 0u32;
        loop {
            let mut d = SP_DEVINFO_DATA {
                cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
                ..Default::default()
            };
            if SetupDiEnumDeviceInfo(h, i, &mut d).is_err() {
                break;
            }
            if let Some(dev) = read_dev(h, &d) {
                out.push(dev);
            }
            i += 1;
        }
        let _ = SetupDiDestroyDeviceInfoList(h);
    }
    out
}

/// ID を指定して 1 台だけ読む (USB 以外のコントローラーの名前にも使う)
pub fn get(id: &str) -> Option<Dev> {
    unsafe {
        let h = SetupDiCreateDeviceInfoList(None, None).ok()?;
        let mut d = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        let wid = wide(id);
        let r = if SetupDiOpenDeviceInfoW(h, PCWSTR(wid.as_ptr()), None, 0, Some(&mut d)).is_ok() {
            read_dev(h, &d)
        } else {
            None
        };
        let _ = SetupDiDestroyDeviceInfoList(h);
        r
    }
}

/// 有効・無効の切り替え (デバイスマネージャーと同じやり方)。管理者の権限が要る。
pub fn set_enabled(id: &str, enable: bool) -> Result<(), String> {
    unsafe {
        let h = SetupDiCreateDeviceInfoList(None, None).map_err(|e| e.to_string())?;
        let result = (|| {
            let mut d = SP_DEVINFO_DATA {
                cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
                ..Default::default()
            };
            let wid = wide(id);
            SetupDiOpenDeviceInfoW(h, PCWSTR(wid.as_ptr()), None, 0, Some(&mut d))
                .map_err(|e| format!("機器が見つかりません: {e}"))?;
            let p = SP_PROPCHANGE_PARAMS {
                ClassInstallHeader: SP_CLASSINSTALL_HEADER {
                    cbSize: std::mem::size_of::<SP_CLASSINSTALL_HEADER>() as u32,
                    InstallFunction: DIF_PROPERTYCHANGE,
                },
                StateChange: if enable { DICS_ENABLE } else { DICS_DISABLE },
                Scope: DICS_FLAG_GLOBAL,
                HwProfile: 0,
            };
            SetupDiSetClassInstallParamsW(
                h,
                Some(&d),
                Some(&p.ClassInstallHeader),
                std::mem::size_of::<SP_PROPCHANGE_PARAMS>() as u32,
            )
            .map_err(|e| format!("設定できませんでした: {e}"))?;
            SetupDiCallClassInstaller(DIF_PROPERTYCHANGE, h, Some(&d))
                .map_err(|e| format!("切り替えられませんでした: {e}"))?;
            Ok(())
        })();
        let _ = SetupDiDestroyDeviceInfoList(h);
        result
    }
}

/// 機器を安全に取り外す (Windows の「ハードウェアを安全に取り外してメディアを取り出す」と同じ)。
/// 管理者の権限は要らない。
pub fn eject_device(id: &str) -> Result<(), String> {
    unsafe {
        let mut devinst = 0u32;
        let wid = wide(id);
        let r = CM_Locate_DevNodeW(&mut devinst, PCWSTR(wid.as_ptr()), CM_LOCATE_DEVNODE_NORMAL);
        if r != CR_SUCCESS {
            return Err("機器が見つかりませんでした (すでに取り外されているかもしれません)".into());
        }
        let mut veto_type = PNP_VETO_TYPE(0);
        let mut veto_name = [0u16; 260];
        let er = CM_Request_Device_EjectW(
            devinst,
            Some(&mut veto_type),
            Some(&mut veto_name),
            0,
        );
        if er == CR_SUCCESS {
            Ok(())
        } else if er == CR_REMOVE_VETOED {
            let reason = from_wide(&veto_name);
            Err(format!(
                "今は取り外せませんでした (ファイルやアプリが使用中です{})",
                if reason.is_empty() { String::new() } else { format!("：{reason}") }
            ))
        } else {
            Err(format!("安全に取り外せませんでした (コード {:?})", er.0))
        }
    }
}

/// 今つながっていない過去の USB 機器 (ゴーストデバイス) を Windows の一覧から削除する。
/// 安全のため、現在接続中の機器は絶対に消さない。管理者の権限が必要。
pub fn remove_ghost_device(id: &str) -> Result<(), String> {
    if !id.to_ascii_uppercase().starts_with("USB\\") {
        return Err("安全のため USB 機器だけ削除できます".into());
    }
    if let Some(d) = get(id) {
        if d.present {
            return Err("現在つながっている機器は削除できません (先に抜いてください)".into());
        }
    }
    unsafe {
        let h = SetupDiCreateDeviceInfoList(None, None).map_err(|e| e.to_string())?;
        let result = (|| {
            let mut d = SP_DEVINFO_DATA {
                cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
                ..Default::default()
            };
            let wid = wide(id);
            SetupDiOpenDeviceInfoW(h, PCWSTR(wid.as_ptr()), None, 0, Some(&mut d))
                .map_err(|e| format!("機器が見つかりませんでした: {e}"))?;
            SetupDiCallClassInstaller(DIF_REMOVE, h, Some(&d))
                .map_err(|e| format!("削除できませんでした: {e}"))?;
            Ok(())
        })();
        let _ = SetupDiDestroyDeviceInfoList(h);
        result
    }
}
