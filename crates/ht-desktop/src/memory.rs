#[cfg(windows)]
pub fn print() -> anyhow::Result<()> {
    use anyhow::Context as _;
    use windows::Win32::System::{
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX},
        Threading::GetCurrentProcess,
    };

    let mut counters = PROCESS_MEMORY_COUNTERS_EX::default();
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    counters.cb = size;
    unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(),
            size,
        )
    }
    .context("GetProcessMemoryInfo")?;
    println!(
        "MEMORY private_mb={:.2} working_set_mb={:.2}",
        counters.PrivateUsage as f64 / 1_048_576.0,
        counters.WorkingSetSize as f64 / 1_048_576.0,
    );
    Ok(())
}

#[cfg(not(windows))]
pub fn print() -> anyhow::Result<()> {
    Ok(())
}

pub fn write_store_receipt(db: &ht_core::Db) {
    let Some(path) = std::env::var_os("GPUI_MEMORY_RECEIPT") else { return };
    let path = std::path::PathBuf::from(path).with_extension("store.json");
    let result = db.memory_usage().and_then(|usage| {
        std::fs::write(path, serde_json::to_vec(&usage)?).map_err(Into::into)
    });
    if let Err(error) = result {
        eprintln!("Hypertask: store memory measurement failed: {error:#}");
    }
}

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct MeasuredAllocator;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
#[global_allocator]
static ALLOCATOR: MeasuredAllocator = MeasuredAllocator;

fn allocated(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK.fetch_max(live, Ordering::Relaxed);
}

// Requested Rust heap bytes, not native allocations, resident pages or GPU payloads.
unsafe impl GlobalAlloc for MeasuredAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() { allocated(layout.size()); }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() { allocated(layout.size()); }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let new = unsafe { System.realloc(ptr, layout, size) };
        if !new.is_null() {
            if size >= layout.size() { allocated(size - layout.size()); }
            else { LIVE.fetch_sub(layout.size() - size, Ordering::Relaxed); }
        }
        new
    }
}

pub fn rust_heap_usage() -> serde_json::Value {
    serde_json::json!({
        "RustLiveRequestedBytes": LIVE.load(Ordering::Relaxed),
        "RustPeakRequestedBytes": PEAK.load(Ordering::Relaxed),
    })
}
