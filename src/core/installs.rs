//! Install discovery + removal shared by `installs list`/`installs remove` and
//! the `uninstall` command. An install is identified by its tag
//! ([`store::install_name`] — adr/027): the
//! binary name, skill dirs, permission keys, and data dir all derive from it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::core::error::CarpenterError;
use crate::core::skill::{self, App};
use crate::core::store::{self, Paths};

/// One carpenter install discovered on this machine.
#[derive(Debug)]
pub struct DiscoveredInstall {
    /// The install tag (binary stem — `carpenter`, `carpenter-<branch>`).
    pub tag: String,
    /// The binary path, if present.
    pub bin: Option<PathBuf>,
    /// Agent apps holding a registered skill for this tag.
    pub skill_apps: Vec<String>,
    /// The install's data dir.
    pub data_dir: PathBuf,
}

/// Discover installs: the union of `carpenter*` binaries in `bin_dir`, skill
/// dirs registered for either app, and data dirs under `xdg` (the config base
/// — every install's [`store::config_dir`] is
/// a `<tag>` leaf under it), keyed by tag and sorted by tag. `carpenter`
/// itself is the stock install (adr/027).
pub fn list(bin_dir: &Path, xdg: &Path, home: &Path) -> Vec<DiscoveredInstall> {
    let mut tags: BTreeMap<String, DiscoveredInstall> = BTreeMap::new();
    let mut note = |tag: String, fill: &dyn Fn(&mut DiscoveredInstall)| {
        let entry = tags
            .entry(tag.clone())
            .or_insert_with(|| DiscoveredInstall {
                tag,
                bin: None,
                skill_apps: Vec::new(),
                data_dir: PathBuf::new(),
            });
        fill(entry);
    };
    let bin_tag = |p: &Path| -> Option<String> {
        let stem = p.file_stem()?.to_string_lossy().into_owned();
        // binaries carry the tag as kebab (`carpenter-nightly`) or as the
        // underscored command name (`carpenter_nightly`, adr/027)
        let is_install = stem == skill::NAME
            || stem.starts_with(&format!("{}-", skill::NAME))
            || stem.starts_with(&format!("{}_", skill::NAME));
        is_install.then(|| store::install_name_from(Some(&stem)))
    };
    if let Ok(entries) = std::fs::read_dir(bin_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file() {
                if let Some(tag) = bin_tag(&p) {
                    let path = p.clone();
                    note(tag, &|d| d.bin = Some(path.clone()));
                }
            }
        }
    }
    let skill_roots = [
        (App::Opencode, xdg.join(App::Opencode.name())),
        (App::ClaudeCode, home.join(".claude")),
    ];
    for (app, root) in skill_roots {
        let Ok(entries) = std::fs::read_dir(root.join("skills")) else {
            continue;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if (name == skill::NAME || name.starts_with(&format!("{}-", skill::NAME)))
                && skill::name_is_valid(&name)
                && e.path().join("SKILL.md").is_file()
            {
                let app = app.name().to_string();
                note(name, &|d| d.skill_apps.push(app.clone()));
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(xdg) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if (name == skill::NAME || name.starts_with(&format!("{}-", skill::NAME)))
                && skill::name_is_valid(&name)
            {
                let dir = e.path();
                note(name, &|d| d.data_dir = dir.clone());
            }
        }
    }
    tags.into_values()
        .map(|mut d| {
            if d.data_dir.as_os_str().is_empty() {
                d.data_dir = xdg.join(&d.tag);
            }
            d
        })
        .collect()
}

/// The outcome of removing an install.
#[derive(Debug)]
pub struct Removed {
    /// removed binary path (`None` if it was not present).
    pub bin: Option<String>,
    /// skill removal outcome: per-app `{removed,app,path}` or
    /// `{removed:false,reason:"not_registered"}`.
    pub skill: Value,
    /// whether the config file was removed.
    pub config_purged: bool,
}

/// Remove the install `name` (tag): skill of every registered app (best-effort),
/// then the install's binary (the underscored command name or the kebab stem —
/// whichever exists), then the config file with `purge_config`
/// ([adr/019] semantics — course data is never touched). `NotFound` when
/// neither skill nor binary exists.
pub fn remove(
    paths: &Paths,
    bin_dir: Option<&str>,
    purge_config: bool,
    name: &str,
) -> Result<Removed, CarpenterError> {
    let cfg = paths
        .config_file()
        .map(|p| crate::core::config::load_from(&p))
        .unwrap_or_default();
    let target_dir = bin_dir.map(PathBuf::from).unwrap_or(cfg.bin_dir);
    let bin = find_binary(&target_dir, name)
        .unwrap_or_else(|| target_dir.join(store::binary_file_name(name)));
    let (xdg, home) = (paths.xdg_root()?, paths.home_dir()?);
    let skill_registered = App::all()
        .iter()
        .any(|a| a.skill_path(xdg, home, name).exists());
    let config_path = paths.config_file();
    if !skill_registered && !bin.exists() {
        return Err(CarpenterError::NotFound(format!(
            "carpenter install {name:?} not found (no registered skill and no binary at {})",
            bin.display()
        )));
    }
    let skill = remove_skill(paths, name);
    let removed_bin = if bin.exists() {
        std::fs::remove_file(&bin).map_err(store::io_to_store)?;
        Some(bin.display().to_string())
    } else {
        None
    };
    let config_purged = if purge_config {
        config_path
            .map(|p| p.exists() && std::fs::remove_file(&p).map_err(store::io_to_store).is_ok())
            .unwrap_or(false)
    } else {
        false
    };
    Ok(Removed {
        bin: removed_bin,
        skill,
        config_purged,
    })
}

/// Locate an install's binary in `dir` for the tag `name`: the underscored
/// command name first, then the bare kebab tag, then any file whose stem
/// slugifies to the tag (manual renames). Deterministic (sorted scan).
fn find_binary(dir: &Path, name: &str) -> Option<PathBuf> {
    for candidate in [store::binary_file_name(name), name.to_string()] {
        let p = dir.join(&candidate);
        if p.is_file() {
            return Some(p);
        }
    }
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_stem()
                    .map(|s| store::install_name_from(s.to_str()))
                    .as_deref()
                    == Some(name)
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Remove the skill of every app registered for `name`, best-effort: a missing
/// or failing removal never fails the uninstall (mirrors `upgrade`'s
/// `skill_outcome_for`).
fn remove_skill(paths: &Paths, name: &str) -> Value {
    let (root, home) = match (paths.xdg_root(), paths.home_dir()) {
        (Ok(r), Ok(h)) => (r, h),
        (Err(e), _) | (_, Err(e)) => {
            return json!({"removed": false, "reason": "no_anchor", "error": e.to_string()})
        }
    };
    let outcomes: Vec<Value> = App::all()
        .into_iter()
        .filter(|app| app.skill_path(root, home, name).exists())
        .map(|app| match skill::deregister(app, root, home, name) {
            Ok(d) => json!({"removed": true, "app": d.app, "path": d.path}),
            Err(e) => json!({"removed": false, "reason": "not_registered", "error": e.to_string()}),
        })
        .collect();
    if outcomes.is_empty() {
        return json!({"removed": false, "reason": "not_registered"});
    }
    Value::Array(outcomes)
}
