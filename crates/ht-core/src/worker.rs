use crate::{Db, Drain, Result, Sync, Transport};
use std::{
    path::Path,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct WorkerHandle {
    sender: mpsc::Sender<Command>,
}

pub struct Worker {
    handle: WorkerHandle,
    thread: Option<thread::JoinHandle<()>>,
    errors: mpsc::Receiver<String>,
}

enum Command {
    Board(i64),
    Task(i64),
    Inbox,
    Event(String),
    Wake,
    Stop,
}

impl WorkerHandle {
    pub fn sync_board(&self, id: i64) -> Result<()> {
        Ok(self.sender.send(Command::Board(id))?)
    }
    pub fn sync_task(&self, id: i64) -> Result<()> {
        Ok(self.sender.send(Command::Task(id))?)
    }
    pub fn sync_inbox(&self) -> Result<()> {
        Ok(self.sender.send(Command::Inbox)?)
    }
    pub fn event(&self, frame: String) -> Result<()> {
        Ok(self.sender.send(Command::Event(frame))?)
    }
    pub fn wake(&self) -> Result<()> {
        Ok(self.sender.send(Command::Wake)?)
    }
}

impl Worker {
    pub fn start(path: impl AsRef<Path>, transport: impl Transport) -> Result<Self> {
        let db = Db::open(path)?;
        let (sender, receive) = mpsc::channel();
        let (errors_tx, errors) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("ht-sync".into())
            .spawn(move || {
                let mut sync = Sync::new(db, transport);
                let mut boards = std::collections::BTreeSet::new();
                let mut inbox = false;
                let mut task = None;
                let mut last_pull = Instant::now();
                loop {
                    let command = match receive.recv_timeout(Duration::from_secs(1)) {
                        Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Ok(command) => Some(command),
                        Err(mpsc::RecvTimeoutError::Timeout) => None,
                    };
                    let result: Result<()> = (|| {
                        match command {
                            Some(Command::Board(id)) => {
                                boards.insert(id);
                                sync.projects()?;
                                sync.board(id)?;
                            }
                            Some(Command::Task(id)) => {
                                task = Some(id);
                                sync.task(id)?;
                                sync.comments(id)?;
                                sync.drafts(id)?;
                            }
                            Some(Command::Inbox) => {
                                inbox = true;
                                sync.inbox()?;
                            }
                            Some(Command::Event(frame)) => sync.db.apply_event(&frame)?,
                            Some(Command::Wake) => {
                                for &id in &boards {
                                    sync.board(id)?;
                                }
                                if inbox {
                                    sync.inbox()?;
                                }
                                if let Some(id) = task {
                                    sync.task(id)?;
                                    sync.comments(id)?;
                                    sync.drafts(id)?;
                                }
                            }
                            _ => (),
                        }
                        loop {
                            match sync
                                .db
                                .drain_once(&mut sync.transport, chrono::Utc::now().timestamp())?
                            {
                                Drain::Applied(_) | Drain::Failed(_) => continue,
                                _ => break,
                            }
                        }
                        sync.refetch_pending()?;
                        if last_pull.elapsed() >= Duration::from_secs(60) {
                            last_pull = Instant::now();
                            for &id in &boards {
                                sync.board(id)?;
                            }
                            if inbox {
                                sync.inbox()?;
                            }
                            if let Some(id) = task {
                                sync.task(id)?;
                                sync.comments(id)?;
                                sync.drafts(id)?;
                            }
                        }
                        Ok(())
                    })();
                    if let Err(error) = result {
                        let _ = errors_tx.send(error.to_string());
                    }
                }
            })?;
        Ok(Self {
            handle: WorkerHandle { sender },
            thread: Some(thread),
            errors,
        })
    }

    pub fn handle(&self) -> WorkerHandle {
        self.handle.clone()
    }
    pub fn errors(&self) -> &mpsc::Receiver<String> {
        &self.errors
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.handle.sender.send(Command::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
