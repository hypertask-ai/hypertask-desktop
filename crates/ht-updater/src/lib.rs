use anyhow::{Context as _, Result, bail, ensure};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant},
};

pub const REPO: &str = "hypertask-ai/hypertask-desktop";
pub const X64: &str = "hypertask-desktop-windows-x64.exe";
pub const ARM64: &str = "hypertask-desktop-windows-arm64.exe";
pub const MANIFEST: &str = "SHA256SUMS";
const MAX_EXE: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseId {
    pub version: Version,
    pub sequence: u64,
    pub sha: String,
}

impl ReleaseId {
    pub fn parse(tag: &str) -> Result<Self> {
        let (version, build) = tag.strip_prefix('v').context("missing version prefix")?
            .split_once('+').context("missing build sequence")?;
        let version = Version::parse(version)?;
        ensure!(version.pre.is_empty() && version.build.is_empty(), "not a stable version");
        let (sequence, sha) = build.split_once('.').context("missing commit")?;
        ensure!(sha.len() == 12 && sha.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()), "invalid commit");
        ensure!(!sequence.starts_with('0'), "invalid sequence");
        Ok(Self { version, sequence: sequence.parse()?, sha: sha.into() })
    }

    pub fn newer_than(&self, current: &Self) -> bool {
        (&self.version, self.sequence) > (&current.version, current.sequence)
    }
}

pub fn compiled_release() -> ReleaseId {
    ReleaseId {
        version: Version::parse(env!("CARGO_PKG_VERSION")).expect("package version"),
        sequence: env!("HT_RELEASE_SEQUENCE").parse().unwrap_or(0),
        sha: env!("HT_RELEASE_SHA").chars().take(12).collect(),
    }
}

pub fn parse_manifest(text: &str) -> Result<BTreeMap<String, String>> {
    let mut hashes = BTreeMap::new();
    for line in text.lines() {
        let (hash, name) = line.split_once("  ").context("invalid manifest line")?;
        ensure!([X64, ARM64].contains(&name), "unexpected asset");
        ensure!(hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()), "invalid SHA-256");
        ensure!(hashes.insert(name.into(), hash.to_ascii_lowercase()).is_none(), "duplicate asset");
    }
    ensure!(hashes.len() == 2, "missing architecture");
    Ok(hashes)
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

fn release_url(tag: &str, name: &str, value: &str) -> Result<()> {
    ReleaseId::parse(tag)?;
    ensure!([X64, ARM64, MANIFEST].contains(&name), "unknown asset");
    let expected = format!("https://github.com/{REPO}/releases/download/{tag}/{name}");
    ensure!(value == expected || value == expected.replace('+', "%2B"), "untrusted release URL");
    Ok(())
}

