//! いろいろなブラウザーのディスク キャッシュを小さくする (生配信の映像の切れ端を SSD に保存させない)。
//! やり方はブラウザーによって 3 通り:
//! - ポリシー (HKCU\Software\Policies\…\DiskCacheSize): Chromium 系で会社がポリシーに対応している物。どこから開いても効く
//! - user.js: Firefox 系。プロファイルごとの設定ファイルに書く
//! - ショートカット: ポリシーに対応していない Chromium 系。スタート・デスクトップ・タスク バーのショートカットに
//!   `--disk-cache-size=` を足す。ほかのアプリのリンクから開いたときは効かない
//! どれも自分のユーザーの中だけを変えるので、管理者の権限は要らない。

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

/// 8 MB。これより大きいファイル (1 MB を超える物) はキャッシュに入らなくなる
pub const SMALL_CACHE: u32 = 8_000_000;
const FLAG: &str = "--disk-cache-size=8000000";
const UJ_BEGIN: &str = "// ---- usb-watch: ディスク キャッシュの設定 ここから (このアプリが書いた。消してもよい) ----";
const UJ_END: &str = "// ---- usb-watch: ここまで ----";

#[derive(Clone, Copy, PartialEq)]
enum Method {
    Policy(&'static str),
    UserJs(&'static str),
    Shortcut,
}

struct Def {
    id: &'static str,
    name: &'static str,
    method: Method,
    /// この PC で本当に効くか試験した (tools/test_browser_cache.py)
    tested: bool,
    /// 入っているか調べる exe (%LOCALAPPDATA% などを含む)
    exes: &'static [&'static str],
    restart: &'static str,
}

const DEFS: &[Def] = &[
    Def { id: "chrome", name: "Google Chrome", method: Method::Policy(r"Software\Policies\Google\Chrome"), tested: true,
        exes: &[r"%ProgramFiles%\Google\Chrome\Application\chrome.exe", r"%ProgramFiles(x86)%\Google\Chrome\Application\chrome.exe", r"%LOCALAPPDATA%\Google\Chrome\Application\chrome.exe"], restart: "chrome://restart" },
    Def { id: "edge", name: "Microsoft Edge", method: Method::Policy(r"Software\Policies\Microsoft\Edge"), tested: true,
        exes: &[r"%ProgramFiles(x86)%\Microsoft\Edge\Application\msedge.exe", r"%ProgramFiles%\Microsoft\Edge\Application\msedge.exe"], restart: "edge://restart" },
    Def { id: "brave", name: "Brave", method: Method::Policy(r"Software\Policies\BraveSoftware\Brave"), tested: true,
        exes: &[r"%ProgramFiles%\BraveSoftware\Brave-Browser\Application\brave.exe", r"%LOCALAPPDATA%\BraveSoftware\Brave-Browser\Application\brave.exe"], restart: "brave://restart" },
    Def { id: "vivaldi", name: "Vivaldi", method: Method::Policy(r"Software\Policies\Vivaldi"), tested: true,
        exes: &[r"%LOCALAPPDATA%\Vivaldi\Application\vivaldi.exe", r"%ProgramFiles%\Vivaldi\Application\vivaldi.exe"], restart: "vivaldi://restart" },
    Def { id: "chromium", name: "Chromium", method: Method::Policy(r"Software\Policies\Chromium"), tested: false,
        exes: &[r"%LOCALAPPDATA%\Chromium\Application\chrome.exe", r"%ProgramFiles%\Chromium\Application\chrome.exe"], restart: "chrome://restart" },
    Def { id: "yandex", name: "Yandex Browser", method: Method::Policy(r"Software\Policies\YandexBrowser"), tested: false,
        exes: &[r"%LOCALAPPDATA%\Yandex\YandexBrowser\Application\browser.exe"], restart: "browser://restart" },
    Def { id: "opera", name: "Opera", method: Method::Shortcut, tested: false,
        exes: &[r"%LOCALAPPDATA%\Programs\Opera\opera.exe", r"%ProgramFiles%\Opera\opera.exe"], restart: "Opera を全部閉じて、ショートカットから開く" },
    Def { id: "operagx", name: "Opera GX", method: Method::Shortcut, tested: false,
        exes: &[r"%LOCALAPPDATA%\Programs\Opera GX\opera.exe"], restart: "Opera GX を全部閉じて、ショートカットから開く" },
    // Iron は Chromium と同じ場所のポリシーを読む (試験で確かめた)。Chromium も入っていれば一緒に変わる
    Def { id: "iron", name: "SRWare Iron", method: Method::Policy(r"Software\Policies\Chromium"), tested: true,
        exes: &[r"%ProgramFiles%\SRWare Iron (64-Bit)\chrome.exe", r"%ProgramFiles%\SRWare Iron\chrome.exe", r"%ProgramFiles(x86)%\SRWare Iron\chrome.exe"], restart: "Iron を全部閉じて開き直す" },
    Def { id: "thorium", name: "Thorium", method: Method::Shortcut, tested: false,
        exes: &[r"%LOCALAPPDATA%\Thorium\Application\thorium.exe", r"%ProgramFiles%\Thorium\Application\thorium.exe"], restart: "Thorium を全部閉じて、ショートカットから開く" },
    Def { id: "supermium", name: "Supermium", method: Method::Shortcut, tested: false,
        exes: &[r"%ProgramFiles%\Supermium\chrome.exe", r"%LOCALAPPDATA%\Supermium\chrome.exe"], restart: "Supermium を全部閉じて、ショートカットから開く" },
    Def { id: "firefox", name: "Firefox", method: Method::UserJs(r"%APPDATA%\Mozilla\Firefox\Profiles"), tested: true,
        exes: &[r"%ProgramFiles%\Mozilla Firefox\firefox.exe", r"%ProgramFiles(x86)%\Mozilla Firefox\firefox.exe"], restart: "Firefox を全部閉じて開き直す" },
    Def { id: "floorp", name: "Floorp", method: Method::UserJs(r"%APPDATA%\Floorp\Profiles"), tested: false,
        exes: &[r"%ProgramFiles%\Ablaze Floorp\floorp.exe", r"%LOCALAPPDATA%\Programs\Ablaze Floorp\floorp.exe"], restart: "Floorp を全部閉じて開き直す" },
    Def { id: "waterfox", name: "Waterfox", method: Method::UserJs(r"%APPDATA%\Waterfox\Profiles"), tested: false,
        exes: &[r"%ProgramFiles%\Waterfox\waterfox.exe"], restart: "Waterfox を全部閉じて開き直す" },
    Def { id: "librewolf", name: "LibreWolf", method: Method::UserJs(r"%APPDATA%\librewolf\Profiles"), tested: false,
        exes: &[r"%ProgramFiles%\LibreWolf\librewolf.exe"], restart: "LibreWolf を全部閉じて開き直す" },
    Def { id: "zen", name: "Zen Browser", method: Method::UserJs(r"%APPDATA%\zen\Profiles"), tested: false,
        exes: &[r"%ProgramFiles%\Zen Browser\zen.exe", r"%LOCALAPPDATA%\Zen Browser\zen.exe"], restart: "Zen を全部閉じて開き直す" },
    Def { id: "palemoon", name: "Pale Moon", method: Method::UserJs(r"%APPDATA%\Moonchild Productions\Pale Moon\Profiles"), tested: false,
        exes: &[r"%ProgramFiles%\Pale Moon\palemoon.exe"], restart: "Pale Moon を全部閉じて開き直す" },
];

#[derive(Serialize)]
pub struct BrowserCache {
    pub id: String,
    pub name: String,
    /// "policy" / "userjs" / "shortcut"
    pub method: String,
    pub tested: bool,
    pub installed: bool,
    /// "small" (小さくしている) / "default" (ブラウザーにまかせている) / "partial" (一部だけ) / "none" (設定する場所がない)
    pub state: String,
    /// 何に書いたか (プロファイルの数・ショートカットの数など)
    pub detail: String,
    pub restart: String,
}

fn expand(p: &str) -> PathBuf {
    let mut s = p.to_string();
    for var in ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA", "APPDATA", "USERPROFILE", "PUBLIC", "ProgramData"] {
        let pat = format!("%{var}%");
        if s.contains(&pat) {
            s = s.replace(&pat, &std::env::var(var).unwrap_or_default());
        }
    }
    PathBuf::from(s)
}

