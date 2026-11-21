# ディスクに本当に書かれた量を、アプリごと・ファイルごとに記録する (Windows の ETW。リソース モニターと同じ仕組み)。
# 管理者で動かす。記録するだけで、何も止めたり変えたりしない。
param([int]$Seconds = 120, [string]$Dir = "$env:TEMP\usbwatch_trace")
New-Item -ItemType Directory -Force $Dir | Out-Null
$etl = Join-Path $Dir 'disk.etl'; $rpt = Join-Path $Dir 'report.xml'; $dump = Join-Path $Dir 'dump.xml'; $log = Join-Path $Dir 'run.log'
Remove-Item $etl, $rpt, $dump -ErrorAction SilentlyContinue
"start $(Get-Date -f HH:mm:ss)" | Set-Content $log
logman stop "NT Kernel Logger" -ets 2>&1 | Out-Null
logman start "NT Kernel Logger" -p "Windows Kernel Trace" "(process,disk,file)" -o $etl -ets 2>&1 | Add-Content $log
Start-Sleep $Seconds
logman stop "NT Kernel Logger" -ets 2>&1 | Add-Content $log
"stopped $(Get-Date -f HH:mm:ss)" | Add-Content $log
tracerpt $etl -o $dump -of XML -report $rpt -f XML -y 2>&1 | Select-Object -Last 3 | Add-Content $log
Remove-Item $dump -ErrorAction SilentlyContinue
"done $(Get-Date -f HH:mm:ss)" | Add-Content $log
