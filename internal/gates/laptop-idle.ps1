param(
  [ValidateSet('Idle','DryRun','Wait','Arm','Monitor','Assert','Stop')][string]$Mode = 'Idle',
  [ValidateRange(600,2147483647)][int]$MinIdle = 600
)
$ErrorActionPreference = 'Stop'
$root = 'C:\Users\valen\AppData\Local\Temp\htbehavior'
$lease = Join-Path $root 'away-lease.txt'
$monitorFile = Join-Path $root 'away-monitor.pid'
$interrupted = Join-Path $root 'away-interrupted.txt'
if (-not ('LaptopIdle' -as [type])) {
  Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class LaptopIdle {
  [StructLayout(LayoutKind.Sequential)] struct LASTINPUTINFO { public uint cbSize; public uint dwTime; }
  [DllImport("user32.dll", SetLastError=true)] static extern bool GetLastInputInfo(ref LASTINPUTINFO info);
  public static double Seconds() {
    var info = new LASTINPUTINFO { cbSize = (uint)Marshal.SizeOf(typeof(LASTINPUTINFO)) };
    if (!GetLastInputInfo(ref info)) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
    return unchecked((uint)Environment.TickCount - info.dwTime) / 1000.0;
  }
}
'@
}
function Stop-TestCopies {
  # Match executable paths, never names: the installed app and profile are off limits.
  Get-CimInstance Win32_Process | Where-Object {
    $_.ExecutablePath -match '^C:\\Users\\valen\\AppData\\Local\\Temp\\ht[^\\]*\\'
  } | ForEach-Object {
    $process = Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue
    if ($process -and $process.Path -eq $_.ExecutablePath) {
      Stop-Process -InputObject $process -Force -ErrorAction SilentlyContinue
      "KILLED_TEST_COPY pid=$($_.ProcessId) path=$($_.ExecutablePath)"
    }
  }
}
function Get-MonitorId {
  $monitorId = 0
  if (Test-Path $monitorFile) { $null = [int]::TryParse([IO.File]::ReadAllText($monitorFile), [ref]$monitorId) }
  return $monitorId
}
$idle = [LaptopIdle]::Seconds()
if ($Mode -eq 'Idle') { 'IDLE_SECONDS={0:F3}' -f $idle; return }
if ($Mode -eq 'Stop') { Remove-Item $lease,$monitorFile -ErrorAction SilentlyContinue; Stop-TestCopies; return }
if ($Mode -eq 'DryRun') {
  'IDLE_SECONDS={0:F3}' -f $idle
  if ($idle -lt $MinIdle) { "DRY_RUN_WAIT idle=$idle required=$MinIdle NO_LAUNCH" }
  else { "DRY_RUN_READY idle=$idle required=$MinIdle NO_LAUNCH" }
  return
}
if ($Mode -eq 'Monitor') {
  try {
    while ((Test-Path $lease) -and (Get-MonitorId) -eq $PID) {
      $until = (Get-Item $lease).LastWriteTimeUtc.AddSeconds(115)
      $idle = [LaptopIdle]::Seconds()
      [IO.File]::WriteAllText((Join-Path $root "away-heartbeat-$PID.txt"), 'alive')
      if ($idle -lt 60 -or [DateTime]::UtcNow -gt $until) {
        "INTERRUPTED idle=$idle utc=$([DateTime]::UtcNow.ToString('o'))" | Set-Content $interrupted
        Stop-TestCopies
        Remove-Item $lease -ErrorAction SilentlyContinue
        return
      }
      Start-Sleep -Seconds 1
    }
  } catch {
    Stop-TestCopies
    throw
  }
  return
}
if ($Mode -eq 'Wait') {
  'IDLE_SECONDS={0:F3}' -f $idle
  if ($idle -ge $MinIdle) { 'AWAY_READY'; return }
  if ($idle -lt 60) { Stop-TestCopies }
  "WAIT idle=$idle required=$MinIdle NO_LAUNCH"
  # Bound each helper request so waiting cannot starve other read-only helper calls.
  Start-Sleep -Seconds 20
  return
}
if ($Mode -eq 'Arm') {
  if ($idle -lt $MinIdle) {
    if ($idle -lt 60) { Stop-TestCopies }
    throw "AWAY_REFUSED idle=$idle required=$MinIdle"
  }
  Remove-Item $interrupted -ErrorAction SilentlyContinue
  [IO.File]::WriteAllText($lease, 'away')
  $log = Join-Path $root ("away-monitor-" + [Guid]::NewGuid().ToString('N'))
  # A fresh PID/heartbeat per step cannot mistake an exited monitor's recent heartbeat for a live watchdog.
  $monitor = Start-Process powershell.exe -PassThru -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File',"$root\laptop-idle.ps1",'-Mode','Monitor' -RedirectStandardOutput "$log.txt" -RedirectStandardError "$log.err"
  [IO.File]::WriteAllText($monitorFile, [string]$monitor.Id)
  $heartbeat = Join-Path $root "away-heartbeat-$($monitor.Id).txt"
  for ($i = 0; $i -lt 50 -and -not (Test-Path $heartbeat); $i++) { Start-Sleep -Milliseconds 100 }
}
# Every native input/launch calls Assert as well as the external wrapper.
$monitorId = Get-MonitorId
$heartbeat = Join-Path $root "away-heartbeat-$monitorId.txt"
$monitor = if ($monitorId -gt 0) { Get-CimInstance Win32_Process -Filter "ProcessId=$monitorId" } else { $null }
if (-not $monitor -or $monitor.CommandLine -notlike "*$root\laptop-idle.ps1*" -or $monitor.CommandLine -notmatch '-Mode\s+Monitor' -or -not (Test-Path $heartbeat) -or ([DateTime]::UtcNow - (Get-Item $heartbeat).LastWriteTimeUtc).TotalSeconds -gt 5) {
  Stop-TestCopies
  throw 'AWAY_REFUSED watchdog process/heartbeat missing or stale'
}
$idle = [LaptopIdle]::Seconds()
if ($idle -lt $MinIdle) {
  if ($idle -lt 60) { Stop-TestCopies }
  throw "AWAY_REFUSED idle=$idle required=$MinIdle"
}
if (-not (Test-Path $lease) -or [DateTime]::UtcNow -gt (Get-Item $lease).LastWriteTimeUtc.AddSeconds(115)) {
  Stop-TestCopies
  throw 'AWAY_REFUSED missing or expired monitor lease'
}
if (Test-Path $interrupted) { Stop-TestCopies; throw 'AWAY_REFUSED interrupted session' }
'AWAY_STEP_ALLOWED idle={0:F3} monitor={1}' -f $idle,$monitorId
