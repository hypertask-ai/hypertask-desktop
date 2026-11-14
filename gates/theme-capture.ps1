param(
    [Parameter(Mandatory)][string]$Exe,
    [Parameter(Mandatory)][string]$CoreExe,
    [Parameter(Mandatory)][string]$Fixtures,
    [Parameter(Mandatory)][string]$Out
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public class ThemeWindow {
    public delegate bool WindowCallback(IntPtr h, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool EnumWindows(WindowCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder name, int count);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int command);
    public struct Rect { public int Left, Top, Right, Bottom; }
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
}
'@
$Exe = (Resolve-Path $Exe).Path
$CoreExe = (Resolve-Path $CoreExe).Path
$Fixtures = (Resolve-Path $Fixtures).Path
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = (Resolve-Path $Out).Path
$seed = Join-Path $Out 'fixtures.db'
& $CoreExe --db $seed import-fixtures $Fixtures
if ($LASTEXITCODE -ne 0) { throw 'Fixture import failed' }
foreach ($theme in 'porcelain', 'graphite', 'amoled', 'dia') {
    $dir = Join-Path $Out $theme
    New-Item -ItemType Directory -Force $dir | Out-Null
    foreach ($screen in 'inbox', 'board', 'task') {
        $db = Join-Path $dir "$screen.db"
        Copy-Item $seed $db
        $app = Start-Process $Exe -PassThru -WindowStyle Normal -ArgumentList @(
            '--db', $db, '--offline', '--theme', $theme, '--screen', $screen, '--board', '15', '--task', '58006'
        )
        try {
            $deadline = [DateTime]::UtcNow.AddSeconds(30)
            $window = [IntPtr]::Zero
            do {
                if ($app.HasExited) { throw "$theme $screen exited before the window opened" }
                $window = [ThemeWindow]::Find($app.Id)
                if ($window -ne [IntPtr]::Zero) { break }
                Start-Sleep -Milliseconds 200
            } while ([DateTime]::UtcNow -lt $deadline)
            if ($window -eq [IntPtr]::Zero) { throw "$theme $screen has no window" }
            [ThemeWindow]::ShowWindow($window, 3) | Out-Null
            Start-Sleep -Seconds 2
            $rect = New-Object ThemeWindow+Rect
            if (-not [ThemeWindow]::GetClientRect($window, [ref]$rect)) { throw 'GetClientRect failed' }
            if ($rect.Right -lt 100 -or $rect.Bottom -lt 100) { throw "$theme $screen client is too small" }
            $bmp = New-Object System.Drawing.Bitmap ($rect.Right), ($rect.Bottom)
            $graphics = [System.Drawing.Graphics]::FromImage($bmp)
            $dc = $graphics.GetHdc()
            try {
                if (-not [ThemeWindow]::PrintWindow($window, $dc, 2)) { throw 'PrintWindow failed' }
            } finally { $graphics.ReleaseHdc($dc) }
            $bmp.Save((Join-Path $dir "$screen.png"))
            $graphics.Dispose()
            $bmp.Dispose()
        } finally {
            if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force }
        }
    }
}
Write-Host 'THEME_CAPTURE_OK'
