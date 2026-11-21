# 「起動時に前回のページを開く」をオンにした試験用プロファイルで、ふつうに閉じて (窓に「閉じて」を送る) 開き直すとタブが戻るか試す
param([string]$Exe, [string]$Dir, [int]$Restore = 1)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
function Ids { @(Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like "*$Dir*" -and $_.Name -eq (Split-Path $Exe -Leaf) } | Select-Object -ExpandProperty ProcessId) }
function Tabs {
  $names = @()
  foreach ($p in $(if ((Ids).Count) { Get-Process -Id (Ids) -ErrorAction SilentlyContinue })) {
    if ($p.MainWindowHandle -eq 0) { continue }
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)
    $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::TabItem)
    $names += @($root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond) | ForEach-Object { $_.Current.Name } | Where-Object { $_ -notmatch '語$' })
  }
  $names -join ' | '
}
New-Item -ItemType Directory -Force "$Dir\Default" | Out-Null
"{`"session`":{`"restore_on_startup`":$Restore}}" | Set-Content -Encoding utf8NoBOM "$Dir\Default\Preferences"
$args0 = "--user-data-dir=`"$Dir`" --no-first-run --no-default-browser-check"
Start-Process $Exe "$args0 https://example.com/ https://example.org/"
Start-Sleep 12
"前: " + (Tabs)
foreach ($p in $(if ((Ids).Count) { Get-Process -Id (Ids) -ErrorAction SilentlyContinue })) { if ($p.MainWindowHandle -ne 0) { [void]$p.CloseMainWindow() } }
$t0 = Get-Date
while ((Ids).Count -gt 0 -and ((Get-Date) - $t0).TotalSeconds -lt 15) { Start-Sleep -Milliseconds 300 }
"閉じ終わるまで: {0:N1} 秒 / 残り {1} 個" -f ((Get-Date) - $t0).TotalSeconds, (Ids).Count
$(if ((Ids).Count) { Get-Process -Id (Ids) -ErrorAction SilentlyContinue }) | Stop-Process -Force
Start-Sleep 1
Start-Process $Exe $args0
Start-Sleep 10
"後: " + (Tabs)
$(if ((Ids).Count) { Get-Process -Id (Ids) -ErrorAction SilentlyContinue }) | Stop-Process -Force
Start-Sleep 2; Remove-Item -Recurse -Force $Dir -ErrorAction SilentlyContinue
