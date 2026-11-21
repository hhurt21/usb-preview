# Brave のタブの復元を、やり方を変えて試す (試験のあとは閉じる)
param([string]$Exe, [string]$Proc = 'brave', [string]$Profile = "$env:LOCALAPPDATA\BraveSoftware\Brave-Browser\User Data")
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
function Tabs {
  $w = Get-Process $Proc -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
  if (-not $w) { return '(窓なし)' }
  $root = [System.Windows.Automation.AutomationElement]::FromHandle($w.MainWindowHandle)
  $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::TabItem)
  (@($root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond) | ForEach-Object { $_.Current.Name } | Where-Object { $_ -notmatch '語$' })) -join ' | '
}
function Stop-Browser { Get-Process $Proc -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep 2 }
function Sessions { Get-ChildItem "$Profile\Default\Sessions" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 4 | ForEach-Object { "{0} {1:HH:mm:ss} {2}B" -f $_.Name, $_.LastWriteTime, $_.Length } }

Start-Process $Exe '--new-window https://example.com/ https://example.org/'
Start-Sleep 12
"開いた: " + (Tabs)
Stop-Browser
"終わらせた後の Sessions:"; Sessions
Start-Process $Exe '--restore-last-session'
Start-Sleep 10
"A: --restore-last-session → " + (Tabs)
Stop-Browser
Start-Process $Exe
Start-Sleep 10
"B: 何も付けない → " + (Tabs)
Get-Process $Proc -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | ForEach-Object { [void]$_.CloseMainWindow() }
Start-Sleep 4; Stop-Browser
"Brave の起動の設定 (restore_on_startup): " + ((Get-Content "$Profile\Default\Preferences" -Raw | ConvertFrom-Json).session.restore_on_startup)
