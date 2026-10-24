//! Opt-in receipt workload: exercise server-shaped history without credentials or network.
use super::*;

impl AiSidebar {
    pub(super) fn load_memory_history(&mut self, cx: &mut Context<Self>) {
        if std::env::var_os("GPUI_MEMORY_RECEIPT").is_none() { return; }
        let Some(path) = std::env::var_os("HT_MEMORY_AI_HISTORY") else { return };
        let sessions: Vec<Session> = serde_json::from_reader(
            std::fs::File::open(path).expect("memory history fixture")
        ).expect("server-shaped memory history");
        self.history = sessions.into_iter().map(Arc::new).collect();
        assert!(!self.history.is_empty());
        self.open_history_chat(0, cx);
    }
}

impl AiSidebar {
    pub(crate) fn memory_usage(&self) -> serde_json::Value {
        serde_json::json!({
            "HistorySessions": self.history.len(),
            "HistoryMessages": self.history.iter().map(|s| s.messages.len()).sum::<usize>(),
            "HistoryContentBytes": self.history.iter().flat_map(|s| &s.messages).map(|m| m.content.len()).sum::<usize>(),
            "OpenChatMessages": self.chats.iter().map(|c| c.messages.len()).sum::<usize>(),
            "OpenChatRichTextBytes": self.chats.iter().flat_map(|c| &c.messages).map(|m| m.rich.memory_payload_bytes()).sum::<usize>(),
        })
    }
}
