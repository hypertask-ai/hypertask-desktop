param([string]$CaseFile = "$PSScriptRoot\cases.json", [int]$Skip = 0, [int]$Take = 8)
$ErrorActionPreference = 'Stop'
$w = 'C:\Users\valen\AppData\Local\Temp\htbehavior'
if ($PSScriptRoot -ne $w) { throw 'Runner must live in htbehavior' }
Add-Type @'
using System; using System.Runtime.InteropServices; using System.Collections.Generic;
public class BehaviorInput {
  [StructLayout(LayoutKind.Sequential)] public struct Point { public int x, y; }
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int left, top, right, bottom; }
  [StructLayout(LayoutKind.Sequential)] struct Message { public IntPtr h; public uint m; public UIntPtr w; public IntPtr l; public uint time; public Point point; public uint extra; }
  [DllImport("user32.dll")] static extern bool PeekMessage(out Message m, IntPtr h, uint min, uint max, uint remove);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref Point point);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect rect);
  [DllImport("user32.dll")] public static extern uint GetClipboardSequenceNumber();
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
    Message message; PeekMessage(out message, IntPtr.Zero, 0, 0, 0);
    uint pid; target = GetWindowThreadProcessId(h, out pid);
    if (!AttachThreadInput(GetCurrentThreadId(), target, true)) throw new Exception("AttachThreadInput failed: " + Marshal.GetLastWin32Error());
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
    try {
      $stream = New-Object IO.FileStream $path, ([IO.FileMode]::Open), ([IO.FileAccess]::Read), ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete)
      $reader = New-Object IO.StreamReader $stream
      try { return $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
    } catch { Wait-Change $path 100 }
  }
  throw "Cannot read JSON: $path"
}
function Assert-That($condition, [string]$message) { if (!$condition) { throw $message } }
function Snapshot([string]$name) {
  $state = Read-Json "$w\$script:id.behavior.json"
  (Read-Json "$w\$script:id.json") | ConvertTo-Json -Depth 50 | Set-Content "$w\$script:id-$name.json" -Encoding UTF8
  $state | ConvertTo-Json -Depth 30 | Set-Content "$w\$script:id-$name.behavior.json" -Encoding UTF8
  return $state
}
function Key([string]$chord) {
  $parts = $chord.Split('+'); $last = $parts[-1]
  $mods = @(); if ($parts -contains 'Ctrl') { $mods += 17 }; if ($parts -contains 'Shift') { $mods += 16 }; if ($parts -contains 'Alt') { $mods += 18 }
  $special = @{ Enter=13; Esc=27; Tab=9; Down=40; Up=38; Left=37; Right=39; F2=113; Home=36; End=35; Backspace=8; Delete=46; ";"=186; ","=188; "."=190; "/"=191 }
  $vk = if ($special.ContainsKey($last)) { $special[$last] } elseif ($last.Length -eq 1) { [int][char]$last.ToUpper() } else { throw "Unknown key $chord" }
  $beforeKey = (Read-Json "$w\$script:id.behavior.json").state.key_events
  $watch = New-Object IO.FileSystemWatcher $w, "$script:id.behavior.json"
  [BehaviorInput]::Begin($script:h, [int[]]$mods)
  try {
    [BehaviorInput]::Key($script:h, $vk)
    if ($null -eq $beforeKey) { Wait-Change "$w\$script:id.behavior.json" }
    else {
      $deadline = [DateTime]::UtcNow.AddSeconds(5)
      do {
        $state = (Read-Json "$w\$script:id.behavior.json").state
        $want = switch ($last) { 'Esc' { 'escape' }; ';' { if ($mods -contains 16) { ';',':' } else { ';' } }; ',' { if ($mods -contains 16) { ',','<' } else { ',' } }; '3' { if ($mods -contains 16) { '3','#' } else { '3' } }; default { $last.ToLower() } }
        $ack = $state.key_events -gt $beforeKey -and $state.last_key -in $want -and $state.last_ctrl -eq ($mods -contains 17) -and $state.last_shift -eq ($mods -contains 16)
        if ($ack) { break }
        $left = [int]($deadline - [DateTime]::UtcNow).TotalMilliseconds
        if ($left -gt 0) { $null = $watch.WaitForChanged([IO.WatcherChangeTypes]::All, $left) }
      } while ([DateTime]::UtcNow -lt $deadline)
      Assert-That $ack "Posted key $chord was not acknowledged with its exact modifiers"
    }
  } finally { [BehaviorInput]::End($script:h, [int[]]$mods); $watch.Dispose() }

}
function Type-Text([string]$text) {
  foreach ($c in $text.ToCharArray()) { [BehaviorInput]::Post($script:h, 0x102, [int]$c, 1) }
  Wait-Change "$w\$script:id.behavior.json"
}
function Click([string]$key) {
  $scale = [BehaviorInput]::GetDpiForWindow($script:h) / 96.0
  $rect = New-Object BehaviorInput+Rect
  $null = [BehaviorInput]::GetClientRect($script:h, [ref]$rect)
  for ($attempt = 0; $attempt -lt 80; $attempt++) {
    $s = Read-Json "$w\$script:id.behavior.json"
    if ($key -eq "inbox.tab.last") { $key = ($s.elements | Where-Object { $_.key -match "^inbox.tab.[0-9]+$" } | Sort-Object { [int]($_.key.Split(".")[-1]) } | Select-Object -Last 1).key }
    $e = @($s.elements | Where-Object key -eq $key)[0]
    Assert-That ($null -ne $e) "Missing measured element $key"
    $x = [int](($e.x + $e.w / 2) * $scale); $y = [int](($e.y + $e.h / 2) * $scale)
    if ($y -gt 0 -and $y -lt ($rect.bottom - 40)) { break }
    $point = New-Object BehaviorInput+Point
    $point.x = [Math]::Min($rect.right-20, [Math]::Max(80,$x)); $point.y = [int]($rect.bottom / 2)
    $pos = ($point.y -shl 16) -bor ($point.x -band 65535)
    [BehaviorInput]::Post($script:h, 0x200, 0, $pos)
    $null = [BehaviorInput]::ClientToScreen($script:h, [ref]$point)
    $pos = ($point.y -shl 16) -bor ($point.x -band 65535)
    $delta = if ($y -gt 0) { -600 } else { 600 }
    [BehaviorInput]::Post($script:h, 0x20A, (($delta -band 65535) -shl 16), $pos)
    Wait-Change "$w\$script:id.behavior.json"
  }
  Assert-That ($y -gt 0 -and $y -lt ($rect.bottom - 40) -and $x -gt 0 -and $x -lt $rect.right) "Element $key is outside the client viewport"
  $pos = ($y -shl 16) -bor ($x -band 65535)
  [BehaviorInput]::Post($script:h, 0x200, 0, $pos)
  [BehaviorInput]::Post($script:h, 0x201, 1, $pos)
  [BehaviorInput]::Post($script:h, 0x202, 0, $pos)
  Wait-Change "$w\$script:id.behavior.json"
}
function Expected($value, $current, $before) {
  switch ($value) {
    '$last' { return $current.state.last_index }
    '$count' { return $current.state.selection_count }
    '$all' { return $current.state.inbox_count }
    '$composer' { return $current.state.composer_index }
    '$before.task_id' { return $before.state.task_id }
    default { return $value }
  }
}

