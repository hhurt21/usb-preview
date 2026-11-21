# ふだんのプロファイルで、再起動マネージャーを使って「保存してから閉じて、開き直す」とタブが戻るか試す。
# 試験のあとはブラウザーを閉じる (もともと開いていなかったブラウザーで使う)。
param([string]$Exe)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
. "$PSScriptRoot\rm_helper.ps1"
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
Start-Sleep 12
"前: " + (Tabs)
"本体: $((Main) -join ',')"
"再起動マネージャー: " + [Rm]::Run([int[]](Main))
Start-Sleep 12
"後: " + (Tabs)
"開き直した本体の指定: " + ((Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq $Exe -and $_.CommandLine -notmatch '--type=' } | Select-Object -First 1).CommandLine)
# 片づけ
Get-Process $name -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | ForEach-Object { [void]$_.CloseMainWindow() }
Start-Sleep 5
Get-Process $name -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
