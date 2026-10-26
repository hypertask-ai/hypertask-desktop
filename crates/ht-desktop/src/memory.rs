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
static LIVE_SIZES: [AtomicUsize; usize::BITS as usize] = [const { AtomicUsize::new(0) }; usize::BITS as usize];

fn size_bucket(bytes: usize) -> usize {
    bytes.max(1).ilog2() as usize
}

#[global_allocator]
static ALLOCATOR: MeasuredAllocator = MeasuredAllocator;

fn allocated(bytes: usize) {
    LIVE_SIZES[size_bucket(bytes)].fetch_add(bytes, Ordering::Relaxed);
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
        LIVE_SIZES[size_bucket(layout.size())].fetch_sub(layout.size(), Ordering::Relaxed);
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let new = unsafe { System.realloc(ptr, layout, size) };
        if !new.is_null() {
            LIVE_SIZES[size_bucket(layout.size())].fetch_sub(layout.size(), Ordering::Relaxed);
            LIVE_SIZES[size_bucket(size)].fetch_add(size, Ordering::Relaxed);
            if size >= layout.size() {
                let live = LIVE.fetch_add(size - layout.size(), Ordering::Relaxed) + size - layout.size();
                PEAK.fetch_max(live, Ordering::Relaxed);
            } else { LIVE.fetch_sub(layout.size() - size, Ordering::Relaxed); }
        }
        new
    }
}

pub fn rust_heap_usage() -> serde_json::Value {
    serde_json::json!({
        "RustLiveRequestedBytes": LIVE.load(Ordering::Relaxed),
        "RustPeakRequestedBytes": PEAK.load(Ordering::Relaxed),
        "RustLiveSizeBuckets": LIVE_SIZES.iter().map(|value| value.load(Ordering::Relaxed)).collect::<Vec<_>>(),
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
    fn shaped_line_keeps_only_one_style_inline() {
        assert!(std::mem::size_of::<gpui::ShapedLine>() < 256);
    }

    #[gpui::test]
    fn compact_line_storage_preserves_more_than_32_styles(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| {
            crate::input::TextInput::new(String::new(), "", gpui::black(), cx)
        });
        view.update_in(cx, |_, window, _| {
            let text: gpui::SharedString = "x".repeat(64).into();
            let runs = (0..64).map(|i| gpui::TextRun {
                len: 1,
                font: gpui::font("monospace"),
                color: if i % 2 == 0 { gpui::black() } else { gpui::white() },
                background_color: None,
                underline: None,
                strikethrough: None,
            }).collect::<Vec<_>>();
            let line = window.text_system().shape_line(text.clone(), gpui::px(14.), &runs, None);
            assert_eq!(line.len(), 64);
            assert_eq!(line.runs.len(), 64);
            assert_eq!(line.runs.iter().map(|run| run.glyphs.len()).sum::<usize>(), 64);
            let wrapped = window.text_system().shape_text(text, gpui::px(14.), &runs, None, None).unwrap();
            assert_eq!(wrapped.len(), 1);
            assert_eq!(wrapped[0].unwrapped_layout.len, 64);
            assert_eq!(wrapped[0].unwrapped_layout.runs.len(), 64);
            let (left, right) = line.split_at(33);
            assert_eq!((left.len(), right.len()), (33, 31));
        });
    }

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
