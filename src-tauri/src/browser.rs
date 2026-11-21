//! ブラウザー (Chrome・Edge) のディスク キャッシュの大きさを、ポリシーで決める。
//! 生配信を見ると、映像の切れ端 (1 つ 1〜2 MB) を全部ディスクに保存して SSD を減らすので、
//! キャッシュを小さくして、大きなファイルを保存させないようにする。
//! 自分のユーザーの設定 (HKCU) だけを変えるので、管理者の権限は要らない。消せば元に戻る。

use serde::Serialize;
use winreg::enums::*;
use winreg::RegKey;

/// 8 MB。これより大きいファイルの多くはキャッシュに入らなくなり、普通のページの部品は入る
pub const SMALL_CACHE: u32 = 8_000_000;

const BROWSERS: [(&str, &str, &str); 2] = [
    ("chrome", "Google Chrome", r"Software\Policies\Google\Chrome"),
    ("edge", "Microsoft Edge", r"Software\Policies\Microsoft\Edge"),
];

#[derive(Serialize)]
pub struct CachePolicy {
    pub id: String,
    pub name: String,
    /// 今の設定 (バイト)。None なら設定なし (ブラウザーにまかせる)
    pub size: Option<u32>,
    /// ブラウザーが入っているか
    pub installed: bool,
}

fn installed(id: &str) -> bool {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let pf = std::env::var("ProgramFiles").unwrap_or_default();
    let pf86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();
    let paths: Vec<String> = match id {
        "chrome" => vec![
            format!(r"{pf}\Google\Chrome\Application\chrome.exe"),
            format!(r"{pf86}\Google\Chrome\Application\chrome.exe"),
            format!(r"{local}\Google\Chrome\Application\chrome.exe"),
        ],
        _ => vec![
            format!(r"{pf86}\Microsoft\Edge\Application\msedge.exe"),
            format!(r"{pf}\Microsoft\Edge\Application\msedge.exe"),
        ],
    };
    paths.iter().any(|p| std::path::Path::new(p).exists())
}

pub fn get() -> Vec<CachePolicy> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    BROWSERS
        .iter()
        .map(|(id, name, key)| CachePolicy {
            id: id.to_string(),
            name: name.to_string(),
            size: hkcu.open_subkey(key).ok().and_then(|k| k.get_value::<u32, _>("DiskCacheSize").ok()),
            installed: installed(id),
        })
        .collect()
}

/// small=true でキャッシュを小さく、false で設定を消してブラウザーにまかせる
pub fn set(id: &str, small: bool) -> Result<Vec<CachePolicy>, String> {
    let (_, _, key) = BROWSERS.iter().find(|(i, _, _)| *i == id).ok_or("知らないブラウザーです")?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if small {
        let (k, _) = hkcu.create_subkey(key).map_err(|e| e.to_string())?;
        k.set_value("DiskCacheSize", &SMALL_CACHE).map_err(|e| e.to_string())?;
    } else if let Ok(k) = hkcu.open_subkey_with_flags(key, KEY_ALL_ACCESS) {
        let _ = k.delete_value("DiskCacheSize");
    }
    Ok(get())
}
