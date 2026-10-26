use super::*;

impl Desktop {
    pub(super) fn write_memory_owners(&self, cx: &App) {
        let Some(path) = std::env::var_os("GPUI_MEMORY_RECEIPT") else { return };
        let mut usage = crate::memory::rust_heap_usage();
        usage["InputBenchmarkProfiler"] = cfg!(feature = "input-benchmark").into();
        usage["ShapedLineBytes"] = std::mem::size_of::<gpui::ShapedLine>().into();
        usage["DataReads"] = crate::memory::data_read_usage();
        // Serialized payloads are reproducible lower bounds, not allocator accounting.
        usage["InboxItems"] = self.inbox.len().into();
        usage["InboxJsonPayloadBytes"] = crate::memory::payload_bytes(&self.inbox).into();
        usage["InboxRetainedTaskAttachments"] = self.inbox.iter().map(|n| n.raw_json.pointer("/task/attachments").and_then(serde_json::Value::as_array).map_or(0, Vec::len)).sum::<usize>().into();
        usage["BoardCards"] = self.columns.iter().map(|c| c.cards.len()).sum::<usize>().into();
        usage["BoardJsonPayloadBytes"] = crate::memory::payload_bytes(&self.columns).into();
        usage["BoardRetainedAttachments"] = self.columns.iter().flat_map(|c| &c.cards).map(|t| t.extra.get("attachments").and_then(serde_json::Value::as_array).map_or(0, Vec::len)).sum::<usize>().into();
        usage["BoardDescriptionBytes"] = self.columns.iter().flat_map(|c| &c.cards).map(|t| t.description.len()).sum::<usize>().into();
        usage["DetailJsonPayloadBytes"] = crate::memory::payload_bytes(&self.detail).into();
        usage["RichTextPayloadBytes"] = (self.description.memory_payload_bytes() + self.comments.iter().map(RichText::memory_payload_bytes).sum::<usize>()).into();
        usage["DescriptionImageDecoderPresent"] = false.into();
        usage["AvatarImageDecoderPresent"] = false.into();
        if let Some(sidebar) = &self.ai_sidebar {
            usage["Ai"] = sidebar.read(cx).memory_usage();
        }
        let path = std::path::PathBuf::from(path).with_extension("app.json");
        if let Err(error) = std::fs::write(path, usage.to_string()) {
            eprintln!("Hypertask: app memory receipt failed: {error}");
        }
    }
}
