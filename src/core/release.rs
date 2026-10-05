//! Published-release fetching for `upgrade` (adr/018): map the running platform
//! to a release asset, download + checksum-verify + extract it via subprocess
//! tools (`curl`, `tar`, `sha256sum`/`shasum`) — the same pipeline
//! `scripts/install.sh` uses, so the two paths cannot drift.
//!
//! `CARPENTER_DOWNLOAD_BASE` overrides the release URL (test/mirror hook); only
//! tests may set it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use crate::core::error::CarpenterError;
use crate::core::store::io_to_store;

/// GitHub repo that hosts releases.
pub const REPO: &str = "tensily/Carpenter";
/// The rolling prerelease tag the `nightly` branch publishes (adr/021).
pub const TAG: &str = "nightly";

/// Which published channel `upgrade` fetches (adr/021): stable is the default;
/// `nightly` is the opt-in canary channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Latest stable release (`releases/latest/download`).
    Stable,
    /// Rolling `nightly` prerelease (`releases/download/nightly`).
    Nightly,
}

impl Channel {
    /// Parse + validate a `--channel` value.
    pub fn parse(s: &str) -> Result<Channel, CarpenterError> {
        match s {
            "stable" => Ok(Channel::Stable),
            "nightly" => Ok(Channel::Nightly),
            other => Err(CarpenterError::ValidationError(format!(
                "unknown channel {other:?} (stable|nightly)"
            ))),
        }
    }

    /// Asset base URL for the channel.
    pub fn base_url(self) -> String {
        match self {
            Channel::Stable => format!("https://github.com/{REPO}/releases/latest/download"),
            Channel::Nightly => format!("https://github.com/{REPO}/releases/download/{TAG}"),
        }
    }
}

/// Map an OS/ARCH pair to the published release target triple (`None` = no asset).
pub fn target_for(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-musl"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        _ => None,
    }
}

/// The release target for the running platform (`None` = no published asset).
pub fn platform_target() -> Option<&'static str> {
    target_for(std::env::consts::OS, std::env::consts::ARCH)
}

/// Base URL for release assets; `CARPENTER_DOWNLOAD_BASE` overrides (tests/mirrors).
pub fn download_base(channel: Channel) -> String {
    std::env::var("CARPENTER_DOWNLOAD_BASE").unwrap_or_else(|_| channel.base_url())
}

