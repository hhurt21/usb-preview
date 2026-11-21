# ブラウザーの「開き直す」で、開いていたタブが戻るかを試す。試験のあとはブラウザーを閉じる。
# 使い方: pwsh -File tools/try_restart.ps1 -Exe <ブラウザーの exe> -Id brave -Proc brave
param([string]$Exe, [string]$Id, [string]$Proc)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
function Tabs {
  $w = Get-Process $Proc -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
  if (-not $w) { return @('(窓なし)') }
  $root = [System.Windows.Automation.AutomationElement]::FromHandle($w.MainWindowHandle)
  $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::TabItem)
  @($root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond) | ForEach-Object { $_.Current.Name })
}
Start-Process $Exe '--new-window https://example.com/ https://example.org/'
Start-Sleep 8
"前のタブ: " + ((Tabs) -join ' | ')
$r = node "$PSScriptRoot\cdp.mjs" "window.__TAURI__.core.invoke('restart_browser',{id:'$Id'}).catch(e=>'ERR '+e)"
"開き直す: $r"
Start-Sleep 10
"後のタブ: " + ((Tabs) -join ' | ')
# 片づけ: 試験で開いたので閉じる
Get-Process $Proc -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | ForEach-Object { [void]$_.CloseMainWindow() }
Start-Sleep 5
"片づけ後に残っている $Proc : $(@(Get-Process $Proc -ErrorAction SilentlyContinue).Count) 個"
