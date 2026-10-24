use crate::{Db, Drain, Result, Sync, Transport, sync_log::SyncLog};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum SyncStatus {
    #[default]
    Healthy,
    Retrying,
    Unavailable,
}

impl SyncStatus {
    pub fn message(self) -> Option<&'static str> {
        (self == Self::Unavailable).then_some("Sync unavailable — edits are saved locally")
    }
}

#[derive(Clone)]
pub struct WorkerHandle {
    sender: mpsc::Sender<Command>,
}

pub struct Worker {
    handle: WorkerHandle,
    thread: Option<thread::JoinHandle<()>>,
    status: Arc<AtomicU8>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Work {
    Projects,
    Board(i64),
    Catalog(i64),
    Task(i64),
    Inbox,
    Event(String),
    Draft(i64),
    Drain,
    Refetch,
}

impl Work {
    fn name(&self) -> &'static str {
        match self {
            Self::Projects => "projects",
            Self::Board(_) => "board",
            Self::Catalog(_) => "catalog",
            Self::Task(_) => "task",
            Self::Inbox => "inbox",
            Self::Event(_) => "event",
            Self::Draft(_) => "draft",
            Self::Drain => "mutations",
            Self::Refetch => "refetch",
        }
    }

    // False means no work was performed, not evidence of recovery.
    fn run<T: Transport>(&self, sync: &mut Sync<T>) -> Result<bool> {
        match self {
            Self::Projects => sync.projects()?,
            Self::Board(id) => sync.board(*id)?,
            Self::Catalog(id) => sync.catalog(*id)?,
            Self::Task(id) => {
                if let Ok(response) = sync.transport.get("/user/context")
                    && let Ok(context) = crate::http::successful(response)
                    && let Some(id) = context["user"]["id"].as_i64().filter(|id| *id > 0)
                {
                    sync.db.kv_set("current-user-id", &id.to_string())?;
                }
                sync.task(*id)?;
                sync.comments(*id)?;
                sync.drafts(*id)?;
            }
            Self::Inbox => sync.inbox()?,
            Self::Event(frame) => sync.db.apply_event(frame)?,
            Self::Draft(id) => sync.drafts(*id)?,
            Self::Drain => {
                let mut applied = false;
                loop {
                    match sync
                        .db
                        .drain_once(&mut sync.transport, chrono::Utc::now().timestamp())?
                    {
                        Drain::Applied(_) | Drain::Failed(_) => applied = true,
                        Drain::Retrying(id) => {
                            let rows = sync.db.mutations()?;
                            let error = rows
                                .iter()
                                .find(|row| row.id == id)
                                .and_then(|row| row.last_error.as_deref());
                            if let Some(status) = error
                                .and_then(|e| e.strip_prefix("HTTP "))
                                .and_then(|s| s.parse::<u16>().ok())
                                .filter(|s| (100..600).contains(s))
                            {
                                return Err(crate::http::HttpStatus(status).into());
                            }
                            let transport = match error {
                                Some("Offline") => Some(crate::http::TransportError::Offline),
                                Some("Timeout") => Some(crate::http::TransportError::Timeout),
                                Some("InvalidResponse") => {
                                    Some(crate::http::TransportError::InvalidResponse)
                                }
                                _ => None,
                            };
                            if let Some(error) = transport {
                                return Err(error.into());
                            }
                            anyhow::bail!("mutation retry pending");
                        }
                        Drain::Waiting(_) => return Ok(false),
                        Drain::Empty => return Ok(applied),
                    }
                }
            }
            Self::Refetch => {
                if sync.db.refetches()?.is_empty() {
                    return Ok(false);
                }
                sync.refetch_pending()?;
            }
        }
        Ok(true)
    }
}

struct Pending {
    attempts: u32,
    due: Instant,
}

fn schedule(pending: &mut BTreeMap<Work, Pending>, work: Work) {
    pending.entry(work).or_insert(Pending {
        attempts: 0,
        due: Instant::now(),
    });
}

enum Command {
    Work(Work),
    Wake,
    Stop,
}

impl WorkerHandle {
    pub fn sync_board(&self, id: i64) -> Result<()> {
        Ok(self.sender.send(Command::Work(Work::Board(id)))?)
    }
    pub fn sync_catalog(&self, id: i64) -> Result<()> {
        Ok(self.sender.send(Command::Work(Work::Catalog(id)))?)
    }
    pub fn sync_task(&self, id: i64) -> Result<()> {
        Ok(self.sender.send(Command::Work(Work::Task(id)))?)
    }
    pub fn sync_inbox(&self) -> Result<()> {
        Ok(self.sender.send(Command::Work(Work::Inbox))?)
    }
    pub fn event(&self, frame: String) -> Result<()> {
        Ok(self.sender.send(Command::Work(Work::Event(frame)))?)
    }
    pub fn wake(&self) -> Result<()> {
        Ok(self.sender.send(Command::Wake)?)
    }
}

