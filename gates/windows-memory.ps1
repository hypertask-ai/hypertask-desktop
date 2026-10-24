param(
    [Parameter(Mandatory)][string]$Exe,
    [Parameter(Mandatory)][string]$CoreExe,
    [Parameter(Mandatory)][string]$Fixtures,
    [Parameter(Mandatory)][string]$Revision,
    [Parameter(Mandatory)][string]$Out,
    [string]$Budget
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class MemoryWindow {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h, int command);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool EnumDisplaySettings(string device, int mode, byte[] settings);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int ChangeDisplaySettings(byte[] settings, uint flags);
    public delegate bool WindowCallback(IntPtr h, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool EnumWindows(WindowCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder name, int count);
    public static IntPtr FindWindow(int pid) {
        IntPtr result = IntPtr.Zero;
        EnumWindows(delegate(IntPtr h, IntPtr ignored) {
            uint owner;
            GetWindowThreadProcessId(h, out owner);
            var name = new System.Text.StringBuilder(256);
            GetClassName(h, name, name.Capacity);
            if (owner == pid && name.ToString() == "Zed::Window" && IsWindowVisible(h)) result = h;
            return true;
        }, IntPtr.Zero);
        return result;
    }
    public static int RequestFullHD() {
        // DEVMODEW's fixed layout; preserve all driver fields except width/height.
        byte[] mode = new byte[220];
        BitConverter.GetBytes((short)220).CopyTo(mode, 68);
        if (!EnumDisplaySettings(null, -1, mode)) return -1;
        BitConverter.GetBytes(0x180000).CopyTo(mode, 72);
        BitConverter.GetBytes(1920).CopyTo(mode, 172);
        BitConverter.GetBytes(1080).CopyTo(mode, 176);
        int test = ChangeDisplaySettings(mode, 2);
        return test == 0 ? ChangeDisplaySettings(mode, 0) : test;
    }
    public struct Rect { public int Left, Top, Right, Bottom; }
}
'@
[MemoryWindow]::SetProcessDPIAware() | Out-Null
$Exe = (Resolve-Path $Exe).Path
$CoreExe = (Resolve-Path $CoreExe).Path
$Fixtures = (Resolve-Path $Fixtures).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = (Resolve-Path $Out).Path
$displayResult = [MemoryWindow]::RequestFullHD()
$display = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
Write-Host "Requested 1920x1080: result=$displayResult; actual=$($display.Width)x$($display.Height)"
$seed = Join-Path $Out 'fixtures.db'
& $CoreExe --db $seed import-fixtures $Fixtures
if ($LASTEXITCODE -ne 0) { throw 'Fixture import failed' }
$rows = @()
foreach ($round in 1..3) {
    foreach ($screen in 'inbox', 'board', 'task') {
        $db = Join-Path $Out "run-$screen-$round.db"
        Copy-Item $seed $db
        $dump = Join-Path $Out "$screen-$round.json"
        $p = Start-Process $Exe -WindowStyle Maximized -PassThru -ArgumentList @(
            '--db', "`"$db`"", '--offline', '--theme', 'graphite', '--screen', $screen,
            '--board', '15', '--task', '64897', '--dump-layout', "`"$dump`""
        )
        try {
            $deadline = [DateTime]::UtcNow.AddSeconds(30)
            do {
                if ($p.HasExited) { throw "$screen exited before opening a window" }
                $h = [MemoryWindow]::FindWindow($p.Id)
                if ($h -ne [IntPtr]::Zero) { break }
                Start-Sleep -Milliseconds 100
            } while ([DateTime]::UtcNow -lt $deadline)
            if ($h -eq [IntPtr]::Zero) { throw "$screen has no visible GPUI window" }
            [MemoryWindow]::ShowWindowAsync($h, 3) | Out-Null
            Start-Sleep -Seconds 8
            $p.Refresh()
            if ($p.HasExited) { throw "$screen run $round exited" }
            if (-not [MemoryWindow]::IsZoomed($h)) { throw "$screen is not maximized" }
            if (-not (Test-Path $dump)) { throw "No rendered layout for $screen" }
            $rect = New-Object MemoryWindow+Rect
            if (-not [MemoryWindow]::GetClientRect($h, [ref]$rect)) { throw 'GetClientRect failed' }
            $rows += [pscustomobject]@{
                Screen = $screen; Run = $round; Pid = $p.Id
                WorkingSet64 = $p.WorkingSet64; PrivateMemorySize64 = $p.PrivateMemorySize64
                ClientWidth = $rect.Right; ClientHeight = $rect.Bottom
                Dpi = [MemoryWindow]::GetDpiForWindow($h); Maximized = $true
            }
            if ($round -eq 3) {
                $bmp = New-Object System.Drawing.Bitmap ($rect.Right), ($rect.Bottom)
                $g = [System.Drawing.Graphics]::FromImage($bmp)
                $dc = $g.GetHdc()
                try {
                    if (-not [MemoryWindow]::PrintWindow($h, $dc, 3)) { throw 'PrintWindow failed' }
                } finally { $g.ReleaseHdc($dc) }
                $bmp.Save((Join-Path $Out "$screen.png"))
                $g.Dispose(); $bmp.Dispose()
            }
        } finally {
            if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force; $p.WaitForExit() }
        }
    }
}
$medians = @(foreach ($screen in 'inbox', 'board', 'task') {
    $samples = @($rows | Where-Object Screen -eq $screen)
    [pscustomobject]@{
        Screen = $screen
        WorkingSet64 = @($samples.WorkingSet64 | Sort-Object)[1]
        PrivateMemorySize64 = @($samples.PrivateMemorySize64 | Sort-Object)[1]
    }
})
$result = [pscustomobject]@{
    Revision = $Revision; RunnerImage = $env:ImageVersion
    ExeSHA256 = (Get-FileHash $Exe).Hash; DatabaseSHA256 = (Get-FileHash $seed).Hash
    DisplayRequestResult = $displayResult; DisplayWidth = $display.Width; DisplayHeight = $display.Height
    WaitSeconds = 8; Samples = $rows; Medians = $medians
}
$result | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $Out 'memory.json') -Encoding UTF8
$result | ConvertTo-Json -Depth 5 | Write-Host
$table = @('| Screen | Working set MB | Private MB |', '| --- | ---: | ---: |')
foreach ($m in $medians) {
    $table += '| {0} | {1:F3} | {2:F3} |' -f $m.Screen, ($m.WorkingSet64 / 1e6), ($m.PrivateMemorySize64 / 1e6)
}
$table | Write-Host
if ($env:GITHUB_STEP_SUMMARY) {
    @("## Memory: $Revision", "Display $($display.Width)x$($display.Height), maximized, 8 s, median of three. MB = 1,000,000 bytes.") + $table | Add-Content $env:GITHUB_STEP_SUMMARY
}
if ($Budget) {
    $limit = Get-Content $Budget -Raw | ConvertFrom-Json
    $inbox = ($medians | Where-Object Screen -eq 'inbox').WorkingSet64
    if ($inbox -gt $limit.InboxWorkingSet64) {
        throw "Inbox working set $inbox exceeds frozen baseline + 5% ($($limit.InboxWorkingSet64))"
    }
    Write-Host "MEMORY_GATE_PASS inbox=$inbox threshold=$($limit.InboxWorkingSet64)"
}