fn fetch(url: &str, limit: u64, release_asset: bool) -> Result<Vec<u8>> {
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(Duration::from_secs(120))
        .build();
    let mut next = url.to_owned();
    for _ in 0..4 {
        let response = agent.get(&next)
            .set("User-Agent", "Hypertask-Desktop-Updater")
            .set("Accept", "application/vnd.github+json")
            .call()?;
        if response.status() == 200 {
            if let Some(size) = response.header("Content-Length") {
                ensure!(size.parse::<u64>()? <= limit, "download too large");
            }
            let mut bytes = Vec::new();
            response.into_reader().take(limit + 1).read_to_end(&mut bytes)?;
            ensure!(bytes.len() as u64 <= limit, "download too large");
            return Ok(bytes);
        }
        ensure!(release_asset && [301, 302, 303, 307, 308].contains(&response.status()), "unexpected HTTP status");
        let location = response.header("Location").context("missing redirect")?;
        let redirect = url::Url::parse(location)?;
        // Only GitHub's signed asset redirect may leave the repository URL.
        ensure!(redirect.scheme() == "https"
            && redirect.host_str() == Some("release-assets.githubusercontent.com")
            && redirect.username().is_empty() && redirect.password().is_none()
            && redirect.port().is_none()
            && redirect.path().starts_with("/github-production-release-asset/"), "untrusted redirect");
        next = location.into();
    }
    bail!("too many redirects")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Stage {
    tag: String,
    asset: String,
    hash: String,
}

#[derive(Clone)]
pub struct Install {
    exe: PathBuf,
}

impl Install {
    pub fn new(exe: impl Into<PathBuf>) -> Self { Self { exe: exe.into() } }
    fn path(&self, extension: &str) -> PathBuf { self.exe.with_extension(extension) }

    pub fn lock(&self) -> Result<File> {
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0);
        }
        Ok(options.open(self.path("update-lock"))?)
    }

    fn metadata(&self, extension: &str) -> Result<Stage> {
        let data = fs::read(self.path(extension))?;
        ensure!(data.len() <= 4096, "invalid stage metadata");
        let stage: Stage = serde_json::from_slice(&data)?;
        ReleaseId::parse(&stage.tag)?;
        ensure!([X64, ARM64].contains(&stage.asset.as_str()), "invalid staged asset");
        ensure!(stage.hash.len() == 64 && stage.hash.bytes().all(|c| c.is_ascii_hexdigit()), "invalid staged hash");
        Ok(stage)
    }

    fn write_metadata(&self, extension: &str, stage: &Stage) -> Result<()> {
        let mut file = File::create(self.path(extension))?;
        file.write_all(&serde_json::to_vec(stage)?)?;
        file.sync_all()?;
        Ok(())
    }

    pub fn stage(&self, tag: &str, asset: &str, hash: &str, bytes: &[u8]) -> Result<()> {
        ReleaseId::parse(tag)?;
        ensure!([X64, ARM64].contains(&asset), "invalid architecture");
        ensure!(bytes.len() as u64 <= MAX_EXE && !bytes.is_empty(), "invalid executable size");
        ensure!(format!("{:x}", Sha256::digest(bytes)) == hash.to_ascii_lowercase(), "hash mismatch");
        ensure!(!self.path("trial.json").exists(), "trial still pending");
        remove(&self.path("stage.json"))?;
        remove(&self.path("staged"))?;
        let mut file = File::create(self.path("download"))?;
        file.write_all(bytes)?;
        #[cfg(unix)]
        file.set_permissions(fs::metadata(&self.exe)?.permissions())?;
        file.sync_all()?;
        drop(file);
        fs::rename(self.path("download"), self.path("staged"))?;
        self.write_metadata("stage.json", &Stage { tag: tag.into(), asset: asset.into(), hash: hash.to_ascii_lowercase() })
    }

    pub fn check(&self, current: &ReleaseId, asset: &str) -> Result<bool> {
        let release: Release = serde_json::from_slice(&fetch(
            &format!("https://api.github.com/repos/{REPO}/releases/latest"), 256 * 1024, false)?)?;
        ensure!(!release.draft && !release.prerelease, "not a public stable release");
        let latest = ReleaseId::parse(&release.tag_name)?;
        if !latest.newer_than(current)
            || fs::read_to_string(self.path("rejected")).ok().as_deref() == Some(&release.tag_name) {
            return Ok(false);
        }
        if let Ok(staged) = self.metadata("stage.json") {
            if !latest.newer_than(&ReleaseId::parse(&staged.tag)?) { return Ok(false); }
        }
        let find = |name: &str| -> Result<&str> {
            let matches = release.assets.iter().filter(|a| a.name == name).collect::<Vec<_>>();
            ensure!(matches.len() == 1, "missing or duplicate asset");
            release_url(&release.tag_name, name, &matches[0].browser_download_url)?;
            Ok(&matches[0].browser_download_url)
        };
        let hashes = parse_manifest(std::str::from_utf8(&fetch(find(MANIFEST)?, 16 * 1024, true)?)?)?;
        let bytes = fetch(find(asset)?, MAX_EXE, true)?;
        self.stage(&release.tag_name, asset, hashes.get(asset).context("missing checksum")?, &bytes)?;
        Ok(true)
    }

    pub fn begin_swap(&self, current: &ReleaseId, asset: &str) -> Result<bool> {
        if !self.path("stage.json").exists() { return Ok(false); }
        let stage = self.metadata("stage.json")?;
        if !ReleaseId::parse(&stage.tag)?.newer_than(current) || stage.asset != asset {
            remove(&self.path("stage.json"))?;
            remove(&self.path("staged"))?;
            return Ok(false);
        }
        verify_file(&self.path("staged"), &stage.hash)?;
        remove(&self.path("previous.exe"))?;
        remove(&self.path("healthy"))?;
        self.write_metadata("trial.json", &stage)?;
        fs::rename(&self.exe, self.path("previous.exe"))?;
        if let Err(error) = fs::rename(self.path("staged"), &self.exe) {
            fs::rename(self.path("previous.exe"), &self.exe)?;
            remove(&self.path("trial.json"))?;
            return Err(error.into());
        }
        remove(&self.path("stage.json"))?;
        Ok(true)
    }

    pub fn mark_healthy(&self) -> Result<()> {
        let trial = self.metadata("trial.json")?;
        let mut file = File::create(self.path("healthy"))?;
        file.write_all(trial.tag.as_bytes())?;
        file.sync_all()?;
        Ok(())
    }

    pub fn accept(&self) -> Result<()> {
        let trial = self.metadata("trial.json")?;
        ensure!(fs::read_to_string(self.path("healthy"))? == trial.tag, "trial not healthy");
        remove(&self.path("trial.json"))?;
        remove(&self.path("healthy"))?;
        Ok(())
    }

    pub fn recover(&self) -> Result<bool> {
        if !self.path("trial.json").exists() { return Ok(false); }
        if self.path("previous.exe").exists() {
            remove(&self.path("failed.exe"))?;
            if self.exe.exists() { fs::rename(&self.exe, self.path("failed.exe"))?; }
            fs::rename(self.path("previous.exe"), &self.exe)?;
            // A malformed marker still restores the known predecessor.
            if let Ok(stage) = self.metadata("trial.json") {
                fs::write(self.path("rejected"), stage.tag)?;
            }
        }
        remove(&self.path("trial.json"))?;
        remove(&self.path("stage.json"))?;
        remove(&self.path("healthy"))?;
        Ok(true)
    }

    pub fn supervise(&self, args: &[std::ffi::OsString], timeout: Duration) -> Result<bool> {
        let trial = self.metadata("trial.json")?;
        let mut child = match Command::new(&self.exe).args(args)
            .env("HT_UPDATER_TRIAL", &trial.tag).spawn() {
            Ok(child) => child,
            Err(_) => { self.recover()?; return Ok(false); }
        };
        let healthy = self.wait_for_start(&mut child, timeout).unwrap_or(false);
        if healthy {
            self.accept()?;
        } else {
            // Restore only after the failed child has stopped using its files.
            let _ = child.kill();
            child.wait()?;
            self.recover()?;
        }
        Ok(healthy)
    }

    fn wait_for_start(&self, child: &mut Child, timeout: Duration) -> Result<bool> {
        let start = Instant::now();
        loop {
            let status = child.try_wait()?;
            if status.is_some_and(|s| !s.success()) { return Ok(false); }
            if self.path("healthy").exists() { return Ok(true); }
            if status.is_some() || start.elapsed() >= timeout { return Ok(false); }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn is_trial(&self) -> bool {
        self.metadata("trial.json").is_ok_and(|stage|
            std::env::var("HT_UPDATER_TRIAL").ok().as_deref() == Some(stage.tag.as_str()))
    }
}

fn verify_file(path: &Path, hash: &str) -> Result<()> {
    let file = File::open(path)?;
    ensure!(file.metadata()?.len() <= MAX_EXE, "staged executable too large");
    let mut hasher = Sha256::new();
    let mut reader = file.take(MAX_EXE + 1);
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    ensure!(format!("{:x}", hasher.finalize()) == hash, "staged hash mismatch");
    Ok(())
}

fn remove(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const TAG: &str = "v0.1.0+20.abcdef123456";
    fn current() -> ReleaseId { ReleaseId::parse("v0.1.0+19.abcdef123456").unwrap() }
    fn stage(install: &Install, bytes: &[u8]) {
        install.stage(TAG, X64, &format!("{:x}", Sha256::digest(bytes)), bytes).unwrap();
    }
    fn fixture() -> (tempfile::TempDir, Install) {
        let dir = tempfile::tempdir().unwrap();
        let install = Install::new(dir.path().join("app.exe"));
        fs::write(&install.exe, b"old").unwrap();
        (dir, install)
    }

    #[test]
    fn manifest_requires_exactly_both_architectures() {
        let text = format!("{}  {X64}\n{}  {ARM64}\n", "a".repeat(64), "B".repeat(64));
        assert_eq!(parse_manifest(&text).unwrap()[ARM64], "b".repeat(64));
        for invalid in [String::new(), text.lines().next().unwrap().into(), text.replace(X64, ARM64),
            text.replace(X64, "../app.exe"), text.replace('a', "g"), format!("{text}\n")] {
            assert!(parse_manifest(&invalid).is_err(), "accepted {invalid}");
        }
        println!("UPDATER_MANIFEST_VERIFIED");
    }

    #[test]
    fn same_or_older_releases_skip_but_main_builds_advance() {
        let now = ReleaseId::parse(TAG).unwrap();
        assert!(!now.newer_than(&now));
        assert!(!current().newer_than(&now));
        assert!(now.newer_than(&current()));
        assert!(!ReleaseId::parse("v0.0.9+100.abcdef123456").unwrap().newer_than(&now));
        assert!(ReleaseId::parse("v0.2.0+1.abcdef123456").unwrap().newer_than(&now));
        for invalid in ["v0.1.0", "v0.1.0+0.abcdef123456", "v0.1.0+2.../../../bad", "v0.1.0-rc.1+2.abcdef123456"] {
            assert!(ReleaseId::parse(invalid).is_err());
        }
        println!("UPDATER_VERSION_VERIFIED");
    }

    #[test]
    fn only_this_repositories_release_assets_are_allowed() {
        let good = format!("https://github.com/{REPO}/releases/download/{TAG}/{X64}");
        assert!(release_url(TAG, X64, &good).is_ok());
        assert!(release_url(TAG, X64, &good.replace('+', "%2B")).is_ok());
        for bad in [good.replace(REPO, "attacker/repo"), good.replace("https:", "http:"),
            format!("{good}?redirect=bad"), good.replace("github.com", "github.com.attacker.test")] {
            assert!(release_url(TAG, X64, &bad).is_err());
        }
        println!("UPDATER_ORIGIN_VERIFIED");
    }

    #[test]
    fn hash_mismatch_never_stages_and_tampered_stage_never_swaps() {
        let (_dir, install) = fixture();
        assert!(install.stage(TAG, X64, &"0".repeat(64), b"new").is_err());
        assert!(!install.path("staged").exists());
        stage(&install, b"new");
        fs::write(install.path("staged"), b"tampered").unwrap();
        assert!(install.begin_swap(&current(), X64).is_err());
        assert_eq!(fs::read(&install.exe).unwrap(), b"old");
        println!("UPDATER_HASH_REFUSAL_VERIFIED");
    }

    #[test]
    fn swap_accepts_only_a_healthy_trial_and_keeps_backup() {
        let (_dir, install) = fixture();
        stage(&install, b"new");
        assert!(install.begin_swap(&current(), X64).unwrap());
        assert_eq!(fs::read(&install.exe).unwrap(), b"new");
        assert_eq!(fs::read(install.path("previous.exe")).unwrap(), b"old");
        assert!(install.accept().is_err());
        install.mark_healthy().unwrap();
        install.accept().unwrap();
        assert!(!install.recover().unwrap());
        assert_eq!(fs::read(install.path("previous.exe")).unwrap(), b"old");
        println!("UPDATER_SWAP_ACCEPT_VERIFIED");
    }

    #[test]
    fn rollback_and_interrupted_swap_restore_old_executable() {
        for installed in [true, false] {
            let (_dir, install) = fixture();
            stage(&install, b"new");
            assert!(install.begin_swap(&current(), X64).unwrap());
            if !installed { fs::remove_file(&install.exe).unwrap(); }
            assert!(install.recover().unwrap());
            assert_eq!(fs::read(&install.exe).unwrap(), b"old");
            assert_eq!(fs::read_to_string(install.path("rejected")).unwrap(), TAG);
            assert!(!install.recover().unwrap());
        }
        println!("UPDATER_ROLLBACK_VERIFIED");
    }

    #[test]
    fn same_or_wrong_arch_stage_does_not_replace_install() {
        for arch in [X64, ARM64] {
            let (_dir, install) = fixture();
            stage(&install, b"new");
            let now = if arch == X64 { ReleaseId::parse(TAG).unwrap() } else { current() };
            assert!(!install.begin_swap(&now, arch).unwrap());
            assert_eq!(fs::read(&install.exe).unwrap(), b"old");
        }
    }

    #[cfg(windows)]
    #[test]
    fn install_lock_excludes_other_process_writers() {
        let (_dir, install) = fixture();
        let guard = install.lock().unwrap();
        assert!(install.lock().is_err());
        drop(guard);
        assert!(install.lock().is_ok());
        println!("UPDATER_INSTALL_LOCK_VERIFIED");
    }

    #[test]
    fn spawn_failure_restores_old_executable() {
        let (_dir, install) = fixture();
        stage(&install, b"not an executable");
        assert!(install.begin_swap(&current(), X64).unwrap());
        assert!(!install.supervise(&[], Duration::from_secs(1)).unwrap());
        assert_eq!(fs::read(&install.exe).unwrap(), b"old");
        assert_eq!(fs::read_to_string(install.path("rejected")).unwrap(), TAG);
        println!("UPDATER_SPAWN_FAILURE_ROLLBACK_VERIFIED");
    }

    #[test]
    fn startup_probe() {
        let Some(root) = std::env::var_os("HT_UPDATER_PROBE_ROOT") else { return; };
        if std::env::var("HT_UPDATER_PROBE_FAIL").is_ok() { std::process::exit(17); }
        Install::new(PathBuf::from(root).join("app.exe")).mark_healthy().unwrap();
    }

    #[test]
    fn running_executable_swap_and_failed_start_rollback() {
        if let Some(root) = std::env::var_os("HT_UPDATER_SWAP_ROOT") {
            let install = Install::new(PathBuf::from(root).join("app.exe"));
            let bytes = fs::read(&install.exe).unwrap();
            stage(&install, &bytes);
            assert!(install.begin_swap(&current(), X64).unwrap());
            let args = ["--exact", "tests::startup_probe", "--nocapture"].map(Into::into);
            let healthy = install.supervise(&args, Duration::from_secs(10)).unwrap();
            assert_eq!(healthy, std::env::var("HT_UPDATER_PROBE_FAIL").is_err());
            assert!(install.exe.exists());
            return;
        }
        for fail in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let exe = dir.path().join("app.exe");
            fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
            let mut command = Command::new(&exe);
            command.args(["--exact", "tests::running_executable_swap_and_failed_start_rollback", "--nocapture"])
                .env("HT_UPDATER_SWAP_ROOT", dir.path()).env("HT_UPDATER_PROBE_ROOT", dir.path());
            if fail { command.env("HT_UPDATER_PROBE_FAIL", "1"); }
            assert!(command.status().unwrap().success());
            let install = Install::new(exe);
            assert_eq!(install.path("rejected").exists(), fail);
            assert_eq!(install.path("previous.exe").exists(), !fail);
        }
        println!("UPDATER_RUNNING_EXE_HANDOFF_AND_ROLLBACK_VERIFIED");
    }
}
