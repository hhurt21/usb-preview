# C: への書き込みを見張り、多いときに「その瞬間たくさん書いていたアプリ」を記録する。読むだけ。
# 使い方: pwsh -File tools/watch_writes.ps1 -Minutes 10 -Threshold 2 -Out <ログ>
param([int]$Minutes = 10, [double]$Threshold = 2, [string]$Out = "$env:TEMP\usbwatch_writes.log")
$end = (Get-Date).AddMinutes($Minutes)
$total = 0.0; $n = 0
"開始 $(Get-Date -f HH:mm:ss)  しきい値 ${Threshold}MB/秒" | Set-Content $Out
while ((Get-Date) -lt $end) {
  $s = Get-Counter -Counter '\PhysicalDisk(0 c:)\Disk Write Bytes/sec', '\Process(*)\IO Write Bytes/sec' -SampleInterval 2 -MaxSamples 1 -ErrorAction SilentlyContinue
  if (-not $s) { continue }
  $disk = ($s.CounterSamples | Where-Object Path -like '*physicaldisk*').CookedValue / 1MB
  $total += $disk * 2; $n++
  if ($disk -ge $Threshold) {
    $top = $s.CounterSamples | Where-Object { $_.Path -like '*\process(*' -and $_.InstanceName -notin '_total', 'idle' } |
      Sort-Object CookedValue -Descending | Select-Object -First 4 | ForEach-Object { '{0} {1:N1}' -f $_.InstanceName, ($_.CookedValue / 1MB) }
    "{0}  C: {1,6:N1} MB/秒  ← {2}" -f (Get-Date -f HH:mm:ss), $disk, ($top -join ' / ') | Add-Content $Out
  }
}
"終了 $(Get-Date -f HH:mm:ss)  合計 {0:N0} MB（平均 {1:N2} MB/秒、1日に直すと約 {2:N0} GB）" -f $total, ($total / [math]::Max(1, $n * 2)), ($total / [math]::Max(1, $n * 2) * 86400 / 1024) | Add-Content $Out