impl Worker {
    pub fn start(path: impl AsRef<Path>, transport: impl Transport) -> Result<Self> {
        let db = Db::open(&path)?;
        let mut log = SyncLog::new(path.as_ref());
        let (sender, receive) = mpsc::channel();
        let status = Arc::new(AtomicU8::new(SyncStatus::Healthy as u8));
        let worker_status = status.clone();
        let thread = thread::Builder::new()
            .name("ht-sync".into())
            .spawn(move || {
                let mut sync = Sync::new(db, transport);
                let mut boards = BTreeSet::new();
                let mut inbox = false;
                let mut task = None;
                let mut pending = BTreeMap::<Work, Pending>::new();
                let mut last_pull = Instant::now();
                let mut last_draft_pull = Instant::now();
                let mut draft_tasks = VecDeque::new();
                log.record("started", "worker", "none", 0);
                loop {
                    let command = match receive.recv_timeout(Duration::from_secs(1)) {
                        Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Ok(command) => Some(command),
                        Err(mpsc::RecvTimeoutError::Timeout) => None,
                    };
                    let wake = matches!(command, Some(Command::Wake));
                    if let Some(Command::Work(work)) = command {
                        match &work {
                            Work::Board(id) => {
                                boards.insert(*id);
                                schedule(&mut pending, Work::Projects);
                            }
                            Work::Inbox => inbox = true,
                            Work::Task(id) => task = Some(*id),
                            _ => (),
                        }
                        schedule(&mut pending, work);
                    }
                    if wake || last_pull.elapsed() >= Duration::from_secs(60) {
                        last_pull = Instant::now();
                        for &id in &boards {
                            schedule(&mut pending, Work::Board(id));
                        }
                        if inbox {
                            schedule(&mut pending, Work::Inbox);
                        }
                        if let Some(id) = task {
                            schedule(&mut pending, Work::Task(id));
                        }
                        if wake {
                            // A websocket reconnect is a reason to refetch now, not
                            // proof that previously failed HTTP work has recovered.
                            for entry in pending.values_mut() {
                                entry.due = Instant::now();
                            }
                            log.record("wake", "worker", "none", 0);
                        }
                    }
                    schedule(&mut pending, Work::Drain);
                    schedule(&mut pending, Work::Refetch);
                    // Walk board drafts round-robin within the bearer quota.
                    if last_draft_pull.elapsed() >= Duration::from_secs(1) {
                        last_draft_pull = Instant::now();
                        if draft_tasks.is_empty() {
                            for &id in &boards {
                                if let Ok(columns) = sync.db.board(id) {
                                    draft_tasks.extend(
                                        columns
                                            .into_iter()
                                            .flat_map(|c| c.cards.into_iter().map(|t| t.id)),
                                    );
                                }
                            }
                        }
                        if let Some(id) = draft_tasks.pop_front() {
                            schedule(&mut pending, Work::Draft(id));
                        }
                    }
                    let due: Vec<_> = pending
                        .iter()
                        .filter(|(_, p)| p.due <= Instant::now())
                        .map(|(w, _)| w.clone())
                        .collect();
                    for work in due {
                        let entry = pending.get_mut(&work).unwrap();
                        match work.run(&mut sync) {
                            Ok(verified) => {
                                if entry.attempts > 0 && !verified {
                                    // Empty mutation queues can verify completion; a
                                    // waiting retry or idle tick cannot clear a failure.
                                    if work != Work::Drain
                                        || !sync
                                            .db
                                            .mutations()
                                            .is_ok_and(|p| p.iter().all(|m| m.status == "failed"))
                                    {
                                        entry.due = Instant::now() + Duration::from_secs(1);
                                        continue;
                                    }
                                }
                                if entry.attempts > 0 {
                                    log.record("recovered", work.name(), "none", entry.attempts);
                                }
                                pending.remove(&work);
                            }
                            Err(error) => {
                                entry.attempts = entry.attempts.saturating_add(1);
                                entry.due = Instant::now()
                                    + Duration::from_secs(
                                        crate::queue::backoff(entry.attempts - 1) as u64,
                                    );
                                log.failure(work.name(), &error, entry.attempts);
                            }
                        }
                    }
                    let attempts = pending.values().map(|p| p.attempts).max().unwrap_or(0);
                    let next = match attempts {
                        0 => SyncStatus::Healthy,
                        1..=2 => SyncStatus::Retrying,
                        _ => SyncStatus::Unavailable,
                    };
                    worker_status.store(next as u8, Ordering::Release);
                }
            })?;
        Ok(Self {
            handle: WorkerHandle { sender },
            thread: Some(thread),
            status,
        })
    }

    pub fn handle(&self) -> WorkerHandle {
        self.handle.clone()
    }

    pub fn status(&self) -> SyncStatus {
        match self.status.load(Ordering::Acquire) {
            0 => SyncStatus::Healthy,
            1 => SyncStatus::Retrying,
            _ => SyncStatus::Unavailable,
        }
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
