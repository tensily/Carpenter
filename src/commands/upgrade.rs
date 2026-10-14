//! `upgrade` — replace the installed carpenter binary, then update the skill.
//!
//! Two modes (adr/018): **release** (default — fetch a published tarball,
//! verify its checksum, extract, probe, atomically replace) and **source**
//! (`--source` or config `source_dir` — rebuild via `cargo xtask build
//! --release`). Release mode picks its channel via `--channel stable|nightly`
//! (adr/021): `stable` (default) follows the Latest release published from
//! `main`; `nightly` follows the rolling prerelease published from the
//! `nightly` branch. Both modes refresh the skill if **registered**
//! (installer parity — the confirming installer never registers a new skill;
//! adr/018 update). `--bin-dir`/`--no-skill` apply to both.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::core::config;
use crate::core::error::CarpenterError;
use crate::core::exec;
use crate::core::release::{self, Staged};
use crate::core::skill;
use crate::core::store::{self, Paths};
use crate::models::Data;

/// The single-sourced "not registered" warning (spec 18).
const NOT_REGISTERED_WARNING: &str =
    "CLI not registered in any agent app — nothing to upgrade. Run `carpenter register`.";

/// `upgrade [--channel stable|nightly] [--source <p>] [--bin-dir <p>] [--no-skill]`.
/// `name` is the install tag (adr/027) — the caller resolves it via
/// [`store::install_name`]; the upgraded
/// binary replaces `<bin_dir>/<name>` in place.
pub fn upgrade(
    paths: &Paths,
    source: Option<&str>,
    bin_dir: Option<&str>,
    no_skill: bool,
    channel: &str,
    name: &str,
) -> Result<Data, CarpenterError> {
    let channel = release::Channel::parse(channel)?;
    let cfg = paths
        .config_file()
        .map(|p| config::load_from(&p))
        .unwrap_or_default();
    // `_stage` keeps the download dir alive until the binary is copied out.
    let (version, origin, built, _stage) = match resolve_mode(source, &cfg)? {
        Mode::Release => {
            let staged = upgrade_from_release(channel)?;
            (
                release::probe_version(&staged.bin)?,
                staged.url.clone(),
                staged.bin.clone(),
                Some(staged),
            )
        }
        Mode::Source(dir) => {
            let built = build_from_source(&dir)?;
            (
                release::probe_version(&built)?,
                dir.display().to_string(),
                built,
                None,
            )
        }
    };
    let target_dir = bin_dir.map(PathBuf::from).unwrap_or(cfg.bin_dir);
    std::fs::create_dir_all(&target_dir).map_err(store::io_to_store)?;
    // The upgraded binary replaces the running one in place (the running
    // file's own name — `carpenter_nightly` upgrades `carpenter_nightly`); an
    // explicit --bin-dir targets `<dir>/<binary_file_name>` instead.
    let dest = match bin_dir {
        Some(dir) => PathBuf::from(dir).join(store::binary_file_name(name)),
        None => std::env::current_exe()
            .map(|p| {
                let s = p.display().to_string();
                PathBuf::from(s.strip_suffix(" (deleted)").unwrap_or(&s))
            })
            .unwrap_or_else(|_| target_dir.join(store::binary_file_name(name))),
    };
    let tmp = dest.with_extension("carpenter-tmp");
    std::fs::copy(&built, &tmp).map_err(store::io_to_store)?;
    std::fs::rename(&tmp, &dest).map_err(store::io_to_store)?;
    let skill_outcome = if no_skill {
        None
    } else {
        Some(skill_outcome_for(paths, name))
    };
    Ok(Data::Upgrade {
        upgraded: true,
        version,
        bin: dest.display().to_string(),
        source: origin,
        skill: skill_outcome,
    })
}

/// Where this upgrade comes from: the published release or a source checkout.
enum Mode {
    /// Published release tarball (channel-dependent).
    Release,
    /// Local source dir to rebuild.
    Source(PathBuf),
}

fn resolve_mode(source: Option<&str>, cfg: &config::Config) -> Result<Mode, CarpenterError> {
    if let Some(s) = source.map(PathBuf::from) {
        return Ok(Mode::Source(s));
    }
    if let Some(s) = cfg.source_dir.clone() {
        return Ok(Mode::Source(s));
    }
    Ok(Mode::Release)
}

