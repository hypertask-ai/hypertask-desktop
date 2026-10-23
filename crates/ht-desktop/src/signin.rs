use crate::store::KeyringStore;
use ht_core::auth::{
    CALLBACK_TIMEOUT, SITE_URL, TokenStore, clear_if_unauthorized, exchange_code, serve_callback,
    sign_in_url,
};
use std::{
    net::TcpListener,
    sync::{Arc, atomic::AtomicBool, mpsc},
    thread,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Ready,
    SignedOut,
    Waiting,
}

pub enum Event {
    Idle,
    SignedIn(String),
    Expired,
}

pub struct SignIn {
    pub phase: Phase,
    pub message: Option<String>,
    store: Arc<dyn TokenStore>,
    from_env: bool,
    port: Option<u16>,
    outcome: Option<mpsc::Receiver<Result<String, String>>>,
    unauthorized: Option<Arc<AtomicBool>>,
}

impl SignIn {
    pub fn start(offline: bool) -> (Self, Option<String>) {
        Self::with_store(offline, Arc::new(KeyringStore::default()))
    }

    pub fn with_store(offline: bool, store: Arc<dyn TokenStore>) -> (Self, Option<String>) {
        let env = std::env::var("HT_TOKEN").ok().filter(|s| !s.is_empty());
        let from_env = env.is_some();
        let secret = if offline {
            None
        } else {
            env.or_else(|| store.get())
        };
        let phase = if offline || secret.is_some() {
            Phase::Ready
        } else {
            Phase::SignedOut
        };
        let signin = Self {
            phase,
            message: None,
            store,
            from_env,
            port: None,
            outcome: None,
            unauthorized: None,
        };
        (signin, secret)
    }

    pub fn is_interactive(&self) -> bool {
        !self.from_env
    }

    pub fn watch(&mut self, flag: Arc<AtomicBool>) {
        self.unauthorized = Some(flag);
    }

    pub fn begin(&mut self) {
        if self.phase == Phase::Waiting {
            if let Some(port) = self.port {
                self.open_browser(port);
            }
            return;
        }
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(_) => return self.fail("Could not start sign-in on this computer."),
        };
        let Ok(address) = listener.local_addr() else {
            return self.fail("Could not start sign-in on this computer.");
        };
        let (send, receive) = mpsc::channel();
        let spawned = thread::Builder::new()
            .name("ht-signin".into())
            .spawn(move || {
                let result = serve_callback(&listener, CALLBACK_TIMEOUT)
                    .map_err(|_| "Sign-in timed out. Try again.".to_owned())
                    .and_then(|code| {
                        exchange_code(SITE_URL, &code).map_err(|error| error.to_string())
                    });
                let _ = send.send(result);
            });
        if spawned.is_err() {
            return self.fail("Could not start sign-in on this computer.");
        }
        self.port = Some(address.port());
        self.outcome = Some(receive);
        self.message = None;
        self.phase = Phase::Waiting;
        self.open_browser(address.port());
    }

    fn open_browser(&mut self, port: u16) {
        if open::that_detached(sign_in_url(port)).is_err() {
            self.message = Some("Could not open your browser.".into());
        }
    }

    fn fail(&mut self, message: &str) {
        self.phase = Phase::SignedOut;
        self.message = Some(message.to_owned());
        self.outcome = None;
        self.port = None;
    }

    pub fn poll(&mut self) -> Event {
        if let Some(flag) = self.unauthorized.clone() {
            if self.from_env {
                flag.store(false, std::sync::atomic::Ordering::SeqCst);
            } else if clear_if_unauthorized(&flag, self.store.as_ref()) {
                self.unauthorized = None;
                self.phase = Phase::SignedOut;
                self.message = Some("Your session expired. Sign in again.".into());
                return Event::Expired;
            }
        }
        let Some(outcome) = &self.outcome else {
            return Event::Idle;
        };
        match outcome.try_recv() {
            Ok(Ok(secret)) => {
                self.outcome = None;
                self.port = None;
                let _ = self.store.set(&secret);
                self.phase = Phase::Ready;
                self.message = None;
                Event::SignedIn(secret)
            }
            Ok(Err(message)) => {
                self.fail(&message);
                Event::Idle
            }
            Err(mpsc::TryRecvError::Empty) => Event::Idle,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.fail("Sign-in failed. Try again.");
                Event::Idle
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ht_core::auth::MemoryStore;

    fn signed_in(store: Arc<MemoryStore>) -> (SignIn, Arc<AtomicBool>) {
        let flag = Arc::new(AtomicBool::new(false));
        let signin = SignIn {
            phase: Phase::Ready,
            message: None,
            store,
            from_env: false,
            port: None,
            outcome: None,
            unauthorized: Some(flag.clone()),
        };
        (signin, flag)
    }

    #[test]
    fn a_401_clears_the_stored_login_and_shows_the_panel() {
        let store = Arc::new(MemoryStore::default());
        store.set("stored").unwrap();
        let (mut signin, flag) = signed_in(store.clone());
        assert!(matches!(signin.poll(), Event::Idle));
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(matches!(signin.poll(), Event::Expired));
        assert_eq!(store.get(), None);
        assert!(signin.phase == Phase::SignedOut);
        assert!(signin.message.is_some());
    }

    #[test]
    fn a_finished_sign_in_is_stored_and_reported() {
        let store = Arc::new(MemoryStore::default());
        let (mut signin, _) = signed_in(store.clone());
        signin.phase = Phase::Waiting;
        let (send, receive) = mpsc::channel();
        signin.outcome = Some(receive);
        send.send(Ok("fresh".to_owned())).unwrap();
        assert!(matches!(signin.poll(), Event::SignedIn(s) if s == "fresh"));
        assert_eq!(store.get().as_deref(), Some("fresh"));
        assert!(signin.phase == Phase::Ready);
    }

    #[test]
    fn a_failed_sign_in_keeps_the_button_and_shows_one_line() {
        let (mut signin, _) = signed_in(Arc::new(MemoryStore::default()));
        signin.phase = Phase::Waiting;
        let (send, receive) = mpsc::channel();
        signin.outcome = Some(receive);
        send.send(Err("Hypertask rejected the sign-in code".to_owned())).unwrap();
        assert!(matches!(signin.poll(), Event::Idle));
        assert!(signin.phase == Phase::SignedOut);
        assert_eq!(signin.message.as_deref(), Some("Hypertask rejected the sign-in code"));
    }
}
