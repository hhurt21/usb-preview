"""試験用の空のプロファイルで、「終わらせて --restore-last-session で開き直す」とタブが戻るかを試す。
終わらせるのは、試験用のプロファイルを使っているプロセスだけ (ふだん使っているブラウザーには触れない)。
"""
import shutil
import subprocess
import sys
import tempfile
import time

PS_TABS = r"""
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$ids = @(Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*{DIR}*' } | Select-Object -ExpandProperty ProcessId)
$names = @()
foreach ($p in Get-Process -Id $ids -ErrorAction SilentlyContinue) {
  if ($p.MainWindowHandle -eq 0) { continue }
  $root = [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)
  $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::TabItem)
  $names += @($root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond) | ForEach-Object { $_.Current.Name } | Where-Object { $_ -notmatch '語$' })
}
$names -join ' | '
"""
PS_KILL = r"""Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*{DIR}*' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }"""


def ps(script):
    return subprocess.run(["powershell", "-NoProfile", "-Command", script], capture_output=True, text=True, encoding="utf-8", errors="replace").stdout.strip()


def test(name, exe):
    d = tempfile.mkdtemp(prefix="restoretest_")
    try:
        common = [exe, f"--user-data-dir={d}", "--no-first-run", "--no-default-browser-check"]
        subprocess.Popen(common + ["https://example.com/", "https://example.org/"])
        time.sleep(12)
        before = ps(PS_TABS.replace("{DIR}", d))
        ps(PS_KILL.replace("{DIR}", d))
        time.sleep(2)
        subprocess.Popen(common + ["--restore-last-session"])
        time.sleep(10)
        after = ps(PS_TABS.replace("{DIR}", d))
        ps(PS_KILL.replace("{DIR}", d))
        time.sleep(2)
        print(f"{name}\n  前: {before}\n  後: {after}")
    finally:
        shutil.rmtree(d, ignore_errors=True)


for name, exe in [
    ("Brave", r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"),
    ("Chrome", r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
]:
    test(name, exe)
