//! PC のスペック。Windows の管理情報 (CIM) とレジストリから集める。
//! PC の名前・ユーザー名・シリアル番号・MAC アドレス・IP アドレスなど、
//! 人や機械を特定できる情報は集めない (画面に出したり、コピーして人に見せたりするため)。

use std::os::windows::process::CommandExt;

const SCRIPT: &str = r#"
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$ErrorActionPreference = 'SilentlyContinue'
function T($s) { if ($null -eq $s) { $null } else { ([string]$s).Trim() } }

$os = Get-CimInstance Win32_OperatingSystem
$cv = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
$cpu = @(Get-CimInstance Win32_Processor)
$mem = @(Get-CimInstance Win32_PhysicalMemory)
$arr = @(Get-CimInstance Win32_PhysicalMemoryArray)
$bb = Get-CimInstance Win32_BaseBoard
$bios = Get-CimInstance Win32_BIOS
$cs = Get-CimInstance Win32_ComputerSystem

# GPU: VRAM は Win32_VideoController だと 4GB で頭打ちになるので、レジストリの qwMemorySize を使う
$vram = @{}
Get-ChildItem 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}' |
  Where-Object { $_.PSChildName -match '^\d{4}$' } | ForEach-Object {
    $p = Get-ItemProperty $_.PSPath
    if ($p.'HardwareInformation.qwMemorySize') { $vram[[string]$p.DriverDesc] = [int64]$p.'HardwareInformation.qwMemorySize' }
  }
$gpu = @(Get-CimInstance Win32_VideoController | ForEach-Object {
  [pscustomobject]@{
    name = T $_.Name; driver = T $_.DriverVersion
    driver_date = if ($_.DriverDate) { $_.DriverDate.ToString('yyyy-MM-dd') } else { $null }
    vram = if ($vram.ContainsKey([string]$_.Name)) { $vram[[string]$_.Name] } else { [int64]$_.AdapterRAM }
    pnp = T $_.PNPDeviceID
  } })

$disks = @(Get-PhysicalDisk | ForEach-Object {
  [pscustomobject]@{ name = T $_.FriendlyName; media = [string]$_.MediaType; bus = [string]$_.BusType; size = [int64]$_.Size; health = [string]$_.HealthStatus } })
$vols = @(Get-Volume | Where-Object { $_.DriveLetter } | Sort-Object DriveLetter | ForEach-Object {
  [pscustomobject]@{ letter = [string]$_.DriveLetter; fs = [string]$_.FileSystem; size = [int64]$_.Size; free = [int64]$_.SizeRemaining; kind = [string]$_.DriveType } })
$nets = @(Get-NetAdapter -Physical | ForEach-Object {
  [pscustomobject]@{ name = T $_.InterfaceDescription; status = [string]$_.Status; speed = [string]$_.LinkSpeed; kind = [string]$_.MediaType } })

[pscustomobject]@{
  os = [pscustomobject]@{
    name = T $os.Caption; version = T $cv.DisplayVersion; build = "$($os.BuildNumber).$($cv.UBR)"
    arch = T $os.OSArchitecture
    boot = if ($os.LastBootUpTime) { $os.LastBootUpTime.ToString('o') } else { $null }
  }
  cpu = @($cpu | ForEach-Object { [pscustomobject]@{
    name = T $_.Name; cores = [int]$_.NumberOfCores; threads = [int]$_.NumberOfLogicalProcessors
    mhz = [int]$_.MaxClockSpeed; l2 = [int]$_.L2CacheSize; l3 = [int]$_.L3CacheSize; socket = T $_.SocketDesignation } })
  memory = [pscustomobject]@{
    total = [int64]$cs.TotalPhysicalMemory
    slots = [int](($arr | Measure-Object -Property MemoryDevices -Sum).Sum)
    modules = @($mem | ForEach-Object { [pscustomobject]@{
      slot = T $_.DeviceLocator; size = [int64]$_.Capacity; speed = [int]$_.Speed; configured = [int]$_.ConfiguredClockSpeed
      maker = T $_.Manufacturer; part = T $_.PartNumber; kind = [int]$_.SMBIOSMemoryType } })
  }
  board = [pscustomobject]@{ maker = T $bb.Manufacturer; product = T $bb.Product }
  bios = [pscustomobject]@{ maker = T $bios.Manufacturer; version = T $bios.SMBIOSBIOSVersion
    date = if ($bios.ReleaseDate) { $bios.ReleaseDate.ToString('yyyy-MM-dd') } else { $null } }
  gpu = $gpu; disks = $disks; volumes = $vols; nets = $nets
} | ConvertTo-Json -Depth 6 -Compress
"#;

pub fn get() -> Result<serde_json::Value, String> {
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| e.to_string())?;
    let s = String::from_utf8_lossy(&out.stdout);
    let s = s.trim().trim_start_matches('\u{feff}');
    if s.is_empty() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    serde_json::from_str(s).map_err(|e| format!("読めませんでした: {e}"))
}
