$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class RamWindow {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h, int command);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint message, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
    public delegate bool WindowCallback(IntPtr h, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool EnumWindows(WindowCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder name, int count);
    public static IntPtr FindWindow(int pid) {
        IntPtr result = IntPtr.Zero;
        long largest = 0;
        EnumWindows(delegate(IntPtr h, IntPtr ignored) {
            uint owner; Rect r;
            GetWindowThreadProcessId(h, out owner);
            var name = new System.Text.StringBuilder(256);
            GetClassName(h, name, name.Capacity);
            if (owner == pid && name.ToString() == "Zed::Window" && IsWindowVisible(h) && GetClientRect(h, out r)) {
                long area = (long)r.Right * r.Bottom;
                if (area > largest) { largest = area; result = h; }
            }
            return true;
        }, IntPtr.Zero);
        return result;
    }
    public struct Rect { public int Left, Top, Right, Bottom; }
}
'@
[RamWindow]::SetProcessDPIAware() | Out-Null
$w = 'C:\Users\valen\AppData\Local\Temp\htram'
$display = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
if ($display.Width -ne 1920 -or $display.Height -ne 1080) { throw "Expected 1920x1080, got $display" }
function Save-Capture([IntPtr]$h, [string]$path) {
    # Normalize hover without moving the owner's physical cursor.
    [RamWindow]::PostMessage($h, 0x200, [IntPtr]::Zero, [IntPtr](-1)) | Out-Null
    [RamWindow]::PostMessage($h, 0x2A3, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    Start-Sleep -Milliseconds 200
    $r = New-Object RamWindow+Rect
    if (-not [RamWindow]::GetClientRect($h, [ref]$r)) { throw 'Capture GetClientRect failed' }
    $bmp = New-Object System.Drawing.Bitmap ($r.Right), ($r.Bottom)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $dc = $g.GetHdc()
    try { if (-not [RamWindow]::PrintWindow($h, $dc, 3)) { throw 'PrintWindow failed' } }
    finally { $g.ReleaseHdc($dc) }
    $bmp.Save($path)
    $g.Dispose(); $bmp.Dispose()
}
$rows = @()
foreach ($round in 1..3) {
    foreach ($screen in 'inbox', 'board', 'task') {
        $db = "$w\run-$screen-$round.db"
        Remove-Item $db, "$db-wal", "$db-shm" -Force -ErrorAction SilentlyContinue
        Copy-Item "$w\parity.db" $db
        $dump = "$w\$screen-$round.json"
        Remove-Item $dump -Force -ErrorAction SilentlyContinue
        $p = Start-Process "$w\ht-ram.exe" -WindowStyle Maximized -PassThru -ArgumentList '--db', $db, '--offline', '--board', '7283', '--task', '60253', '--theme', 'graphite', '--screen', $screen, '--dump-layout', $dump
        try {
            $deadline = [DateTime]::UtcNow.AddSeconds(10)
            do {
                $h = [RamWindow]::FindWindow($p.Id)
                if ($h -ne [IntPtr]::Zero) { break }
                if ($p.HasExited) { throw "$screen exited before opening a window" }
                Start-Sleep -Milliseconds 100
            } while ([DateTime]::UtcNow -lt $deadline)
            if ($h -eq [IntPtr]::Zero) { throw "$screen has no visible window" }
            [RamWindow]::ShowWindowAsync($h, 3) | Out-Null
            Start-Sleep -Seconds 8
            $p.Refresh()
            if ($p.HasExited) { throw "$screen run $round exited" }
            $h = [RamWindow]::FindWindow($p.Id)
            if (-not [RamWindow]::IsZoomed($h)) { throw "$screen is not maximized (pid=$($p.Id) hwnd=$h)" }
            $dpi = [RamWindow]::GetDpiForWindow($h)
            if ($dpi -ne 96) { throw "Expected 100% scaling, got DPI $dpi" }
            if (-not (Test-Path $dump)) { throw "No layout for $screen" }
            $rect = New-Object RamWindow+Rect
            if (-not [RamWindow]::GetClientRect($h, [ref]$rect)) { throw 'GetClientRect failed' }
            $rows += [pscustomobject]@{
                Screen = $screen; Run = $round; Pid = $p.Id
                WorkingSet64 = $p.WorkingSet64; PrivateMemorySize64 = $p.PrivateMemorySize64
                ClientWidth = $rect.Right; ClientHeight = $rect.Bottom; Dpi = $dpi; Maximized = $true
            }
            if ($round -eq 3) {
                Save-Capture $h "$w\$screen.png"
                Copy-Item $dump "$w\$screen.json" -Force
                [RamWindow]::ShowWindowAsync($h, 9) | Out-Null
                Start-Sleep -Milliseconds 300
                if (-not [RamWindow]::SetWindowPos($h, [IntPtr]::Zero, 100, 100, 1280, 800, 0x14)) { throw 'Resize failed' }
                Start-Sleep -Milliseconds 500
                $small = New-Object RamWindow+Rect
                [RamWindow]::GetClientRect($h, [ref]$small) | Out-Null
                if ($small.Right -ge $rect.Right -or [RamWindow]::IsZoomed($h)) { throw 'Window did not restore/resize' }
                [RamWindow]::ShowWindowAsync($h, 6) | Out-Null
                Start-Sleep -Milliseconds 300
                [RamWindow]::ShowWindowAsync($h, 3) | Out-Null
                Start-Sleep -Seconds 1
                $restored = New-Object RamWindow+Rect
                [RamWindow]::GetClientRect($h, [ref]$restored) | Out-Null
                if (-not [RamWindow]::IsZoomed($h) -or $restored.Right -ne $rect.Right -or $restored.Bottom -ne $rect.Bottom) { throw 'Maximize recovery failed' }
                Save-Capture $h "$w\$screen-recovered.png"
            }
        } finally {
            if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force; $p.WaitForExit() }
        }
    }
}
$medians = foreach ($screen in 'inbox', 'board', 'task') {
    $samples = @($rows | Where-Object Screen -eq $screen)
    [pscustomobject]@{
        Screen = $screen
        WorkingSet64 = @($samples.WorkingSet64 | Sort-Object)[1]
        PrivateMemorySize64 = @($samples.PrivateMemorySize64 | Sort-Object)[1]
    }
}
$result = [pscustomobject]@{
    ExeSHA256 = (Get-FileHash "$w\ht-ram.exe").Hash
    DatabaseSHA256 = (Get-FileHash "$w\parity.db").Hash
    DisplayWidth = $display.Width; DisplayHeight = $display.Height
    Samples = $rows; Medians = @($medians); ResizeMinimizeRestore = $true
}
$result | ConvertTo-Json -Depth 5 | Set-Content "$w\ram.json" -Encoding UTF8
$result | ConvertTo-Json -Depth 5
