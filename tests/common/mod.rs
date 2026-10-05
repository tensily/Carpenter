//! Shared helpers for integration tests (`tests/*.rs`): unique temp dirs,
//! a release-layout fixture (tarball + `SHA256SUMS` from the real test
//! binary), and subprocess env sandboxing (Linux + macOS `dirs` resolution).

#![allow(dead_code)] // each test binary consumes a subset

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use carpenter::core::release::{self, checksum_tool_for};

static UNIQUE_SEQ: AtomicU64 = AtomicU64::new(0);

/// A unique temp dir path for `tag` (caller creates it). Uniqueness never
/// depends on clock resolution: the pid separates test binaries and the
/// atomic counter separates threads (`SystemTime` granularity is
/// platform-dependent — two macOS test threads once read the same nanosecond
/// and one `release_fixture` deleted the other's staged binary).
pub fn unique(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = UNIQUE_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "carpenter-it-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ))
}

#[test]
fn unique_is_distinct_under_contention() {
    let paths = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    for _ in 0..16 {
        let paths = std::sync::Arc::clone(&paths);
        handles.push(std::thread::spawn(move || {
            for _ in 0..64 {
                paths.lock().unwrap().push(unique("contend"));
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    let paths = paths.lock().unwrap();
    let mut distinct = std::collections::HashSet::new();
    for p in paths.iter() {
        assert!(
            distinct.insert(p.clone()),
            "duplicate temp path {}",
            p.display()
        );
    }
    assert_eq!(distinct.len(), 16 * 64);
}

/// Build a release-layout fixture (tarball + correct SHA256SUMS) from the real
/// carpenter test binary; returns its dir.
pub fn release_fixture(target: &str) -> PathBuf {
    let dir = unique("rel");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_carpenter"), dir.join("carpenter")).unwrap();
    let tarball = format!("carpenter-{target}.tar.gz");
    let status = Command::new("tar")
        .args(["-czf"])
        .arg(dir.join(&tarball))
        .arg("-C")
        .arg(&dir)
        .arg("carpenter")
        .status()
        .unwrap();
    assert!(status.success());
    std::fs::remove_file(dir.join("carpenter")).unwrap();
    let (tool, args) = checksum_tool_for(std::env::consts::OS).unwrap();
    let out = Command::new(tool)
        .args(args)
        .arg(dir.join(&tarball))
        .output()
        .unwrap();
    assert!(out.status.success());
    let hash = String::from_utf8_lossy(&out.stdout);
    let hash = hash.split_whitespace().next().unwrap();
    std::fs::write(dir.join("SHA256SUMS"), format!("{hash}  {tarball}\n")).unwrap();
    dir
}

/// The platform target for this runner, if a release asset is published for it.
pub fn platform_target() -> Option<&'static str> {
    release::platform_target()
}

/// The config root a carpenter subprocess will resolve when spawned with
/// `HOME=<root>` (+ `XDG_CONFIG_HOME`): `~/.config` on Linux,
/// `~/Library/Application Support` on macOS (`dirs::config_dir()` ignores
/// `XDG_CONFIG_HOME` there).
pub fn config_root(root: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        root.join("Library").join("Application Support")
    } else {
        root.join(".config")
    }
}

/// A sandboxed subprocess builder: carpenter spawned with `HOME` and
/// `XDG_CONFIG_HOME` pointed inside `root`, so config resolution stays off
/// the real user config on both Linux and macOS.
pub fn sandboxed(exe: &Path, root: &Path) -> Command {
    let mut cmd = Command::new(exe);
    cmd.env("HOME", root)
        .env("XDG_CONFIG_HOME", config_root(root))
        .arg("--root")
        .arg(root);
    cmd
}

/// The sandboxed opencode paths for `root`: (`SKILL.md`, `opencode.json`).
pub fn opencode_paths(root: &Path) -> (PathBuf, PathBuf) {
    let cfg = config_root(root);
    let skill = cfg
        .join("opencode")
        .join("skills")
        .join("carpenter")
        .join("SKILL.md");
    let perms = cfg.join("opencode").join("opencode.json");
    (skill, perms)
}

/// The sandboxed carpenter config file for `root`.
pub fn config_file(root: &Path) -> PathBuf {
    config_root(root).join("carpenter").join("config.json")
}
