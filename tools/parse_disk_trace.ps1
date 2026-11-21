# trace_disk.ps1 で取った記録 (disk.etl) から、ディスクへの書き込みをファイルごと・アプリごとに数える。読むだけ。
param([string]$Dir = "$env:TEMP\usbwatch_trace", [int]$Disk = 0, [int]$Top = 25)
$etl = Join-Path $Dir 'disk.etl'; $dump = Join-Path $Dir 'dump.xml'
if (-not (Test-Path $dump)) { tracerpt $etl -o $dump -of XML -y | Out-Null }

$DISKIO = '{3d6fa8d4-fe05-11d0-9dda-00c04fd7ba7c}'
$FILEIO = '{90cbdc39-4a3e-11d1-84f4-0000f80464e3}'
$PROC = '{3d6fa8d0-fe05-11d0-9dda-00c04fd7ba7c}'
$names = @{}      # FileObject → ファイル名
$procs = @{}      # PID → アプリ名
$writes = New-Object System.Collections.Generic.List[object]

$settings = New-Object System.Xml.XmlReaderSettings
$settings.IgnoreWhitespace = $true
$r = [System.Xml.XmlReader]::Create($dump, $settings)
while ($r.ReadToFollowing('Event')) {
  $ev = [xml]$r.ReadOuterXml()
  $sys = $ev.Event.System
  $eg = $ev.Event.ExtendedTracingInfo.EventGuid; $guid = ([string]$(if ($eg) { $eg } else { $sys.Provider.Guid })).ToLower()
  $op = [int]$sys.Opcode
  $data = @{}
  foreach ($d in $ev.Event.EventData.Data) { $data[$d.Name] = ([string]$d.'#text').Trim() }
  if ($guid -eq $PROC -and $data.ContainsKey('ImageFileName')) {
    $procs[[string][Convert]::ToInt64($data['ProcessId'], 16)] = $data['ImageFileName']
  } elseif ($guid -eq $FILEIO -and $data.ContainsKey('FileName') -and $data.ContainsKey('FileObject')) {
    $names[$data['FileObject']] = $data['FileName']
  } elseif ($guid -eq $DISKIO -and $op -eq 11) {
    if ([int]$data['DiskNumber'] -ne $Disk) { continue }
    $writes.Add([pscustomobject]@{ size = [int64]$data['TransferSize']; fo = $data['FileObject']; pid = [string]$sys.Execution.ProcessID })
  }
}
$r.Close()

$total = ($writes | Measure-Object size -Sum).Sum
"ディスク $Disk への書き込み: 合計 {0:N1} MB（{1} 回）" -f ($total / 1MB), $writes.Count
""
"--- ファイルごと（多い順） ---"
$writes | Group-Object fo | ForEach-Object {
  $n = $names[$_.Name]; if (-not $n) { $n = "(名前の分からないファイル・Windows の管理領域など)" }
  [pscustomobject]@{ file = $n; mb = ($_.Group | Measure-Object size -Sum).Sum / 1MB }
} | Group-Object file | ForEach-Object { [pscustomobject]@{ file = $_.Name; mb = ($_.Group | Measure-Object mb -Sum).Sum } } |
  Sort-Object mb -Descending | Select-Object -First $Top | ForEach-Object {
  "{0,8:N1} MB  {1}" -f $_.mb, ($_.file -replace '\\Device\\HarddiskVolume\d+', '' -replace [regex]::Escape($env:USERPROFILE.Substring(2)), '%USERPROFILE%')
}
""
"--- アプリごと（ディスクに書いた瞬間のアプリ。Windows がまとめて書く分は System に入る） ---"
$writes | Group-Object pid | ForEach-Object { [pscustomobject]@{ app = ($procs[$_.Name] ?? "PID $($_.Name)"); mb = ($_.Group | Measure-Object size -Sum).Sum / 1MB } } |
  Group-Object app | ForEach-Object { [pscustomobject]@{ app = $_.Name; mb = ($_.Group | Measure-Object mb -Sum).Sum } } |
  Sort-Object mb -Descending | Select-Object -First 12 | ForEach-Object { "{0,8:N1} MB  {1}" -f $_.mb, $_.app }

""
"--- 場所ごと（アプリのフォルダーでまとめた） ---"
function Bucket($f) {
  if (-not $f -or $f -like '(名前の*') { return '(名前の分からない書き込み：NTFS の管理情報・ページファイルなど)' }
  $rules = @(
    @('\\Google\\Chrome\\User Data\\[^\\]+\\Cache\\', 'Chrome：ページのキャッシュ'),
    @('\\Google\\Chrome\\User Data\\[^\\]+\\(IndexedDB|Local Extension Settings|Extension State)', 'Chrome：拡張機能のデータ'),
    @('\\Google\\Chrome\\User Data\\[^\\]+\\Code Cache', 'Chrome：スクリプトのキャッシュ'),
    @('\\Google\\Chrome\\', 'Chrome：その他（履歴・設定など）'),
    @('\\discord', 'Discord'),
    @('\\Mozilla\\Firefox', 'Firefox'),
    @('\\Vivaldi\\', 'Vivaldi'),
    @('\\usb-watch\\', 'USB 見張り（このアプリの画面の部品）'),
    @('\\Code\\|\\\.vscode', 'VS Code'),
    @('\\Claude|\\\.claude', 'Claude Code'),
    @('\\Steam\\|\\steamapps', 'Steam'),
    @('\\AppData\\Local\\Temp\\|\\Windows\\Temp\\', '一時ファイル（Temp）'),
    @('\\Windows\\Prefetch|\\System32\\|\\Windows\\', 'Windows'),
    @('\\ProgramData\\Microsoft\\', 'Windows（Defender・検索など）')
  )
  foreach ($r in $rules) { if ($f -match $r[0]) { return $r[1] } }
  $s = $f -replace '\\Device\\HarddiskVolume\d+', '' -replace '^\\Users\\[^\\]+', '%USERPROFILE%'
  return (($s -split '\\')[0..3] -join '\')
}
$writes | ForEach-Object { [pscustomobject]@{ b = (Bucket $names[$_.fo]); size = $_.size } } | Group-Object b |
  ForEach-Object { [pscustomobject]@{ b = $_.Name; mb = ($_.Group | Measure-Object size -Sum).Sum / 1MB } } |
  Sort-Object mb -Descending | ForEach-Object { "{0,8:N1} MB  {1,5:N1}%  {2}" -f $_.mb, ($_.mb * 1MB / $total * 100), $_.b }