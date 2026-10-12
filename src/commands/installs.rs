//! `installs` — discover and remove carpenter installs on this machine by tag
//! ([adr/027](../../docs/adr/027-install-identity-from-binary-stem.md)).
//!
//! `list` is the discovery lens (binaries ∪ skills ∪ data dirs); `remove` runs
//! the adr/019 uninstall semantics against any tag, not just the running
//! install. Mechanics live in [`core::installs`](crate::core::installs).

use crate::core::confirm;
use crate::core::error::CarpenterError;
use crate::core::installs;
use crate::core::store::{self, Paths};
use crate::models::Data;

/// `installs list [--bin-dir <p>]`.
pub fn list(paths: &Paths, bin_dir: Option<&str>) -> Result<Data, CarpenterError> {
    let cfg = paths
        .config_file()
        .map(|p| crate::core::config::load_from(&p))
        .unwrap_or_default();
    let target_dir = bin_dir.map(std::path::PathBuf::from).unwrap_or(cfg.bin_dir);
    let xdg = paths.xdg_root()?;
    let found = installs::list(&target_dir, xdg, paths.home_dir()?);
    Ok(Data::InstallsList {
        installs: found
            .into_iter()
            .map(|d| crate::models::installs::InstallRow {
                tag: d.tag,
                bin: d.bin.map(|p| p.display().to_string()),
                skill_apps: d.skill_apps,
                data_dir: d.data_dir.display().to_string(),
            })
            .collect(),
    })
}

