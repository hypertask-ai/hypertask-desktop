param(
  [ValidateSet('Idle','DryRun','Wait','Monitor','Assert','Stop')][string]$Mode = 'Idle',
  [ValidateRange(600,2147483647)][int]$MinIdle = 600
)
$ErrorActionPreference = 'Stop'
$root = 'C:\Users\valen\AppData\Local\Temp\htbehavior'
$lease = Join-Path $root 'away-lease.txt'
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
$idle = [LaptopIdle]::Seconds()
if ($Mode -eq 'Idle') { 'IDLE_SECONDS={0:F3}' -f $idle; return }
if ($Mode -eq 'Stop') { Remove-Item $lease -ErrorAction SilentlyContinue; Stop-TestCopies; return }
if ($Mode -eq 'DryRun') {
  'IDLE_SECONDS={0:F3}' -f $idle
  if ($idle -lt $MinIdle) { "DRY_RUN_WAIT idle=$idle required=$MinIdle NO_LAUNCH" }
  else { "DRY_RUN_READY idle=$idle required=$MinIdle NO_LAUNCH" }
  return
}
if ($Mode -eq 'Monitor') {
  try {
    while (Test-Path $lease) {
      $until = (Get-Item $lease).LastWriteTimeUtc.AddSeconds(115)
      $idle = [LaptopIdle]::Seconds()
      [IO.File]::WriteAllText((Join-Path $root 'away-heartbeat.txt'), [DateTime]::UtcNow.ToString('o'))
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
  $deadline = [DateTime]::UtcNow.AddSeconds(85)
  do {
    $idle = [LaptopIdle]::Seconds()
    'IDLE_SECONDS={0:F3}' -f $idle
    if ($idle -ge $MinIdle) { 'AWAY_READY'; return }
    if ($idle -lt 60) { Stop-TestCopies }
    "WAIT idle=$idle required=$MinIdle NO_LAUNCH"
    Start-Sleep -Seconds 20
  } while ([DateTime]::UtcNow -lt $deadline)
  return
}
# Every native input/launch calls Assert as well as the external wrapper.
$heartbeat = Join-Path $root 'away-heartbeat.txt'
if (-not (Test-Path $heartbeat) -or ([DateTime]::UtcNow - (Get-Item $heartbeat).LastWriteTimeUtc).TotalSeconds -gt 5) {
  Stop-TestCopies
  throw 'AWAY_REFUSED watchdog heartbeat missing or stale'
}
if ($idle -lt $MinIdle) {
  if ($idle -lt 60) { Stop-TestCopies }
  throw "AWAY_REFUSED idle=$idle required=$MinIdle"
}
if (-not (Test-Path $lease) -or [DateTime]::UtcNow -gt (Get-Item $lease).LastWriteTimeUtc.AddSeconds(115)) {
  Stop-TestCopies
  throw 'AWAY_REFUSED missing or expired monitor lease'
}
if (Test-Path $interrupted) { Stop-TestCopies; throw 'AWAY_REFUSED interrupted session' }
'AWAY_STEP_ALLOWED idle={0:F3}' -f $idle
