//! `register` — write the carpenter `SKILL.md` into an agent app's skills dir
//! and merge the allow permission entry.
//!
//! `--print-skill` instead emits the rendered bytes (no filesystem change).

use crate::core::error::CarpenterError;
use crate::core::skill::{self, App};
use crate::core::store::Paths;
use crate::models::Data;

/// `register [--app opencode] [--print-skill]`. `name` is the skill name /
/// install tag (adr/027) — the caller resolves it via
/// [`store::install_name`](crate::core::store::install_name).
pub fn register(
    paths: &Paths,
    app: &str,
    print_skill: bool,
    name: &str,
) -> Result<Data, CarpenterError> {
    let app = App::parse(app)?;
    if print_skill {
        return Ok(Data::PrintSkill {
            skill: skill::render(name)?,
        });
    }
    let r = skill::register(app, paths.xdg_root()?, paths.home_dir()?, name)?;
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
        } = register(&paths, "opencode", false, "carpenter").expect("register")
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
        let Data::PrintSkill { skill } =
            register(&paths, "opencode", true, "carpenter").expect("print")
        else {
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
            register(&paths, "opencode", false, "carpenter-ivan-topic").expect("register")
        else {
            panic!("Register");
        };
        assert!(
            path.ends_with("opencode/skills/carpenter-ivan-topic/SKILL.md"),
            "{path}"
        );
        let _ = std::fs::remove_dir_all(paths.root);
    }

    #[test]
    fn register_rejects_unsupported_app() {
        let paths = testutil::meta_setup();
        let err = register(&paths, "agents", false, "carpenter").unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
        let _ = std::fs::remove_dir_all(paths.root);
    }

    #[test]
    fn register_claude_code_ok() {
        let paths = testutil::meta_setup();
        let Data::Register {
            app,
            path,
            installed,
            ..
        } = register(&paths, "claude-code", false, "carpenter").expect("register")
        else {
            panic!("Register");
        };
        assert_eq!(app, "claude-code");
        assert!(installed);
        assert!(
            path.ends_with(".claude/skills/carpenter/SKILL.md"),
            "{path}"
        );
        assert!(paths
            .home_dir()
            .unwrap()
            .join(".claude/skills/carpenter/SKILL.md")
            .exists());
        let _ = std::fs::remove_dir_all(paths.root);
    }
}
