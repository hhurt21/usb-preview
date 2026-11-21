"""SRWare Iron がどの場所のポリシーを読むか試す (test_browser_cache.py の道具を使う)。"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import test_browser_cache as t  # noqa: E402

# iron.exe は本体 (chrome.exe) を起動するだけの起動役なので、本体で試す
exe = t.first(r"C:\Program Files\SRWare Iron (64-Bit)\chrome.exe", r"C:\Program Files\SRWare Iron\chrome.exe")
srv, url = t.serve()
mb = lambda n: f"{n / 1024 / 1024:.1f} MB"
print("Iron:", exe)
print("何もしない(大きめ指定):", mb(t.chromium_test(exe, url, ["--disk-cache-size=500000000"])))
print("起動の指定:", mb(t.chromium_test(exe, url, ["--disk-cache-size=8000000"])))
for key in [r"Software\Policies\Chromium", r"Software\Policies\SRWare Iron", r"Software\Policies\Iron"]:
    with t.TempPolicy(key):
        print(f"ポリシー[{key}]:", mb(t.chromium_test(exe, url)))
srv.shutdown()
