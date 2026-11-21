# ブラウザーに「〇〇://restart」を開かせて、ブラウザー自身に開き直してもらうとタブが戻るか試す (試験のあとは閉じる)
param([string]$Exe, [string]$Url)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$name = [IO.Path]::GetFileNameWithoutExtension($Exe)
function Main { @(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq $Exe -and $_.CommandLine -notmatch '--type=' }) }
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
Start-Sleep 12
$before = (Main | Select-Object -First 1)
"前: " + (Tabs) + "（本体 $($before.ProcessId)）"
Start-Process $Exe $Url
Start-Sleep 15
$after = (Main | Select-Object -First 1)
"後: " + (Tabs) + "（本体 $($after.ProcessId)・入れ替わった: $($before.ProcessId -ne $after.ProcessId)）"
Get-Process $name -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | ForEach-Object { [void]$_.CloseMainWindow() }
Start-Sleep 5
Get-Process $name -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