/// `installs remove <tag> [--bin-dir <p>] [--purge-config] [--yes]`. The `tag`
/// must be kebab-case (`^[a-z0-9]+(-[a-z0-9]+)*$`, max 60) — a `carpenter` or
/// `carpenter-<branch>` stem (adr/027). Prints the removal plan and asks for
/// confirmation unless `--yes` (adr/019 revision); a declined or absent
/// confirmation stream is a `Conflict` error envelope — nothing is removed.
pub fn remove(
    paths: &Paths,
    tag: &str,
    bin_dir: Option<&str>,
    purge_config: bool,
    yes: bool,
) -> Result<Data, CarpenterError> {
    store::validate_slug(tag)?;
    if !yes {
        confirm::confirm(&installs::removal_plan(paths, bin_dir, purge_config, tag)?)?;
    }
    let r = installs::remove(paths, bin_dir, purge_config, tag)?;
    Ok(Data::InstallsRemove {
        removed: true,
        tag: tag.to_string(),
        bin: r.bin,
        skill: r.skill,
        config_purged: r.config_purged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::testutil;
    use crate::core::skill::App;
    use serde_json::json;

    #[test]
    fn list_finds_stock_and_tagged_installs() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("carpenter"), b"fake").unwrap();
        std::fs::write(bin_dir.join("carpenter-ivan-topic"), b"fake").unwrap();
        // a non-carpenter binary and a directory must be ignored
        std::fs::write(bin_dir.join("other-tool"), b"fake").unwrap();
        std::fs::create_dir_all(bin_dir.join("carpenter-weird")).unwrap();
        crate::commands::register::register(&paths, "opencode", false, "carpenter-ivan-topic")
            .expect("register tagged skill");
        let Data::InstallsList { installs } =
            list(&paths, Some(bin_dir.to_str().unwrap())).expect("list")
        else {
            panic!("InstallsList");
        };
        let tags: Vec<&str> = installs.iter().map(|r| r.tag.as_str()).collect();
        assert_eq!(
            tags,
            vec!["carpenter", "carpenter-ivan-topic"],
            "sorted by tag"
        );
        let stock = &installs[0];
        assert_eq!(stock.skill_apps, Vec::<String>::new());
        assert_eq!(
            stock.bin.as_deref(),
            Some(bin_dir.join("carpenter").to_str().unwrap())
        );
        let tagged = &installs[1];
        assert_eq!(tagged.skill_apps, vec![String::from("opencode")]);
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn list_includes_skill_only_and_data_only_installs() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        // no binaries at all — install visible only through its skill + data dir
        crate::commands::register::register(&paths, "claude-code", false, "carpenter-old")
            .expect("register");
        std::fs::create_dir_all(paths.xdg_root().unwrap().join("carpenter-stale")).unwrap();
        let Data::InstallsList { installs } =
            list(&paths, Some(bin_dir.to_str().unwrap())).expect("list")
        else {
            panic!("InstallsList");
        };
        let tags: Vec<&str> = installs.iter().map(|r| r.tag.as_str()).collect();
        assert!(tags.contains(&"carpenter-old"), "{tags:?}");
        assert!(tags.contains(&"carpenter-stale"), "{tags:?}");
        let old = installs.iter().find(|r| r.tag == "carpenter-old").unwrap();
        assert_eq!(old.skill_apps, vec![String::from("claude-code")]);
        assert_eq!(old.bin, None);
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn remove_targets_any_install_by_tag() {
        let paths = testutil::meta_setup();
        // this process "is" the stock install; it removes a tagged one
        crate::commands::register::register(&paths, "opencode", false, "carpenter-ivan-topic")
            .expect("register");
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("carpenter-ivan-topic"), b"fake").unwrap();
        let Data::InstallsRemove {
            removed,
            tag,
            bin: _,
            skill,
            config_purged,
        } = remove(
            &paths,
            "carpenter-ivan-topic",
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
        )
        .expect("remove")
        else {
            panic!("InstallsRemove");
        };
        assert!(removed);
        assert_eq!(tag, "carpenter-ivan-topic");
        assert!(!bin_dir.join("carpenter-ivan-topic").exists());
        assert!(!paths
            .xdg_root()
            .unwrap()
            .join("opencode/skills/carpenter-ivan-topic")
            .exists());
        let arr = skill.as_array().expect("per-app outcomes");
        assert_eq!(arr[0]["removed"], json!(true));
        assert!(!config_purged);
    }

    #[test]
    fn remove_rejects_non_kebab_tags() {
        let paths = testutil::meta_setup();
        let err = remove(&paths, "Not_A_Tag", None, false, true).unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)), "{err:?}");
        let _ = std::fs::remove_dir_all(paths.root);
    }

    #[test]
    fn remove_not_found_for_unknown_tag() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let err = remove(
            &paths,
            "carpenter-ghost",
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
        )
        .unwrap_err();
        assert!(matches!(err, CarpenterError::NotFound(_)), "{err:?}");
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn remove_accepts_the_stock_tag() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("carpenter"), b"fake").unwrap();
        let Data::InstallsRemove { tag, bin, .. } = remove(
            &paths,
            "carpenter",
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
        )
        .expect("remove stock") else {
            panic!("InstallsRemove");
        };
        assert_eq!(tag, "carpenter");
        assert_eq!(
            bin.as_deref(),
            Some(bin_dir.join("carpenter").to_str().unwrap())
        );
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn list_and_remove_understand_underscored_command_names() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("carpenter_nightly"), b"fake").unwrap();
        std::fs::write(bin_dir.join("carpenter_ivan_topic"), b"fake").unwrap();
        let Data::InstallsList { installs } =
            list(&paths, Some(bin_dir.to_str().unwrap())).expect("list")
        else {
            panic!("InstallsList");
        };
        let tags: Vec<&str> = installs.iter().map(|r| r.tag.as_str()).collect();
        // underscored stems slugify onto the kebab identity (adr/027)
        assert_eq!(
            tags,
            vec!["carpenter-ivan-topic", "carpenter-nightly"],
            "{tags:?}"
        );
        // removal by tag finds the underscored file
        let Data::InstallsRemove { bin: removed, .. } = remove(
            &paths,
            "carpenter-nightly",
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
        )
        .expect("remove") else {
            panic!("InstallsRemove");
        };
        assert_eq!(
            removed.as_deref(),
            Some(bin_dir.join("carpenter_nightly").to_str().unwrap())
        );
        assert!(!bin_dir.join("carpenter_nightly").exists());
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn remove_purges_the_target_tags_config_not_the_callers() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("carpenter_nightly"), b"fake").unwrap();
        let caller_cfg = paths.config_file().expect("config file");
        std::fs::create_dir_all(caller_cfg.parent().unwrap()).unwrap();
        std::fs::write(&caller_cfg, b"{}").unwrap();
        let target_cfg = paths
            .xdg_root()
            .unwrap()
            .join("carpenter-nightly")
            .join("config.json");
        std::fs::create_dir_all(target_cfg.parent().unwrap()).unwrap();
        std::fs::write(&target_cfg, b"{}").unwrap();
        let Data::InstallsRemove {
            removed: _,
            tag: _,
            bin: _,
            skill: _,
            config_purged,
        } = remove(
            &paths,
            "carpenter-nightly",
            Some(bin_dir.to_str().unwrap()),
            true,
            true,
        )
        .expect("remove")
        else {
            panic!("InstallsRemove");
        };
        assert!(config_purged);
        assert!(!target_cfg.exists(), "the removed tag's config purged");
        assert!(caller_cfg.exists(), "the calling install's config survives");
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn remove_succeeds_for_data_dir_only_tag() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("empty-bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(paths.xdg_root().unwrap().join("carpenter-stale")).unwrap();
        let Data::InstallsRemove {
            removed,
            tag,
            bin,
            skill,
            config_purged,
        } = remove(
            &paths,
            "carpenter-stale",
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
        )
        .expect("remove")
        else {
            panic!("InstallsRemove");
        };
        assert!(removed);
        assert_eq!(tag, "carpenter-stale");
        assert_eq!(bin, None);
        assert_eq!(skill["removed"], json!(false));
        assert_eq!(skill["reason"], json!("not_registered"));
        assert!(!config_purged);
        assert!(
            paths.xdg_root().unwrap().join("carpenter-stale").exists(),
            "the data dir itself is kept (adr/019)"
        );
        let _ = std::fs::remove_dir_all(paths.root);
    }

    // guard: skill_path signature stays name-parameterized (adr/027)
    #[test]
    fn skill_paths_carry_the_tag() {
        let paths = testutil::meta_setup();
        let p = App::Opencode.skill_path(
            paths.xdg_root().unwrap(),
            paths.home_dir().unwrap(),
            "carpenter-x",
        );
        assert!(p.ends_with("opencode/skills/carpenter-x/SKILL.md"), "{p:?}");
        let _ = std::fs::remove_dir_all(paths.root);
    }
}