fn installed_exe(d: &Def) -> Option<PathBuf> {
    d.exes.iter().map(|e| expand(e)).find(|p| p.exists())
}

// ---------- ポリシー ----------
fn policy_get(key: &str) -> Option<u32> {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey(key).ok()?.get_value::<u32, _>("DiskCacheSize").ok()
}

fn policy_set(key: &str, small: bool) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if small {
        let (k, _) = hkcu.create_subkey(key).map_err(|e| e.to_string())?;
        k.set_value("DiskCacheSize", &SMALL_CACHE).map_err(|e| e.to_string())
    } else {
        if let Ok(k) = hkcu.open_subkey_with_flags(key, KEY_ALL_ACCESS) {
            let _ = k.delete_value("DiskCacheSize");
        }
        Ok(())
    }
}

// ---------- user.js (Firefox 系) ----------
fn profiles(root: &str) -> Vec<PathBuf> {
    std::fs::read_dir(expand(root))
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.join("prefs.js").exists()).collect())
        .unwrap_or_default()
}

/// user.js の中の、このアプリの部分だけを取り出す・置き換える
fn split_userjs(text: &str) -> (String, Option<String>) {
    match (text.find(UJ_BEGIN), text.find(UJ_END)) {
        (Some(b), Some(e)) if e > b => {
            let block = text[b..e + UJ_END.len()].to_string();
            let rest = format!("{}{}", &text[..b], &text[e + UJ_END.len()..]);
            (rest.trim_end().to_string(), Some(block))
        }
        _ => (text.trim_end().to_string(), None),
    }
}

