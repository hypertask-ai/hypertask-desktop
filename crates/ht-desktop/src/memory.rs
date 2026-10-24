// Keep the system allocator available for CI's otherwise identical A/B build.
#[cfg(all(windows, feature = "compact-allocator"))]
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
