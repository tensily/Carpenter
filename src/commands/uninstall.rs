//! `uninstall` — remove the carpenter skill from opencode,
//! delete the installed binary, and optionally purge the config (adr/019).
//!
//! Ordering: skill first (recoverable via `register`), binary last (point of
//! no return). Removing the running binary is safe on Linux/macOS — unlink
//! keeps the inode alive for the running process (the same property
//! `upgrade`'s copy-replace relies on). Course data is never touched. The
//! mechanics live in [`core::installs`](crate::core::installs) — shared with
//! `installs remove`.

use crate::core::confirm;
use crate::core::error::CarpenterError;
use crate::core::installs;
use crate::core::store::Paths;
use crate::models::Data;

/// `uninstall [--bin-dir <p>] [--purge-config] [--yes]`. `name` is the install
/// tag (adr/027) — the caller resolves it via
/// [`store::install_name`](crate::core::store::install_name); the binary name,
/// skill dirs, and permission keys all derive from it. Prints the removal plan
/// and asks for confirmation unless `--yes` (adr/019 revision); a declined or
/// absent confirmation stream is a `Conflict` error envelope.
pub fn uninstall(
    paths: &Paths,
    bin_dir: Option<&str>,
    purge_config: bool,
    yes: bool,
    name: &str,
) -> Result<Data, CarpenterError> {
    if !yes {
        confirm::confirm(&installs::removal_plan(paths, bin_dir, purge_config, name)?)?;
    }
    let r = installs::remove(paths, bin_dir, purge_config, name)?;
    Ok(Data::Uninstall {
        uninstalled: true,
        bin: r.bin,
        skill: r.skill,
        config_purged: r.config_purged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::testutil;
    use crate::core::skill;
    use serde_json::json;

    #[test]
    fn uninstall_removes_skill_and_binary() {
        let paths = testutil::meta_setup();
        crate::commands::register::register(&paths, false, "carpenter").expect("register");
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join("carpenter");
        std::fs::write(&bin, b"fake binary").unwrap();
        let Data::Uninstall {
            uninstalled,
            bin: removed_bin,
            skill,
            config_purged,
        } = uninstall(
            &paths,
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
            "carpenter",
        )
        .expect("uninstall")
        else {
            panic!("Uninstall");
        };
        assert!(uninstalled);
        assert_eq!(removed_bin.as_deref(), Some(bin.to_str().unwrap()));
        assert!(!bin.exists(), "binary should be gone");
        assert!(!skill_path_of(&paths).exists(), "skill should be gone");
        let arr = skill.as_array().expect("skill removal outcome");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["removed"], json!(true));
        assert!(!config_purged);
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn uninstall_binary_only_reports_not_registered_skill() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("carpenter"), b"fake binary").unwrap();
        let Data::Uninstall { skill, bin, .. } = uninstall(
            &paths,
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
            "carpenter",
        )
        .expect("uninstall") else {
            panic!("Uninstall");
        };
        assert_eq!(skill["removed"], json!(false));
        assert_eq!(skill["reason"], json!("not_registered"));
        assert!(bin.is_some());
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn uninstall_skill_only_reports_null_bin() {
        let paths = testutil::meta_setup();
        crate::commands::register::register(&paths, false, "carpenter").expect("register");
        let bin_dir = paths.root.join("empty-bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let Data::Uninstall { bin, skill, .. } = uninstall(
            &paths,
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
            "carpenter",
        )
        .expect("uninstall") else {
            panic!("Uninstall");
        };
        assert!(bin.is_none(), "no binary was present");
        let arr = skill.as_array().expect("skill removal outcome");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["removed"], json!(true));
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn uninstall_not_found_when_nothing_installed() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let err = uninstall(
            &paths,
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
            "carpenter",
        )
        .unwrap_err();
        assert!(matches!(err, CarpenterError::NotFound(_)), "{err:?}");
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn uninstall_bin_path_is_directory_is_store_error() {
        let paths = testutil::meta_setup();
        crate::commands::register::register(&paths, false, "carpenter").expect("register");
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(bin_dir.join("carpenter")).unwrap();
        let err = uninstall(
            &paths,
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
            "carpenter",
        )
        .unwrap_err();
        assert!(matches!(err, CarpenterError::StoreError(_)), "{err:?}");
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn uninstall_purges_config_only_with_flag() {
        let paths = testutil::meta_setup();
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("carpenter"), b"fake binary").unwrap();
        let cfg = paths.config_file().expect("config file");
        if let Some(parent) = cfg.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&cfg, b"{}").unwrap();
        let Data::Uninstall { config_purged, .. } = uninstall(
            &paths,
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
            "carpenter",
        )
        .expect("keep config") else {
            panic!("Uninstall");
        };
        assert!(!config_purged);
        assert!(cfg.exists(), "config must survive without --purge-config");
        std::fs::write(bin_dir.join("carpenter"), b"fake binary").unwrap();
        let Data::Uninstall { config_purged, .. } = uninstall(
            &paths,
            Some(bin_dir.to_str().unwrap()),
            true,
            true,
            "carpenter",
        )
        .expect("purge config") else {
            panic!("Uninstall");
        };
        assert!(config_purged);
        assert!(!cfg.exists(), "config should be purged");
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn uninstall_is_tagged_by_the_install_name() {
        let paths = testutil::meta_setup();
        crate::commands::register::register(&paths, false, "carpenter-ivan-topic")
            .expect("register");
        let bin_dir = paths.root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join("carpenter-ivan-topic");
        std::fs::write(&bin, b"fake binary").unwrap();
        let Data::Uninstall {
            bin: removed,
            skill,
            ..
        } = uninstall(
            &paths,
            Some(bin_dir.to_str().unwrap()),
            false,
            true,
            "carpenter-ivan-topic",
        )
        .expect("uninstall")
        else {
            panic!("Uninstall");
        };
        assert_eq!(removed.as_deref(), Some(bin.to_str().unwrap()));
        assert!(!bin.exists(), "tagged binary should be gone");
        let arr = skill.as_array().expect("skill removal outcome");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["removed"], json!(true));
        assert!(!paths
            .xdg_root()
            .unwrap()
            .join("opencode/skills/carpenter-ivan-topic")
            .exists());
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    fn skill_path_of(paths: &Paths) -> std::path::PathBuf {
        skill::skill_path(paths.xdg_root().unwrap(), "carpenter")
    }
}