pub fn userjs_block(small: bool) -> String {
    let body = if small {
        "user_pref(\"browser.cache.disk.smart_size.enabled\", false);\n\
         user_pref(\"browser.cache.disk.capacity\", 8192);\n\
         user_pref(\"browser.cache.disk.max_entry_size\", 1024);"
    } else {
        // user.js を消しても前の値が残る作りなので、元の値 (Firefox の最初の設定) を書いて戻す
        "user_pref(\"browser.cache.disk.smart_size.enabled\", true);\n\
         user_pref(\"browser.cache.disk.capacity\", 256000);\n\
         user_pref(\"browser.cache.disk.max_entry_size\", 51200);"
    };
    format!("{UJ_BEGIN}\n{body}\n{UJ_END}\n")
}

fn userjs_is_small(dir: &Path) -> bool {
    let text = std::fs::read_to_string(dir.join("user.js")).unwrap_or_default();
    matches!(split_userjs(&text).1, Some(b) if b.contains("\"browser.cache.disk.capacity\", 8192"))
}

fn userjs_set(dir: &Path, small: bool) -> Result<(), String> {
    let path = dir.join("user.js");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let (rest, block) = split_userjs(&text);
    // もともとこのアプリが書いていなくて、元に戻すだけなら何もしない
    if !small && block.is_none() {
        return Ok(());
    }
    let mut out = rest;
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(&userjs_block(small));
    std::fs::write(&path, out).map_err(|e| format!("{}: {e}", path.display()))
}

// ---------- ショートカット ----------
fn shortcut_dirs() -> Vec<PathBuf> {
    [
        r"%APPDATA%\Microsoft\Windows\Start Menu\Programs",
        r"%ProgramData%\Microsoft\Windows\Start Menu\Programs",
        r"%USERPROFILE%\Desktop",
        r"%PUBLIC%\Desktop",
        r"%APPDATA%\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar",
    ]
    .iter()
    .map(|d| expand(d))
    .filter(|p| p.exists())
    .collect()
}

fn ps(script: &str) -> Result<String, String> {
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// exe を指すショートカットを探して、指定を足す (small) か外す。mode="read" なら数えるだけ。
/// 返事: 1 行 = "パス|指定が入っているか(1/0)|書けたか(1/0/-)"
fn shortcuts(exe: &Path, mode: &str) -> Vec<(String, bool, Option<bool>)> {
    let dirs: Vec<String> = shortcut_dirs().iter().map(|d| ps_quote(&d.to_string_lossy())).collect();
    if dirs.is_empty() {
        return Vec::new();
    }
    let script = format!(
        r#"[Console]::OutputEncoding = [Text.Encoding]::UTF8
$sh = New-Object -ComObject WScript.Shell
$exe = {exe}
$flag = '{FLAG}'
foreach ($d in @({dirs})) {{
  Get-ChildItem -LiteralPath $d -Recurse -Filter *.lnk -ErrorAction SilentlyContinue | ForEach-Object {{
    $l = $sh.CreateShortcut($_.FullName)
    if ($l.TargetPath -ne $exe) {{ return }}
    $has = $l.Arguments -like "*--disk-cache-size=*"
    $ok = '-'
    if ('{mode}' -eq 'small' -and -not $has) {{
      $l.Arguments = ($l.Arguments + ' ' + $flag).Trim()
      try {{ $l.Save(); $ok = '1'; $has = $true }} catch {{ $ok = '0' }}
    }} elseif ('{mode}' -eq 'default' -and $has) {{
      $l.Arguments = (($l.Arguments -replace '\s*--disk-cache-size=\S+', '')).Trim()
      try {{ $l.Save(); $ok = '1'; $has = $false }} catch {{ $ok = '0' }}
    }}
    "$($_.FullName)|$([int]$has)|$ok"
  }}
}}"#,
        exe = ps_quote(&exe.to_string_lossy()),
        dirs = dirs.join(","),
    );
    ps(&script)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut it = l.trim().rsplitn(3, '|');
            let ok = it.next()?;
            let has = it.next()? == "1";
            let path = it.next()?.to_string();
            Some((path, has, match ok { "1" => Some(true), "0" => Some(false), _ => None }))
        })
        .collect()
}

