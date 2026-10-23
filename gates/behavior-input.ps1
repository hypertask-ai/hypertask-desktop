param([string]$CaseFile = "$PSScriptRoot\cases.json", [int]$Skip = 0, [int]$Take = 8)
$ErrorActionPreference = 'Stop'
$w = 'C:\Users\valen\AppData\Local\Temp\htbehavior'
if ($PSScriptRoot -ne $w) { throw 'Runner must live in htbehavior' }
Add-Type @'
using System; using System.Runtime.InteropServices; using System.Collections.Generic;
public class BehaviorInput {
  public delegate bool EnumProc(IntPtr h, IntPtr p);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr p);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll", SetLastError=true)] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
  [DllImport("user32.dll")] public static extern bool GetKeyboardState(byte[] state);
  [DllImport("user32.dll")] public static extern bool SetKeyboardState(byte[] state);
  [DllImport("user32.dll", SetLastError=true)] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint key, uint type);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int w, int he, uint flags);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  public static IntPtr Find(uint pid) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, p) => { uint id; GetWindowThreadProcessId(h, out id); if (id == pid) { found = h; return false; } return true; }, IntPtr.Zero);
    return found;
  }
  static byte[] old; static uint target;
  public static void Begin(IntPtr h, int[] modifiers) {
    uint pid; target = GetWindowThreadProcessId(h, out pid);
    if (!AttachThreadInput(GetCurrentThreadId(), target, true)) throw new Exception("AttachThreadInput failed");
    old = new byte[256]; GetKeyboardState(old);
    byte[] state = new byte[256];
    foreach (int m in modifiers) { state[m] = 128; if (m == 17) state[162] = 128; if (m == 16) state[160] = 128; if (m == 18) state[164] = 128; }
    if (!SetKeyboardState(state)) throw new Exception("SetKeyboardState failed");
    foreach (int m in modifiers) Post(h, 0x100, m, 1);
  }
  public static void End(IntPtr h, int[] modifiers) {
    foreach (int m in modifiers) Post(h, 0x101, m, unchecked((int)0xC0000001));
    SetKeyboardState(old); AttachThreadInput(GetCurrentThreadId(), target, false);
  }
  public static void Post(IntPtr h, uint message, int key, int flags) {
    if (!PostMessage(h, message, (IntPtr)key, (IntPtr)flags)) throw new Exception("PostMessage failed");
  }
  public static void Key(IntPtr h, int key) {
    int scan = (int)MapVirtualKey((uint)key, 0);
    Post(h, 0x100, key, 1 | (scan << 16));
    Post(h, 0x101, key, unchecked((int)0xC0000001) | (scan << 16));
  }
}
'@
function Wait-Change([string]$file, [int]$timeout = 1200) {
  $watch = New-Object IO.FileSystemWatcher (Split-Path $file), (Split-Path $file -Leaf)
  try { $null = $watch.WaitForChanged([IO.WatcherChangeTypes]::All, $timeout) } finally { $watch.Dispose() }
}
function Read-Json([string]$path) {
  # A writer may be completing its write when the file notification arrives.
  for ($attempt = 0; $attempt -lt 8; $attempt++) {
    try { return Get-Content -Raw $path | ConvertFrom-Json } catch { Wait-Change $path 100 }
  }
  throw "Cannot read JSON: $path"
}
function Assert-That($condition, [string]$message) { if (!$condition) { throw $message } }
function Snapshot([string]$name) {
  $state = Read-Json "$w\$script:id.behavior.json"
  Copy-Item "$w\$script:id.json" "$w\$script:id-$name.json" -Force
  $state | ConvertTo-Json -Depth 30 | Set-Content "$w\$script:id-$name.behavior.json" -Encoding UTF8
  return $state
}
function Key([string]$chord) {
  $parts = $chord.Split('+'); $last = $parts[-1]
  $mods = @(); if ($parts -contains 'Ctrl') { $mods += 17 }; if ($parts -contains 'Shift') { $mods += 16 }; if ($parts -contains 'Alt') { $mods += 18 }
  $special = @{ Enter=13; Esc=27; Tab=9; Down=40; Up=38; Left=37; Right=39; F2=113; Home=36; End=35; Backspace=8; Delete=46 }
  $vk = if ($special.ContainsKey($last)) { $special[$last] } elseif ($last.Length -eq 1) { [int][char]$last.ToUpper() } else { throw "Unknown key $chord" }
  [BehaviorInput]::Begin($script:h, [int[]]$mods)
  try { [BehaviorInput]::Key($script:h, $vk); Wait-Change "$w\$script:id.behavior.json" } finally { [BehaviorInput]::End($script:h, [int[]]$mods) }
}
function Type-Text([string]$text) {
  foreach ($c in $text.ToCharArray()) { [BehaviorInput]::Post($script:h, 0x102, [int]$c, 1) }
  Wait-Change "$w\$script:id.behavior.json"
}
function Click([string]$key) {
  $s = Read-Json "$w\$script:id.behavior.json"
  if ($key -eq "inbox.tab.last") { $key = ($s.elements | Where-Object { $_.key -match "^inbox.tab.[0-9]+$" } | Sort-Object { [int]($_.key.Split(".")[-1]) } | Select-Object -Last 1).key }
  $e = @($s.elements | Where-Object key -eq $key)[0]
  Assert-That ($null -ne $e) "Missing measured element $key"
  $scale = [BehaviorInput]::GetDpiForWindow($script:h) / 96.0
  $x = [int](($e.x + $e.w / 2) * $scale); $y = [int](($e.y + $e.h / 2) * $scale)
  $pos = ($y -shl 16) -bor ($x -band 65535)
  [BehaviorInput]::Post($script:h, 0x200, 0, $pos)
  [BehaviorInput]::Post($script:h, 0x201, 1, $pos)
  [BehaviorInput]::Post($script:h, 0x202, 0, $pos)
  Wait-Change "$w\$script:id.behavior.json"
}
$results = @()
foreach ($case in @((Read-Json $CaseFile) | Select-Object -Skip $Skip -First $Take)) {
  $script:id = $case.id
  if ($id -notmatch '^[a-z0-9-]+$') { throw 'Unsafe case id' }
  $p = $null; $failure = $null
  try {
    Copy-Item "$w\parity.db" "$w\$id.db" -Force
    $foreground = [BehaviorInput]::GetForegroundWindow()
    $p = Start-Process "$w\ht-behavior.exe" -PassThru -ArgumentList '--db', "$w\$id.db", '--offline', '--background', '--board', '7283', '--task', '60253', '--theme', 'graphite', '--screen', $case.screen, '--dump-layout', "$w\$id.json" -RedirectStandardOutput "$w\$id.stdout" -RedirectStandardError "$w\$id.stderr"
    if (!(Test-Path "$w\$id.behavior.json")) { Wait-Change "$w\$id.behavior.json" 15000 }
    $script:h = [BehaviorInput]::Find($p.Id)
    Assert-That ($h -ne [IntPtr]::Zero) 'Test HWND not found'
    # Bottom of Z-order, shown without activation; never attach to the owner's input queue.
    $null = [BehaviorInput]::SetWindowPos($h, [IntPtr]1, 0, 0, 1440, 940, 0x10 -bor 0x40)
    if (!(Test-Path "$w\$id.behavior.json")) { Wait-Change "$w\$id.behavior.json" 15000 }
    $before = Snapshot 'before'
    foreach ($step in $case.steps) {
      switch ($step.op) {
        'key' { Key $step.value }
        'type' { Type-Text $step.value }
        'click' { Click $step.value }
        'expect' {
          $current = Snapshot $step.name
          $actual = $current.state.($step.field)
          Assert-That ($actual -eq $step.value) "$($step.field): expected $($step.value), got $actual"
        }
        default { throw "Unknown operation $($step.op)" }
      }
      Assert-That ([BehaviorInput]::GetForegroundWindow() -ne $h) 'Test copy stole foreground focus'
    }
    $null = Snapshot 'after'
    Assert-That ([BehaviorInput]::GetForegroundWindow() -ne $h) 'Test copy stole foreground focus'
  } catch { $failure = $_.Exception.Message }
  finally {
    if ($p -and !$p.HasExited) {
      # Graceful close checkpoints SQLite; forced termination is an infrastructure failure.
      [BehaviorInput]::Post($script:h, 0x10, 0, 0)
      if (!$p.WaitForExit(10000)) { Stop-Process -Id $p.Id -Force; $failure = 'Test copy failed to exit gracefully' }
    }
  }
  $result = [pscustomobject]@{ id=$id; passed=($null -eq $failure); error=$failure; database="$id.db" }
  $results += $result
  if ($failure) { "FAIL $id $failure" } else { "INPUT_OK $id (SQLite assertions pending on VPS)" }
}
$results | ConvertTo-Json -Depth 10 | Set-Content "$w\results-$Skip.json" -Encoding UTF8
