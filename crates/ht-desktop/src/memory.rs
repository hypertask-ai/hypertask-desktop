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

struct PayloadCounter(usize);
impl std::io::Write for PayloadCounter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

pub fn payload_bytes(value: &impl serde::Serialize) -> usize {
    let mut counter = PayloadCounter(0);
    serde_json::to_writer(&mut counter, value).expect("memory payload serialization");
    counter.0
}

#[cfg(test)]
mod tests {
    #[test]
    fn memory_payload_counter_matches_serialization_without_retaining_a_copy() {
        let value = serde_json::json!({"unicode":"é🙂", "html":"<p>text</p>", "items":[1,2,3]});
        assert_eq!(super::payload_bytes(&value), serde_json::to_vec(&value).unwrap().len());
    }
}

static READ_BYTES: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];

pub fn measure_read<T>(owner: usize, read: impl FnOnce() -> T) -> T {
    let before = LIVE.load(Ordering::Relaxed);
    let value = read();
    READ_BYTES[owner].store(LIVE.load(Ordering::Relaxed).saturating_sub(before), Ordering::Relaxed);
    value
}

pub fn data_read_usage() -> serde_json::Value {
    serde_json::json!({
        "InboxReadNetRequestedBytes": READ_BYTES[0].load(Ordering::Relaxed),
        "BoardReadNetRequestedBytes": READ_BYTES[1].load(Ordering::Relaxed),
    })
}

#[cfg(all(windows, not(feature = "untrimmed-startup")))]
pub fn trim_after_startup(cx: &gpui::App) {
    let executor = cx.background_executor().clone();
    cx.background_executor().spawn(async move {
        executor.timer(std::time::Duration::from_secs(5)).await;
        // Startup touches font/driver/DLL pages that need not stay resident while
        // idle. This only evicts pages, not allocations; active pages fault back
        // in normally. Never repeat it on a timer or on navigation.
        use windows::Win32::System::{
            ProcessStatus::EmptyWorkingSet,
            Threading::GetCurrentProcess,
        };
        let result = unsafe { EmptyWorkingSet(GetCurrentProcess()) };
        if let Err(error) = &result {
            eprintln!("Hypertask: startup working-set trim failed: {error}");
        }
        if let Some(path) = std::env::var_os("GPUI_MEMORY_RECEIPT") {
            let path = std::path::PathBuf::from(path).with_extension("startup.json");
            let _ = std::fs::write(path, serde_json::json!({
                "StartupTrimSucceeded": result.is_ok(), "DelaySeconds": 5,
                "Repeats": false,
            }).to_string());
        }
    }).detach();
}
