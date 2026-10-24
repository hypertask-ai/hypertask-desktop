param([Parameter(Mandatory)][int]$ProcessId)
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class MemoryInspect {
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect r);
    public struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)]
    public struct MemoryInfo {
        public IntPtr BaseAddress, AllocationBase;
        public uint AllocationProtect;
        public UIntPtr RegionSize;
        public uint State, Protect, Type;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct WorkingSetEntry { public IntPtr Address; public UIntPtr Attributes; }
    public class Region {
        public string Type, Path;
        public long CommittedBytes, ResidentBytes, PrivateResidentBytes, ShareableResidentBytes, SharedResidentBytes;
    }
    [DllImport("kernel32.dll")] static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);
    [DllImport("kernel32.dll")] static extern UIntPtr VirtualQueryEx(IntPtr h, IntPtr address, out MemoryInfo info, UIntPtr size);
    [DllImport("psapi.dll")] static extern bool QueryWorkingSetEx(IntPtr h, [In, Out] WorkingSetEntry[] entries, uint size);
    [DllImport("psapi.dll", CharSet = CharSet.Unicode)] static extern uint GetMappedFileName(IntPtr h, IntPtr address, System.Text.StringBuilder path, uint size);
    public static System.Collections.Generic.List<Region> Map(int pid) {
        var regions = new System.Collections.Generic.List<Region>();
        IntPtr process = OpenProcess(0x410, false, pid);
        if (process == IntPtr.Zero) throw new Exception("OpenProcess failed");
        try {
            long address = 0;
            MemoryInfo info;
            while (VirtualQueryEx(process, new IntPtr(address), out info, (UIntPtr)Marshal.SizeOf<MemoryInfo>()) != UIntPtr.Zero) {
                long size = (long)info.RegionSize.ToUInt64();
                if (size <= 0) break;
                if (info.State == 0x1000) {
                    var path = new System.Text.StringBuilder(1024);
                    if (info.Type != 0x20000) GetMappedFileName(process, info.BaseAddress, path, 1024);
                    var pages = new WorkingSetEntry[(size + 4095) / 4096];
                    for (int i = 0; i < pages.Length; i++) pages[i].Address = new IntPtr(info.BaseAddress.ToInt64() + (long)i * 4096);
                    if (!QueryWorkingSetEx(process, pages, (uint)(pages.Length * Marshal.SizeOf<WorkingSetEntry>()))) throw new Exception("QueryWorkingSetEx failed");
                    long resident = 0, privateResident = 0, shareable = 0, shared = 0;
                    foreach (var page in pages) {
                        ulong bits = page.Attributes.ToUInt64();
                        if ((bits & 1) == 0) continue;
                        resident += 4096;
                        if ((bits & (1UL << 15)) == 0) privateResident += 4096;
                        else {
                            shareable += 4096;
                            if (((bits >> 1) & 7) > 1) shared += 4096;
                        }
                    }
                    regions.Add(new Region {
                        Type = info.Type == 0x20000 ? "Private" : info.Type == 0x1000000 ? "Image" : "Mapped",
                        Path = path.ToString(), CommittedBytes = size, ResidentBytes = resident,
                        PrivateResidentBytes = privateResident, ShareableResidentBytes = shareable, SharedResidentBytes = shared
                    });
                }
                long next = info.BaseAddress.ToInt64() + size;
                if (next <= address) break;
                address = next;
            }
        } finally { CloseHandle(process); }
        return regions;
    }
 }
'@
$p = Get-Process -Id $ProcessId
$rect = New-Object MemoryInspect+Rect
if (-not [MemoryInspect]::GetClientRect($p.MainWindowHandle, [ref]$rect)) { throw 'No client rect' }
$before = [pscustomobject]@{ WorkingSet64 = $p.WorkingSet64; PrivateMemorySize64 = $p.PrivateMemorySize64 }
$regions = @([MemoryInspect]::Map($ProcessId))
$gpuError = $null
try {
    $gpu = @(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUProcessMemory |
        Where-Object Name -Like "pid_${ProcessId}_*" |
        Select-Object Name,DedicatedUsage,SharedUsage,TotalCommitted)
} catch { $gpu = @(); $gpuError = $_.Exception.Message }
$p.Refresh()
[pscustomobject]@{
    TimestampUtc = [DateTime]::UtcNow.ToString('o'); Pid = $p.Id
    BeforeMap = $before
    WorkingSet64 = $p.WorkingSet64; PrivateMemorySize64 = $p.PrivateMemorySize64
    ClientWidth = $rect.Right; ClientHeight = $rect.Bottom
    Dpi = [MemoryInspect]::GetDpiForWindow($p.MainWindowHandle)
    Maximized = [MemoryInspect]::IsZoomed($p.MainWindowHandle)
    Regions = $regions; Gpu = $gpu; GpuCounterError = $gpuError
    Modules = @($p.Modules | Select-Object ModuleName,ModuleMemorySize)
} | ConvertTo-Json -Depth 6