/// Download + verify + extract the channel's release for this platform into a
/// temp stage dir (removed on drop — adr/018).
fn upgrade_from_release(channel: release::Channel) -> Result<Staged, CarpenterError> {
    let target = release::platform_target().ok_or_else(|| {
        CarpenterError::ValidationError(format!(
            "no release asset for {} {} — build from source with `--source <path>` \
             (published: Linux x86_64, macOS Apple Silicon)",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    })?;
    let tmp = release::stage_dir()?;
    release::fetch_release(&release::download_base(channel), target, &tmp)
}

/// Run `cargo xtask build --release` in `dir`; return the built binary's path.
fn build_from_source(source_dir: &Path) -> Result<PathBuf, CarpenterError> {
    if !source_dir.is_dir() {
        return Err(CarpenterError::ValidationError(format!(
            "source dir does not exist: {}",
            source_dir.display()
        )));
    }
    // gen-howto + gen-specs + release build (the embedded howto regenerates)
    exec::run_cargo_or_store(&["xtask", "build", "--release"], source_dir)?;
    let built = source_dir.join("target/release/carpenter");
    if !built.exists() {
        return Err(CarpenterError::StoreError(format!(
            "build finished but binary not found at {}",
            built.display()
        )));
    }
    Ok(built)
}

/// Refresh the skill if registered (skill file present), best-effort:
/// a failure never fails the upgrade, and no skill is registered that wasn't
/// already (the installer only registers on confirmation — parity).
fn skill_outcome_for(paths: &Paths, name: &str) -> Value {
    let root = match paths.xdg_root() {
        Ok(r) => r,
        Err(e) => return json!({"refreshed": false, "reason": "no_anchor", "error": e.to_string()}),
    };
    if !skill::skill_path(root, name).exists() {
        return json!({"refreshed": false, "reason": "not_registered", "warning": NOT_REGISTERED_WARNING});
    }
    Value::Array(vec![match skill::register(root, name) {
        Ok(r) => json!({"refreshed": true, "app": r.app, "path": r.path}),
        Err(e) => json!({"refreshed": false, "reason": "refresh_failed", "error": e.to_string()}),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::testutil;

    #[test]
    fn upgrade_errors_when_source_dir_missing() {
        let paths = testutil::meta_setup();
        let missing = paths.root.join("no-such-src");
        let err = upgrade(
            &paths,
            Some(missing.to_str().unwrap()),
            None,
            true,
            "stable",
            "carpenter",
        )
        .unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn upgrade_rejects_unknown_channel() {
        let paths = testutil::meta_setup();
        let err = upgrade(&paths, None, None, true, "edge", "carpenter").unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn upgrade_resolves_source_from_config() {
        let paths = testutil::meta_setup();
        // config source_dir points at a non-dir → ValidationError (proves resolution)
        crate::commands::config::set(&paths, "source_dir", "/definitely/not/here").unwrap();
        let err = upgrade(&paths, None, None, true, "stable", "carpenter").unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn upgrade_release_mode_fails_cleanly_when_unreachable() {
        // unsupported platforms never reach the download (see target_for tests)
        if release::platform_target().is_none() {
            return;
        }
        let paths = testutil::meta_setup();
        // no --source, no config → release mode; point at an empty base dir.
        // (The only unit test that may touch CARPENTER_DOWNLOAD_BASE.)
        let empty = paths.root.join("empty-release");
        std::fs::create_dir_all(&empty).unwrap();
        std::env::set_var(
            "CARPENTER_DOWNLOAD_BASE",
            format!("file://{}", empty.display()),
        );
        let err = upgrade(&paths, None, None, true, "stable", "carpenter").unwrap_err();
        std::env::remove_var("CARPENTER_DOWNLOAD_BASE");
        assert!(matches!(err, CarpenterError::StoreError(_)), "{err}");
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn not_registered_warning_is_stable() {
        // guards the single-sourced string against drift (spec 18)
        assert!(NOT_REGISTERED_WARNING.contains("not registered in any agent app"));
    }

    #[test]
    fn skill_outcome_refreshes_only_registered_skills() {
        let paths = testutil::meta_setup();
        // nothing registered → single not_registered outcome
        let out = skill_outcome_for(&paths, "carpenter");
        assert_eq!(out["reason"], json!("not_registered"));
        // registered → exactly one refresh, for opencode
        skill::register(paths.xdg_root().unwrap(), "carpenter").unwrap();
        let out = skill_outcome_for(&paths, "carpenter");
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["app"], json!("opencode"));
        assert_eq!(arr[0]["refreshed"], json!(true));
        let _ = std::fs::remove_dir_all(&paths.root);
    }
}
