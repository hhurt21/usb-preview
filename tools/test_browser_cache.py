"""ブラウザーのキャッシュを小さくする設定が、本当に効くかを試す。
試験用の Web サーバーで 3 MB のファイルを配り、一時的なプロファイル (試験専用のフォルダー) でブラウザーを
ヘッドレス (窓なし) で開いて、そのファイルがディスク キャッシュに保存されたかを見る。
ふだん使っているプロファイルには触れない。ポリシーを試すときは試験のあいだだけ書いて、終わったら元に戻す。

使い方: python tools/test_browser_cache.py
"""
import http.server
import os
import shutil
import subprocess
import tempfile
import threading
import time
import winreg

BIG = 3 * 1024 * 1024
LOCAL = os.environ["LOCALAPPDATA"]
PF = os.environ.get("ProgramFiles", r"C:\Program Files")
PF86 = os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)")


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_GET(self):
        if self.path.startswith("/big.js"):
            body = b"//" + b"x" * (BIG - 3) + b"\n"
            self.send_response(200)
            self.send_header("Content-Type", "text/javascript")
            self.send_header("Cache-Control", "public, max-age=86400")
        else:
            body = b"<!doctype html><title>t</title><script src='/big.js'></script>ok"
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def serve():
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv, f"http://127.0.0.1:{srv.server_address[1]}/"


MARK = b"x" * 65536


def largest_file(folder):
    """試験のファイルの中身 (x が続く文字) がキャッシュのどこかに入っていれば、その量を返す。入っていなければ 0"""
    found = 0
    for root, _, files in os.walk(folder):
        for f in files:
            try:
                with open(os.path.join(root, f), "rb") as fh:
                    data = fh.read()
            except OSError:
                continue
            if MARK in data:
                found += data.count(b"x" * 1024) * 1024
    return found


def run(cmd, wait=8):
    p = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(wait)
    subprocess.run(["taskkill", "/PID", str(p.pid), "/T", "/F"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1.5)


def chromium_test(exe, url, extra=()):
    d = tempfile.mkdtemp(prefix="cachetest_")
    try:
        run([exe, "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check",
             f"--user-data-dir={d}", *extra, url])
        return largest_file(os.path.join(d, "Default", "Cache"))
    finally:
        shutil.rmtree(d, ignore_errors=True)


def firefox_test(exe, url, prefs=None):
    d = tempfile.mkdtemp(prefix="cachetest_ff_")
    try:
        if prefs:
            with open(os.path.join(d, "user.js"), "w", encoding="utf-8") as f:
                f.write(prefs)
        run([exe, "-headless", "-no-remote", "-profile", d, url], wait=10)
        return largest_file(os.path.join(d, "cache2"))
    finally:
        shutil.rmtree(d, ignore_errors=True)


class TempPolicy:
    """試験のあいだだけ HKCU のポリシー DiskCacheSize を書き、元の値に戻す"""

    def __init__(self, key):
        self.key = key
        self.old = None
        self.existed = False

    def __enter__(self):
        k = winreg.CreateKey(winreg.HKEY_CURRENT_USER, self.key)
        try:
            self.old = winreg.QueryValueEx(k, "DiskCacheSize")[0]
            self.existed = True
        except FileNotFoundError:
            pass
        winreg.SetValueEx(k, "DiskCacheSize", 0, winreg.REG_DWORD, 8_000_000)
        winreg.CloseKey(k)

    def __exit__(self, *a):
        k = winreg.OpenKey(winreg.HKEY_CURRENT_USER, self.key, 0, winreg.KEY_ALL_ACCESS)
        if self.existed:
            winreg.SetValueEx(k, "DiskCacheSize", 0, winreg.REG_DWORD, self.old)
        else:
            winreg.DeleteValue(k, "DiskCacheSize")
        winreg.CloseKey(k)


def first(*paths):
    return next((p for p in paths if os.path.exists(p)), None)


CHROMIUMS = [
    ("Chrome", first(fr"{PF}\Google\Chrome\Application\chrome.exe", fr"{LOCAL}\Google\Chrome\Application\chrome.exe"), r"Software\Policies\Google\Chrome"),
    ("Edge", first(fr"{PF86}\Microsoft\Edge\Application\msedge.exe", fr"{PF}\Microsoft\Edge\Application\msedge.exe"), r"Software\Policies\Microsoft\Edge"),
    ("Brave", first(fr"{PF}\BraveSoftware\Brave-Browser\Application\brave.exe", fr"{LOCAL}\BraveSoftware\Brave-Browser\Application\brave.exe"), r"Software\Policies\BraveSoftware\Brave"),
    ("Vivaldi", first(fr"{LOCAL}\Vivaldi\Application\vivaldi.exe", fr"{PF}\Vivaldi\Application\vivaldi.exe"), r"Software\Policies\Vivaldi"),
]

FF_PREFS = (
    'user_pref("browser.cache.disk.smart_size.enabled", false);\n'
    'user_pref("browser.cache.disk.capacity", 8192);\n'
    'user_pref("browser.cache.disk.max_entry_size", 1024);\n'
)


def main():
    srv, url = serve()
    mb = lambda n: f"{n / 1024 / 1024:.1f} MB"
    print(f"試験のファイル: {mb(BIG)}（キャッシュに入れば、いちばん大きいファイルが 3 MB 前後になる）\n")
    for name, exe, key in CHROMIUMS:
        if not exe:
            print(f"{name}: 入っていない")
            continue
        base = chromium_test(exe, url, ["--disk-cache-size=500000000"])
        flag = chromium_test(exe, url, ["--disk-cache-size=8000000"])
        with TempPolicy(key):
            pol = chromium_test(exe, url)
        print(f"{name}: 何もしない(大きめ指定) {mb(base)} / 起動の指定 {mb(flag)} / ポリシー[{key}] {mb(pol)}")
    ff = first(fr"{PF}\Mozilla Firefox\firefox.exe", fr"{PF86}\Mozilla Firefox\firefox.exe")
    if ff:
        base = firefox_test(ff, url)
        small = firefox_test(ff, url, FF_PREFS)
        print(f"Firefox: 何もしない {mb(base)} / user.js {mb(small)}")
    srv.shutdown()


if __name__ == "__main__":
    main()
