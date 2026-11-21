# Windows の再起動マネージャー (Restart Manager) で、ブラウザーを「保存してから閉じて、開き直す」とタブが戻るか試す。
# 試験用のプロファイルのブラウザーだけを対象にする。
param([string]$Exe, [string]$Dir)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class Rm {
  [StructLayout(LayoutKind.Sequential)] public struct FILETIME { public uint Low; public uint High; }
  [StructLayout(LayoutKind.Sequential)] public struct RM_UNIQUE_PROCESS { public int dwProcessId; public FILETIME ProcessStartTime; }
  [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)] public static extern int RmStartSession(out uint h, int flags, string key);
  [DllImport("rstrtmgr.dll")] public static extern int RmEndSession(uint h);
  [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)] public static extern int RmRegisterResources(uint h, uint nFiles, string[] files, uint nApps, RM_UNIQUE_PROCESS[] apps, uint nSvc, string[] svcs);
  [DllImport("rstrtmgr.dll")] public static extern int RmShutdown(uint h, uint flags, IntPtr cb);
  [DllImport("rstrtmgr.dll")] public static extern int RmRestart(uint h, int reserved, IntPtr cb);
  [DllImport("kernel32.dll")] public static extern IntPtr OpenProcess(uint a, bool i, int pid);
  [DllImport("kernel32.dll")] public static extern bool GetProcessTimes(IntPtr h, out FILETIME c, out FILETIME e, out FILETIME k, out FILETIME u);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
  public static string Run(int[] pids) {
    uint s; var key = Guid.NewGuid().ToString("N");
    int r = RmStartSession(out s, 0, key); if (r != 0) return "start " + r;
    var apps = new RM_UNIQUE_PROCESS[pids.Length];
    for (int i = 0; i < pids.Length; i++) {
      var h = OpenProcess(0x1000, false, pids[i]); FILETIME c, e, k, u; GetProcessTimes(h, out c, out e, out k, out u); CloseHandle(h);
      apps[i].dwProcessId = pids[i]; apps[i].ProcessStartTime = c;
    }
    r = RmRegisterResources(s, 0, null, (uint)apps.Length, apps, 0, null); if (r != 0) { RmEndSession(s); return "register " + r; }
    r = RmShutdown(s, 0, IntPtr.Zero); var sd = r;
    System.Threading.Thread.Sleep(1500);
    r = RmRestart(s, 0, IntPtr.Zero);
    RmEndSession(s);
    return "shutdown=" + sd + " restart=" + r;
  }
}
"@
function Tabs {
  $ids = @(Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like "*$Dir*" -and $_.Name -eq (Split-Path $Exe -Leaf) } | Select-Object -ExpandProperty ProcessId)
  $names = @()
  foreach ($p in Get-Process -Id $ids -ErrorAction SilentlyContinue) {
    if ($p.MainWindowHandle -eq 0) { continue }
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)
    $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::TabItem)
    $names += @($root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond) | ForEach-Object { $_.Current.Name } | Where-Object { $_ -notmatch '語$' })
  }
  $names -join ' | '
}
New-Item -ItemType Directory -Force $Dir | Out-Null
Start-Process $Exe "--user-data-dir=`"$Dir`" --no-first-run --no-default-browser-check https://example.com/ https://example.org/"
Start-Sleep 12
"前: " + (Tabs)
# ブラウザー本体 (--type= の無いプロセス) を対象にする
$main = @(Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like "*$Dir*" -and $_.Name -eq (Split-Path $Exe -Leaf) -and $_.CommandLine -notmatch '--type=' } | Select-Object -ExpandProperty ProcessId)
"本体: $($main -join ',')"
"再起動マネージャー: " + [Rm]::Run([int[]]$main)
Start-Sleep 10
"後: " + (Tabs)
Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like "*$Dir*" -and $_.Name -eq (Split-Path $Exe -Leaf) } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Start-Sleep 2; Remove-Item -Recurse -Force $Dir -ErrorAction SilentlyContinue
