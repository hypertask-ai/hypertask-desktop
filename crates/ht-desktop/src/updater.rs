use ht_core::SyncLog;
use ht_updater::{Install, compiled_release};
use std::{fs::File, path::{Path, PathBuf}, sync::OnceLock, time::Duration};

static LOG_DB: OnceLock<PathBuf> = OnceLock::new();
const INTERVAL: Duration = Duration::from_secs(4 * 60 * 60);

fn record(event: &'static str, code: &'static str) {
    if let Some(db) = LOG_DB.get() {
        SyncLog::new(db).record(event, "updater", code, 0);
    }
}

fn arch() -> &'static str {
    if cfg!(target_arch = "aarch64") { ht_updater::ARM64 } else { ht_updater::X64 }
}

pub fn start(db: &Path) -> bool {
    let _ = LOG_DB.set(db.to_owned());
    let Ok(exe) = std::env::current_exe() else {
        record("failed", "executable-path");
        return false;
    };
    let install = Install::new(&exe);
    if install.is_trial() {
        check_in_background(install, None);
        return false;
    }
    let Ok(guard) = install.lock() else {
        record("skipped", "install-busy-or-readonly");
        return false;
    };
    match install.recover() {
        Ok(true) => {
            record("recovered", "interrupted-trial");
            drop(guard);
            return relaunch(&exe);
        }
        Err(_) => {
            record("failed", "rollback");
            return false;
        }
        Ok(false) => (),
    }
    match install.begin_swap(&compiled_release(), arch()) {
        Ok(true) => {
            record("started", "trial");
            let args = std::env::args_os().skip(1).collect::<Vec<_>>();
            match install.supervise(&args, Duration::from_secs(60)) {
                Ok(true) => {
                    record("accepted", "presented-frame");
                    return true;
                }
                Ok(false) => {
                    record("recovered", "failed-start");
                    drop(guard);
                    return relaunch(&exe);
                }
                Err(_) => {
                    record("failed", "trial-supervision");
                    // Never run a second UI beside an already-started trial.
                    return true;
                }
            }
        }
        Err(_) => {
            record("failed", "stage-or-swap");
            if install.recover().unwrap_or(false) {
                record("recovered", "swap-error");
            }
        }
        Ok(false) => (),
    }
    check_in_background(install, Some(guard));
    false
}

fn relaunch(exe: &Path) -> bool {
    match std::process::Command::new(exe).args(std::env::args_os().skip(1))
        .env_remove("HT_UPDATER_TRIAL").spawn() {
        Ok(_) => true,
        Err(_) => { record("failed", "relaunch"); false }
    }
}

fn check_in_background(install: Install, guard: Option<File>) {
    let _ = std::thread::Builder::new().name("ht-updater".into()).spawn(move || {
        let _guard = match guard {
            Some(guard) => guard,
            None => {
                // The renamed parent owns the install until first-frame acceptance.
                let mut acquired = None;
                for _ in 0..120 {
                    if let Ok(guard) = install.lock() { acquired = Some(guard); break; }
                    std::thread::sleep(Duration::from_secs(1));
                }
                let Some(guard) = acquired else {
                    record("skipped", "install-busy-or-readonly");
                    return;
                };
                guard
            }
        };
        loop {
            record("started", "release-check");
            match install.check(&compiled_release(), arch()) {
                Ok(true) => record("staged", "sha256-verified"),
                Ok(false) => record("skipped", "no-new-release"),
                Err(_) => record("failed", "release-check"),
            }
            std::thread::sleep(INTERVAL);
        }
    });
}

pub fn mark_ready() {
    let Ok(exe) = std::env::current_exe() else { return; };
    let install = Install::new(exe);
    if install.is_trial() {
        match install.mark_healthy() {
            Ok(()) => record("ready", "presented-frame"),
            Err(_) => record("failed", "startup-marker"),
        }
    }
}