/// The per-OS checksum tool (and args) used to verify `SHA256SUMS`. Public for
/// the integration-test fixture (which writes sums the same way CI does).
pub fn checksum_tool_for(os: &str) -> Option<(&'static str, &'static [&'static str])> {
    match os {
        "linux" => Some(("sha256sum", &[])),
        "macos" => Some(("shasum", &["-a", "256"])),
        _ => None,
    }
}

/// Run a prepared [`Command`]; map spawn failure + non-zero exit to `StoreError`.
fn run(cmd: &mut Command, what: &str) -> Result<Output, CarpenterError> {
    cmd.output().map_err(|e| {
        CarpenterError::StoreError(format!("failed to run `{what}` — is it on PATH? ({e})"))
    })
}

/// A downloaded, verified, extracted release binary. Owns its temp stage dir —
/// dropped (best-effort cleanup) when this value is; keep it alive until the
/// binary has been copied out.
pub struct Staged {
    /// Path to the extracted `carpenter` binary (inside the stage dir).
    pub bin: PathBuf,
    /// The release tarball URL it came from.
    pub url: String,
    /// Stage dir, removed on drop.
    dir: PathBuf,
}

impl Drop for Staged {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A unique empty dir under the system temp dir (for staging a release).
pub fn stage_dir() -> Result<PathBuf, CarpenterError> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| CarpenterError::StoreError(format!("clock: {e}")))?
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("carpenter-upgrade-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(io_to_store)?;
    Ok(dir)
}

/// Download `carpenter-<target>.tar.gz` + `SHA256SUMS` from `base`, verify the
/// checksum, and extract the binary into `dir`. Returns its path on success.
pub fn fetch_release(base: &str, target: &str, dir: &Path) -> Result<Staged, CarpenterError> {
    let tarball = format!("carpenter-{target}.tar.gz");
    let url = format!("{base}/{tarball}");
    let tar_path = dir.join(&tarball);
    let out = run(
        Command::new("curl")
            .args(["-fsSL", "--max-time", "300", "-o"])
            .arg(&tar_path)
            .arg(&url),
        "curl",
    )?;
    if !out.status.success() {
        return Err(CarpenterError::StoreError(format!(
            "download failed: {url}"
        )));
    }
    let sums_path = dir.join("SHA256SUMS");
    let out = run(
        Command::new("curl")
            .args(["-fsSL", "--max-time", "60", "-o"])
            .arg(&sums_path)
            .arg(format!("{base}/SHA256SUMS")),
        "curl",
    )?;
    if !out.status.success() {
        return Err(CarpenterError::StoreError(format!(
            "download failed: {base}/SHA256SUMS"
        )));
    }
    verify_checksum(dir, &tarball, std::env::consts::OS)?;
    let out = run(
        Command::new("tar")
            .args(["-xzf"])
            .arg(&tar_path)
            .arg("-C")
            .arg(dir),
        "tar",
    )?;
    if !out.status.success() {
        return Err(CarpenterError::StoreError(format!(
            "extract failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let bin = dir.join("carpenter");
    if !bin.exists() {
        return Err(CarpenterError::StoreError(format!(
            "tarball does not contain a 'carpenter' binary: {url}"
        )));
    }
    Ok(Staged {
        bin,
        url,
        dir: dir.to_path_buf(),
    })
}

/// Verify `<dir>/<tarball>` against its line in `<dir>/SHA256SUMS`.
fn verify_checksum(dir: &Path, tarball: &str, os: &str) -> Result<(), CarpenterError> {
    let (tool, args) = checksum_tool_for(os).ok_or_else(|| {
        CarpenterError::StoreError(format!("no checksum tool for platform `{os}`"))
    })?;
    let sums = std::fs::read_to_string(dir.join("SHA256SUMS")).map_err(io_to_store)?;
    let expected = sums
        .lines()
        .find(|l| l.ends_with(&format!(" {tarball}")))
        .and_then(|l| l.split_whitespace().next())
        .ok_or_else(|| {
            CarpenterError::StoreError(format!("SHA256SUMS has no entry for {tarball}"))
        })?;
    let out = run(Command::new(tool).args(args).arg(dir.join(tarball)), tool)?;
    if !out.status.success() {
        return Err(CarpenterError::StoreError(format!(
            "{tool} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let actual = String::from_utf8_lossy(&out.stdout);
    let actual = actual.split_whitespace().next().unwrap_or_default();
    if !expected.eq_ignore_ascii_case(actual) {
        return Err(CarpenterError::StoreError(format!(
            "checksum mismatch for {tarball}: expected {expected}, got {actual}"
        )));
    }
    Ok(())
}

/// Read the version string from a binary (`<bin> --version`) — the execution/// probe run before an upgrade replaces anything.
pub fn probe_version(bin: &Path) -> Result<String, CarpenterError> {
    let out = Command::new(bin)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| CarpenterError::StoreError(format!("failed to run new binary: {e}")))?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace()
        .nth(1)
        .map(String::from)
        .ok_or_else(|| CarpenterError::StoreError(format!("could not parse version from {text:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_mapping_covers_published_assets_only() {
        assert_eq!(
            target_for("linux", "x86_64"),
            Some("x86_64-unknown-linux-musl")
        );
        assert_eq!(target_for("macos", "aarch64"), Some("aarch64-apple-darwin"));
        assert_eq!(
            target_for("macos", "x86_64"),
            None,
            "Intel Mac: not published"
        );
        assert_eq!(target_for("windows", "x86_64"), None);
        assert_eq!(target_for("linux", "aarch64"), None);
    }

    #[test]
    fn channel_parse_branches() {
        assert_eq!(Channel::parse("stable").unwrap(), Channel::Stable);
        assert_eq!(Channel::parse("nightly").unwrap(), Channel::Nightly);
        let err = Channel::parse("edge").unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)), "{err}");
    }

    #[test]
    fn channel_base_urls_pin_the_contract() {
        // stable follows GitHub's Latest pointer; nightly pins the rolling tag
        assert_eq!(
            Channel::Stable.base_url(),
            "https://github.com/tensily/Carpenter/releases/latest/download"
        );
        assert_eq!(
            Channel::Nightly.base_url(),
            "https://github.com/tensily/Carpenter/releases/download/nightly"
        );
    }

    #[test]
    fn checksum_tool_matches_installer_sh() {
        assert_eq!(checksum_tool_for("linux"), Some(("sha256sum", &[][..])));
        assert_eq!(
            checksum_tool_for("macos"),
            Some(("shasum", &["-a", "256"][..]))
        );
        assert_eq!(checksum_tool_for("windows"), None);
    }

    #[test]
    fn verify_checksum_rejects_mismatch_and_missing_entry() {
        let Some(target) = platform_target() else {
            return;
        };
        let dir = std::env::temp_dir().join(format!(
            "carpenter-relchk-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let tarball = format!("carpenter-{target}.tar.gz");
        std::fs::write(dir.join(&tarball), b"payload").unwrap();
        std::fs::write(dir.join("SHA256SUMS"), format!("deadbeef {tarball}\n")).unwrap();
        let err = verify_checksum(&dir, &tarball, std::env::consts::OS).unwrap_err();
        assert!(err.to_string().contains("checksum mismatch"), "{err}");
        std::fs::write(dir.join("SHA256SUMS"), "0000 other-file\n").unwrap();
        let err = verify_checksum(&dir, &tarball, std::env::consts::OS).unwrap_err();
        assert!(err.to_string().contains("no entry"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
