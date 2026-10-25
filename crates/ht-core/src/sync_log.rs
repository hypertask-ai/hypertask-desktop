use crate::http::{HttpStatus, TransportError};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 128 * 1024;

pub(crate) struct SyncLog {
    path: PathBuf,
}

impl SyncLog {
    pub(crate) fn new(db: &Path) -> Self {
        Self {
            path: db.with_extension("sync.log"),
        }
    }

    pub(crate) fn failure(
        &mut self,
        operation: &'static str,
        error: &anyhow::Error,
        attempts: u32,
    ) {
        // Never format arbitrary errors: SQL, response bodies and URLs can
        // contain account content or credentials. Only typed codes are safe.
        let code = if let Some(status) = error.downcast_ref::<HttpStatus>() {
            format!("http-{}", status.0)
        } else if let Some(error) = error.downcast_ref::<TransportError>() {
            match error {
                TransportError::Offline => "offline",
                TransportError::Timeout => "timeout",
                TransportError::InvalidResponse => "invalid-response",
            }
            .into()
        } else if error.downcast_ref::<rusqlite::Error>().is_some() {
            "database".into()
        } else {
            "invalid-data".into()
        };
        self.record("failed", operation, &code, attempts);
    }

    pub(crate) fn record(
        &mut self,
        event: &'static str,
        operation: &'static str,
        code: &str,
        attempts: u32,
    ) {
        let line = format!(
            "{} event={event} operation={operation} code={code} attempts={attempts}\n",
            chrono::Utc::now().to_rfc3339()
        );
        let _ = self.append(line.as_bytes());
    }

    fn append(&self, line: &[u8]) -> std::io::Result<()> {
        if fs::metadata(&self.path).is_ok_and(|m| m.len() + line.len() as u64 > MAX_BYTES) {
            let previous = self.path.with_extension("log.1");
            match fs::remove_file(&previous) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e),
            }
            fs::rename(&self.path, previous)?;
        }
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(&self.path)?.write_all(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_are_bounded_rotated_and_never_format_untrusted_errors() {
        let dir = tempfile::tempdir().unwrap();
        let mut log = SyncLog::new(&dir.path().join("local.db"));
        let secret = "fixture-bearer-secret cookie=private task-title=private https://private/?token=private";
        for _ in 0..2000 {
            log.failure("inbox", &anyhow::anyhow!(secret), 3);
        }
        log.failure("inbox", &anyhow::Error::new(HttpStatus(429)), 4);
        log.failure("inbox", &anyhow::Error::new(TransportError::Timeout), 5);
        log.record("recovered", "inbox", "none", 5);
        for path in [&log.path, &log.path.with_extension("log.1")] {
            let text = fs::read_to_string(path).unwrap();
            assert!(text.len() as u64 <= MAX_BYTES);
            assert!(!text.contains("private"));
            assert!(!text.contains("fixture-bearer-secret"));
            assert!(text.contains("code=invalid-data"));
        }
        let text = fs::read_to_string(&log.path).unwrap();
        assert!(text.contains("code=http-429"));
        assert!(text.contains("code=timeout"));
        assert!(text.contains("event=recovered"));
    }

    #[test]
    fn unwritable_diagnostics_do_not_interrupt_sync() {
        let dir = tempfile::tempdir().unwrap();
        let mut log = SyncLog::new(&dir.path().join("missing/local.db"));
        log.record("started", "worker", "none", 0);
        assert!(!log.path.exists());
    }
}
