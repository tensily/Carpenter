//! `register` — write the carpenter `SKILL.md` into an agent app's skills dir
//! and merge the allow permission entry.
//!
//! `--print-skill` instead emits the rendered bytes (no filesystem change).

use crate::core::error::CarpenterError;
use crate::core::skill;
use crate::core::store::Paths;
use crate::models::Data;

/// `register [--print-skill]`. `name` is the skill name /
/// install tag (adr/027) — the caller resolves it via
/// [`store::install_name`](crate::core::store::install_name).
pub fn register(paths: &Paths, print_skill: bool, name: &str) -> Result<Data, CarpenterError> {
    if print_skill {
        return Ok(Data::PrintSkill {
            skill: skill::render(name)?,
        });
    }
    let r = skill::register(paths.xdg_root()?, name)?;
    Ok(Data::Register {
        app: r.app,
        path: r.path,
        version: r.version,
        installed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::testutil;

    #[test]
    fn register_ok() {
        let paths = testutil::meta_setup();
        let Data::Register {
            app,
            path,
            version,
            installed,
        } = register(&paths, false, "carpenter").expect("register")
        else {
            panic!("Register");
        };
        assert_eq!(app, "opencode");
        assert!(
            path.ends_with("opencode/skills/carpenter/SKILL.md"),
            "{path}"
        );
        assert!(!version.is_empty());
        assert!(installed);
        assert!(paths
            .xdg_root()
            .unwrap()
            .join("opencode/skills/carpenter/SKILL.md")
            .exists());
        let _ = std::fs::remove_dir_all(paths.root);
    }

    #[test]
    fn register_print_skill_has_no_fs_effect() {
        let paths = testutil::meta_setup();
        let Data::PrintSkill { skill } = register(&paths, true, "carpenter").expect("print") else {
            panic!("PrintSkill");
        };
        assert!(skill.starts_with("---\nname: carpenter\n"));
        assert!(skill.contains("carpenter howto"));
        // no filesystem change
        assert!(!paths
            .xdg_root()
            .unwrap()
            .join("opencode/skills/carpenter/SKILL.md")
            .exists());
        let _ = std::fs::remove_dir_all(paths.root);
    }

    #[test]
    fn register_passes_the_install_name_through() {
        let paths = testutil::meta_setup();
        let Data::Register { path, .. } =
            register(&paths, false, "carpenter-ivan-topic").expect("register")
        else {
            panic!("Register");
        };
        assert!(
            path.ends_with("opencode/skills/carpenter-ivan-topic/SKILL.md"),
            "{path}"
        );
        let _ = std::fs::remove_dir_all(paths.root);
    }
}
