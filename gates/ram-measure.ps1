$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class RamWindow {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
    public struct Rect { public int Left, Top, Right, Bottom; }
}
'@
[RamWindow]::SetProcessDPIAware() | Out-Null
$w = 'C:\Users\valen\AppData\Local\Temp\htram'
$display = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
if ($display.Width -ne 1920 -or $display.Height -ne 1080) { throw "Expected 1920x1080, got $display" }
$rows = @()
foreach ($round in 1..3) {
    foreach ($screen in 'inbox', 'board', 'task') {
        $db = "$w\run-$screen-$round.db"
        Remove-Item $db, "$db-wal", "$db-shm" -Force -ErrorAction SilentlyContinue
        Copy-Item "$w\parity.db" $db
        $dump = "$w\$screen-$round.json"
        Remove-Item $dump -Force -ErrorAction SilentlyContinue
        $p = Start-Process "$w\ht-ram.exe" -PassThru -ArgumentList '--db', $db, '--offline', '--board', '7283', '--task', '60253', '--theme', 'graphite', '--screen', $screen, '--dump-layout', $dump
        try {
            Start-Sleep -Seconds 8
            $p.Refresh()
            if ($p.HasExited) { throw "$screen run $round exited" }
            $h = $p.MainWindowHandle
            if (-not [RamWindow]::IsZoomed($h)) { throw "$screen is not maximized" }
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
                $bmp = New-Object System.Drawing.Bitmap ($rect.Right), ($rect.Bottom)
                $g = [System.Drawing.Graphics]::FromImage($bmp)
                $dc = $g.GetHdc()
                try { if (-not [RamWindow]::PrintWindow($h, $dc, 3)) { throw 'PrintWindow failed' } }
                finally { $g.ReleaseHdc($dc) }
                $bmp.Save("$w\$screen.png")
                $g.Dispose(); $bmp.Dispose()
                Copy-Item $dump "$w\$screen.json" -Force
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
    Samples = $rows; Medians = @($medians)
}
$result | ConvertTo-Json -Depth 5 | Set-Content "$w\ram.json" -Encoding UTF8
$result | ConvertTo-Json -Depth 5
