param(
    [Parameter(Mandatory)][string]$Exe,
    [Parameter(Mandatory)][string]$CoreExe,
    [Parameter(Mandatory)][string]$Fixtures,
    [Parameter(Mandatory)][string]$PresentMon,
    [Parameter(Mandatory)][string]$Out
)
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public class FrameWindow {
    public delegate bool WindowCallback(IntPtr h, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool EnumWindows(WindowCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder name, int count);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int command);
    [StructLayout(LayoutKind.Sequential)]
    public struct MOUSEINPUT {
        public int dx;
        public int dy;
        public uint mouseData;
        public uint dwFlags;
        public uint time;
        public IntPtr dwExtraInfo;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct KEYBDINPUT {
        public ushort wVk;
        public ushort wScan;
        public uint dwFlags;
        public uint time;
        public IntPtr dwExtraInfo;
    }
    [StructLayout(LayoutKind.Explicit)]
    public struct INPUTUNION {
        [FieldOffset(0)] public MOUSEINPUT mi;
        [FieldOffset(0)] public KEYBDINPUT ki;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct INPUT {
        public uint type;
        public INPUTUNION u;
    }
    [DllImport("user32.dll", SetLastError = true)]
    public static extern uint SendInput(uint count, INPUT[] inputs, int size);
    public static IntPtr Find(int pid) {
        IntPtr result = IntPtr.Zero;
        EnumWindows(delegate(IntPtr h, IntPtr ignored) {
            uint owner;
            GetWindowThreadProcessId(h, out owner);
            var name = new StringBuilder(256);
            GetClassName(h, name, name.Capacity);
            if (owner == pid && name.ToString() == "Zed::Window" && IsWindowVisible(h)) result = h;
            return true;
        }, IntPtr.Zero);
        return result;
    }
    public static void Tap(ushort vk) {
        var down = new INPUT { type = 1, u = new INPUTUNION { ki = new KEYBDINPUT { wVk = vk } } };
        var up = new INPUT { type = 1, u = new INPUTUNION { ki = new KEYBDINPUT { wVk = vk, dwFlags = 2 } } };
        if (SendInput(2, new[] { down, up }, Marshal.SizeOf(typeof(INPUT))) != 2) {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }
    }
}
'@
$Exe = (Resolve-Path $Exe).Path
$CoreExe = (Resolve-Path $CoreExe).Path
$Fixtures = (Resolve-Path $Fixtures).Path
$PresentMon = (Resolve-Path $PresentMon).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = (Resolve-Path $Out).Path
$db = Join-Path $Out 'frame.db'
& $CoreExe --db $db import-fixtures $Fixtures
if ($LASTEXITCODE -ne 0) { throw 'Fixture import failed' }
$csv = Join-Path $Out 'present.csv'
$monitorOut = Join-Path $Out 'presentmon.out.txt'
$monitorErr = Join-Path $Out 'presentmon.err.txt'
$app = Start-Process $Exe -PassThru -WindowStyle Normal -ArgumentList @(
    '--db', $db, '--offline', '--screen', 'board', '--board', '15', '--theme', 'graphite'
)
$monitor = $null
try {
    $deadline = [DateTime]::UtcNow.AddSeconds(40)
    $window = [IntPtr]::Zero
    do {
        if ($app.HasExited) { throw 'desktop exited before the window opened' }
        $window = [FrameWindow]::Find($app.Id)
        if ($window -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($window -eq [IntPtr]::Zero) { throw 'no visible desktop window' }
    [FrameWindow]::ShowWindow($window, 3) | Out-Null
    [FrameWindow]::Tap(0x12)
    [FrameWindow]::SetForegroundWindow($window) | Out-Null
    Start-Sleep -Seconds 1
    $foreground = [FrameWindow]::GetForegroundWindow()
    Write-Host "window=$window foreground=$foreground app_pid=$($app.Id)"
    $monitor = Start-Process $PresentMon -PassThru -NoNewWindow -RedirectStandardOutput $monitorOut -RedirectStandardError $monitorErr -ArgumentList @(
        '--process_id', $app.Id,
        '--output_file', $csv,
        '--stop_existing_session',
        '--no_console_stats',
        '--timed', '20',
        '--terminate_after_timed'
    )
    Start-Sleep -Seconds 5
    if ($monitor.HasExited) { throw "PresentMon exited early with code $($monitor.ExitCode)" }
    foreach ($vk in 0x4A, 0x4B, 0x09, 0x28) {
        1..15 | ForEach-Object {
            [FrameWindow]::Tap([uint16]$vk)
            Start-Sleep -Milliseconds 40
        }
    }
    Start-Sleep -Seconds 2
    if (-not $monitor.WaitForExit(40000)) { throw 'PresentMon did not finish its timed capture' }
} finally {
    if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force }
    if ($monitor -and -not $monitor.HasExited) { Stop-Process -Id $monitor.Id -Force }
    foreach ($log in $monitorOut, $monitorErr) {
        if (Test-Path $log) { Write-Host "--- $log"; Get-Content $log -TotalCount 60 }
    }
    Get-ChildItem $Out | ForEach-Object { Write-Host "$($_.Name) $($_.Length)" }
}
if (-not (Test-Path $csv)) { throw 'PresentMon wrote no CSV' }
python (Join-Path $PSScriptRoot 'presentmon-score.py') $csv
if ($LASTEXITCODE -ne 0) { throw 'PresentMon score failed' }