// ---------- まとめ ----------
fn status(d: &Def) -> BrowserCache {
    let exe = installed_exe(d);
    let (method, state, detail) = match d.method {
        Method::Policy(key) => {
            let v = policy_get(key);
            let st = match v {
                Some(n) if n <= 16_000_000 => "small",
                Some(_) => "partial",
                None => "default",
            };
            ("policy", st.to_string(), match v { Some(n) => format!("ポリシー {} MB", n / 1_000_000), None => String::new() })
        }
        Method::UserJs(root) => {
            let ps = profiles(root);
            let small = ps.iter().filter(|p| userjs_is_small(p)).count();
            let st = if ps.is_empty() { "none" } else if small == ps.len() { "small" } else if small == 0 { "default" } else { "partial" };
            ("userjs", st.to_string(), format!("プロファイル {} 個中 {} 個", ps.len(), small))
        }
        Method::Shortcut => match &exe {
            Some(e) => {
                let list = shortcuts(e, "read");
                let with = list.iter().filter(|x| x.1).count();
                let st = if list.is_empty() { "none" } else if with == list.len() { "small" } else if with == 0 { "default" } else { "partial" };
                ("shortcut", st.to_string(), format!("ショートカット {} 個中 {} 個", list.len(), with))
            }
            None => ("shortcut", "none".to_string(), String::new()),
        },
    };
    BrowserCache {
        id: d.id.into(),
        name: d.name.into(),
        method: method.into(),
        tested: d.tested,
        installed: exe.is_some(),
        state,
        detail,
        restart: d.restart.into(),
    }
}

pub fn list() -> Vec<BrowserCache> {
    DEFS.iter().map(status).collect()
}

/// small=true でキャッシュを小さく、false で元に戻す。書けなかった所があれば Err に理由
pub fn set(id: &str, small: bool) -> Result<Vec<BrowserCache>, String> {
    let d = DEFS.iter().find(|d| d.id == id).ok_or("知らないブラウザーです")?;
    let mut errors: Vec<String> = Vec::new();
    match d.method {
        Method::Policy(key) => policy_set(key, small)?,
        Method::UserJs(root) => {
            for p in profiles(root) {
                if let Err(e) = userjs_set(&p, small) {
                    errors.push(e);
                }
            }
        }
        Method::Shortcut => {
            let exe = installed_exe(d).ok_or("このブラウザーは入っていません")?;
            for (path, _, ok) in shortcuts(&exe, if small { "small" } else { "default" }) {
                if ok == Some(false) {
                    errors.push(format!("{path}（みんなのスタート メニューの物は管理者の権限が要ります）"));
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(list())
    } else {
        Err(format!("書けなかった所があります: {}", errors.join(" / ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn userjs_block_is_replaced_not_duplicated() {
        let user = "user_pref(\"a\", 1);";
        let once = format!("{user}\n\n{}", userjs_block(true));
        let (rest, block) = split_userjs(&once);
        assert_eq!(rest, user);
        assert!(block.unwrap().contains("8192"));
        // 元に戻すと、同じ場所が元の値に置き換わる (ほかの行は残る)
        let back = format!("{rest}\n\n{}", userjs_block(false));
        let (rest2, block2) = split_userjs(&back);
        assert_eq!(rest2, user);
        assert!(block2.unwrap().contains("256000"));
        assert_eq!(back.matches(UJ_BEGIN).count(), 1);
    }

    #[test]
    fn expands_env_vars() {
        let p = expand(r"%LOCALAPPDATA%\x");
        assert!(!p.to_string_lossy().contains('%'));
    }
}
