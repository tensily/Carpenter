//! Install discovery + removal shared by `installs list`/`installs remove` and
//! the `uninstall` command. An install is identified by its tag
//! ([`store::install_name`] — adr/027): the
//! binary name, skill dirs, permission keys, and data dir all derive from it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::core::error::CarpenterError;
use crate::core::skill;
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

/// Discover installs: the union of `carpenter*` binaries in `bin_dir`, skills
/// registered with opencode, and data dirs under `xdg` (the config base
/// — every install's [`store::config_dir`] is
/// a `<tag>` leaf under it), keyed by tag and sorted by tag. `carpenter`
/// itself is the stock install (adr/027).
pub fn list(bin_dir: &Path, xdg: &Path) -> Vec<DiscoveredInstall> {
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
    // the "what names count as an install" rule, hoisted once: the tag is the
    // stock name or a `<NAME>-…` kebab (binaries additionally accept the
    // underscored command spelling, adr/027)
    let kebab = format!("{}-", skill::NAME);
    let underscored = format!("{}_", skill::NAME);
    let tag_like = |s: &str| s == skill::NAME || s.starts_with(&kebab);
    let bin_tag = |p: &Path| -> Option<String> {
        let stem = p.file_stem()?.to_string_lossy().into_owned();
        let is_install = tag_like(&stem) || stem.starts_with(&underscored);
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
    let skills_root = xdg.join(skill::APP).join("skills");
    if let Ok(entries) = std::fs::read_dir(skills_root) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if tag_like(&name) && skill::name_is_valid(&name) && e.path().join("SKILL.md").is_file()
            {
                note(name, &|d| d.skill_apps.push(String::from(skill::APP)));
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(xdg) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if tag_like(&name) && skill::name_is_valid(&name) {
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
    /// skill removal outcome: `{removed,app,path}` or
    /// `{removed:false,reason:"not_registered"}`.
    pub skill: Value,
    /// whether the config file was removed.
    pub config_purged: bool,
}

/// Everything a removal would touch, discovered up front — shared by the
/// confirmation plan ([`removal_plan`]) and the removal itself ([`remove`]),
/// so the plan cannot disagree with the outcome.
struct Legs {
    /// binary candidate path (may not exist).
    bin: PathBuf,
    /// whether the binary exists.
    bin_present: bool,
    /// whether the opencode skill is registered (skill file present).
    skill_present: bool,
    /// the install's data dir (`<config base>/<tag>`), present or not.
    data_dir: PathBuf,
    /// the install's config file (`<data_dir>/config.json`) — the `--purge-config`
    /// target: the removed tag's config, never the calling install's.
    config: PathBuf,
}

fn legs(paths: &Paths, bin_dir: Option<&str>, name: &str) -> Result<Legs, CarpenterError> {
    let target_dir = paths.resolve_bin_dir(bin_dir);
    let bin = find_binary(&target_dir, name)
        .unwrap_or_else(|| target_dir.join(store::binary_file_name(name)));
    let xdg = paths.xdg_root()?;
    let data_dir = xdg.join(name);
    Ok(Legs {
        bin_present: bin.exists(),
        bin,
        skill_present: skill::skill_path(xdg, name).exists(),
        config: data_dir.join("config.json"),
        data_dir,
    })
}

/// The human-readable removal plan shown before confirmation: exactly what the
/// removal will touch, and what it deliberately keeps (the data dir — adr/019).
/// Derived from the same discovery as [`remove`], so the plan is always
/// truthful — including that skills are machine-global per tag (adr/027) and
/// `--bin-dir` scopes only the binary.
pub fn removal_plan(
    paths: &Paths,
    bin_dir: Option<&str>,
    purge_config: bool,
    name: &str,
) -> Result<String, CarpenterError> {
    let l = legs(paths, bin_dir, name)?;
    let mut plan = format!("removal plan for {name:?}\n");
    plan.push_str(&format!(
        "  binary: {}{}\n",
        l.bin.display(),
        if l.bin_present { "" } else { " (not present)" }
    ));
    if l.skill_present {
        let xdg = paths.xdg_root()?;
        plan.push_str(&format!(
            "  skill: {} (opencode reads it from this machine-global path; --bin-dir scopes only the binary)\n",
            skill::skill_path(xdg, name).display()
        ));
    } else {
        plan.push_str("  skills: none registered\n");
    }
    if purge_config {
        plan.push_str(&format!(
            "  config: {}{}\n",
            l.config.display(),
            if l.config.exists() {
                " (purged)"
            } else {
                " (not present)"
            }
        ));
    } else {
        plan.push_str(&format!(
            "  config: {} kept (pass --purge-config to remove)\n",
            l.config.display()
        ));
    }
    plan.push_str(&format!(
        "  data dir {} kept (adr/019: course data is never touched)\n",
        l.data_dir.display()
    ));
    Ok(plan)
}

/// Remove the install `name` (tag): the opencode skill if registered,
/// then the install's binary (the underscored command name or the kebab stem —
/// whichever exists), then the removed tag's config file with `purge_config`
/// ([adr/019] semantics — course data is never touched). `NotFound` only when
/// no leg exists at all (no registered skill, no binary, no data dir) — parity
/// with `list`, which surfaces data-dir-only installs. Callers gate this behind
/// a confirmation plan ([`removal_plan`] + `core::confirm`, or `--yes`).
pub fn remove(
    paths: &Paths,
    bin_dir: Option<&str>,
    purge_config: bool,
    name: &str,
) -> Result<Removed, CarpenterError> {
    let l = legs(paths, bin_dir, name)?;
    if !l.skill_present && !l.bin_present && !l.data_dir.exists() {
        return Err(CarpenterError::NotFound(format!(
            "carpenter install {name:?} not found (no registered skill, no binary at {}, and no data dir at {})",
            l.bin.display(),
            l.data_dir.display()
        )));
    }
    let xdg = paths.xdg_root()?;
    let skill = if l.skill_present {
        Value::Array(vec![match skill::deregister(xdg, name) {
            Ok(d) => json!({"removed": true, "app": d.app, "path": d.path}),
            Err(e) => json!({"removed": false, "reason": "not_registered", "error": e.to_string()}),
        }])
    } else {
        json!({"removed": false, "reason": "not_registered"})
    };
    let removed_bin = if l.bin_present {
        std::fs::remove_file(&l.bin).map_err(store::io_to_store)?;
        Some(l.bin.display().to_string())
    } else {
        None
    };
    let config_purged = if purge_config && l.config.exists() {
        std::fs::remove_file(&l.config).map_err(store::io_to_store)?;
        true
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
