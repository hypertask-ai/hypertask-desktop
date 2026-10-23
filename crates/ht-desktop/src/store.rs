use ht_core::auth::{MemoryStore, TokenStore};

const SERVICE: &str = "ai.hypertask.desktop";
const USER: &str = "default";

#[derive(Default)]
pub struct KeyringStore {
    session: MemoryStore,
}

impl KeyringStore {
    fn entry() -> Option<keyring::Entry> {
        keyring::Entry::new(SERVICE, USER).ok()
    }
}

impl TokenStore for KeyringStore {
    fn get(&self) -> Option<String> {
        Self::entry()
            .and_then(|entry| entry.get_password().ok())
            .filter(|secret| !secret.is_empty())
            .or_else(|| self.session.get())
    }

    fn set(&self, secret: &str) -> anyhow::Result<()> {
        match Self::entry().map(|entry| entry.set_password(secret)) {
            Some(Ok(())) => Ok(()),
            _ => self.session.set(secret),
        }
    }

    fn clear(&self) {
        if let Some(entry) = Self::entry() {
            let _ = entry.delete_credential();
        }
        self.session.clear();
    }
}