Add-Type -AssemblyName System.Windows.Forms
function Clipboard-Backup {
  $original = [Windows.Forms.Clipboard]::GetDataObject()
  $copy = New-Object Windows.Forms.DataObject
  if ($null -ne $original) {
    foreach ($format in $original.GetFormats($false)) {
      $data = $original.GetData($format, $false)
      if ($data -is [IO.MemoryStream]) { $data = New-Object IO.MemoryStream (,$data.ToArray()) }
      $copy.SetData($format, $false, $data)
    }
  }
  return $copy
}
function Clipboard-Expected($kind, $state) {
  $ticket = if ($state.ticket) { $state.ticket } else { [string]$state.task_id }
  $url = "https://app.hypertask.ai/detail/project-$($state.project_id)/$($state.unique_index)"
  switch ($kind) {
    'id' { return $ticket }
    'title-id' { return "$($ticket.ToUpper()) | $($state.task_title)" }
    'url' { return $url }
    'title-url' { return "$($ticket.ToUpper()) | $($state.task_title) | $url" }
    default { throw 'Unknown clipboard assertion' }
  }
}
$results = @()
foreach ($case in @((Read-Json $CaseFile) | Select-Object -Skip $Skip -First $Take)) {
  $script:id = $case.id
  if ($id -notmatch '^[a-z0-9-]+$') { throw 'Unsafe case id' }
  $p = $null; $failure = $null; $clipboardBackup = $null; $ownClipboardSequence = $null
  try {
    if ($case.clipboard) { $clipboardBackup = Clipboard-Backup }
    Copy-Item "$w\parity.db" "$w\$id.db" -Force
    & sqlite3.exe "$w\$id.db" ".read '$w/behavior-seed.sql'"
    Assert-That ($LASTEXITCODE -eq 0) 'Cannot seed isolated behavior fixture' 
    $foreground = [BehaviorInput]::GetForegroundWindow()
    $p = Start-Process "$w\ht-behavior.exe" -PassThru -ArgumentList '--db', "$w\$id.db", '--offline', '--background', '--board', '7283', '--task', '60253', '--theme', 'graphite', '--screen', $case.screen, '--dump-layout', "$w\$id.json" -RedirectStandardOutput "$w\$id.stdout" -RedirectStandardError "$w\$id.stderr"
    $null = $p.WaitForInputIdle(1000)
    $script:h = [BehaviorInput]::Find($p.Id)
    Assert-That ($h -ne [IntPtr]::Zero) 'Test HWND not found'
    # Bottom of Z-order, shown without activation; never attach to the owner's input queue.
    $null = [BehaviorInput]::SetWindowPos($h, [IntPtr]1, 0, 0, 1440, 940, 0x10 -bor 0x40)
    $watch = New-Object IO.FileSystemWatcher $w, "$id.behavior.json"
    try { if (!(Test-Path "$w\$id.behavior.json")) { $null = $watch.WaitForChanged([IO.WatcherChangeTypes]::All, 15000) } } finally { $watch.Dispose() }
    $before = Snapshot 'before'
    foreach ($step in $case.steps) {
      switch ($step.op) {
        'key' { Key $step.value }
        'type' { Type-Text $step.value }
        'click' { Click $step.value }
        'expect' {
          $watch = New-Object IO.FileSystemWatcher $w, "$script:id.behavior.json"
          $deadline = [DateTime]::UtcNow.AddSeconds(5)
          try {
            do {
              $current = Read-Json "$w\$script:id.behavior.json"
              $actual = $current.state.($step.field)
              $wanted = Expected $step.value $current $before
              if ($actual -eq $wanted) { break }
              $left = [int]($deadline - [DateTime]::UtcNow).TotalMilliseconds
              if ($left -gt 0) { $null = $watch.WaitForChanged([IO.WatcherChangeTypes]::All, $left) }
            } while ([DateTime]::UtcNow -lt $deadline)
            Assert-That ($actual -eq $wanted) "$($step.field): expected $wanted, got $actual"
            $null = Snapshot $step.name
          } finally { $watch.Dispose() }
        }
        'expect-change' {
          $current = Snapshot $step.name
          Assert-That ($current.state.($step.field) -ne $before.state.($step.field)) "$($step.field) did not change"
        }
        'expect-element' {
          $current = Snapshot $step.name
          $element = @($current.elements | Where-Object key -eq $step.key)[0]
          Assert-That ($null -ne $element -and $element.text -eq $step.value) "Element $($step.key) did not show $($step.value)"
        }
        default { throw "Unknown operation $($step.op)" }
      }
      Assert-That ([BehaviorInput]::GetForegroundWindow() -ne $h) 'Test copy stole foreground focus'
    }
    $null = Snapshot 'after'
    if ($case.clipboard) {
      $expected = Clipboard-Expected $case.clipboard $before.state
      $actual = [Windows.Forms.Clipboard]::GetText()
      $known = @('id','title-id','url','title-url') | ForEach-Object { Clipboard-Expected $_ $before.state }
      if ($actual -cin $known) { $ownClipboardSequence = [BehaviorInput]::GetClipboardSequenceNumber() }
      Assert-That ($actual -ceq $expected) 'Clipboard does not match selected task'
      $actual | Set-Content "$w\$id-clipboard.txt" -Encoding UTF8 -NoNewline
    }
    Assert-That ([BehaviorInput]::GetForegroundWindow() -ne $h) 'Test copy stole foreground focus'
  } catch { $failure = $_.Exception.Message }
  finally {
    if ($null -ne $clipboardBackup -and $null -ne $ownClipboardSequence -and [BehaviorInput]::GetClipboardSequenceNumber() -eq $ownClipboardSequence) { [Windows.Forms.Clipboard]::SetDataObject($clipboardBackup, $true) }
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
