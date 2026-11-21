# 再起動マネージャーで閉じたあと、A: 再起動マネージャーに開き直してもらう (70 秒待ってから) / B: 自分で --restore-last-session で開く
param([string]$Exe, [ValidateSet('A', 'B')][string]$Mode)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
. "$PSScriptRoot\rm_helper.ps1"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class Rm2 {
  [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)] public static extern int RmStartSession(out uint h, int flags, string key);
  [DllImport("rstrtmgr.dll")] public static extern int RmEndSession(uint h);
  [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)] public static extern int RmRegisterResources(uint h, uint nFiles, string[] files, uint nApps, Rm.RM_UNIQUE_PROCESS[] apps, uint nSvc, string[] svcs);
  [DllImport("rstrtmgr.dll")] public static extern int RmShutdown(uint h, uint flags, IntPtr cb);
  public static int ShutdownOnly(int pid) {
    uint s; RmStartSession(out s, 0, Guid.NewGuid().ToString("N"));
    var h = Rm.OpenProcess(0x1000, false, pid); Rm.FILETIME c, e, k, u; Rm.GetProcessTimes(h, out c, out e, out k, out u); Rm.CloseHandle(h);
    var apps = new Rm.RM_UNIQUE_PROCESS[1]; apps[0].dwProcessId = pid; apps[0].ProcessStartTime = c;
    RmRegisterResources(s, 0, null, 1, apps, 0, null);
    int r = RmShutdown(s, 0, IntPtr.Zero);
    RmEndSession(s);
    return r;
  }
}
"@
$name = [IO.Path]::GetFileNameWithoutExtension($Exe)
function Main { @(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq $Exe -and $_.CommandLine -notmatch '--type=' } | Select-Object -ExpandProperty ProcessId) }
function Tabs {
  $names = @()
  foreach ($p in @(Get-Process $name -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 })) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)
    $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::TabItem)
    $names += @($root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond) | ForEach-Object { $_.Current.Name } | Where-Object { $_ -notmatch '語$' })
  }
  $names -join ' | '
}
if ((Main).Count) { "もう開いているので試験しません"; exit }
Start-Process $Exe '--new-window https://example.com/ https://example.org/'
Start-Sleep $(if ($Mode -eq 'A') { 70 } else { 12 })
"前: " + (Tabs)
if ($Mode -eq 'A') {
  "再起動マネージャー (閉じて開き直す): " + [Rm]::Run([int[]](Main))
} else {
  "再起動マネージャー (閉じるだけ): " + [Rm2]::ShutdownOnly((Main)[0])
  Start-Sleep 3
  "閉じたあと残り: $(@(Get-Process $name -ErrorAction SilentlyContinue).Count) 個"
  Get-Process $name -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
  Start-Process $Exe '--restore-last-session'
}
Start-Sleep 12
"後: " + (Tabs)
"開き直した本体の指定: " + ((Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq $Exe -and $_.CommandLine -notmatch '--type=' } | Select-Object -First 1).CommandLine)
Get-Process $name -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | ForEach-Object { [void]$_.CloseMainWindow() }
Start-Sleep 5
Get-Process $name -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
