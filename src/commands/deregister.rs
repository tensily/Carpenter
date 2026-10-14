//! `deregister` — remove the carpenter `SKILL.md` (+ dir if empty) and the
//! carpenter allow permission key.

use crate::core::confirm;
use crate::core::error::CarpenterError;
use crate::core::skill::{self, App};
use crate::core::store::Paths;
use crate::models::Data;

/// `deregister [--app opencode] [--yes]`. `name` is the skill name / install
/// tag (adr/027) — the caller resolves it via
/// [`store::install_name`](crate::core::store::install_name). Prints a plan
/// (the machine-global skill path the app reads) and asks for confirmation
/// unless `--yes` (adr/019 revision); a declined or absent confirmation stream
/// is a `Conflict` error envelope — the skill is the recovery twin of
/// `register`, but removing it is still destructive.
pub fn deregister(paths: &Paths, app: &str, yes: bool, name: &str) -> Result<Data, CarpenterError> {
    let app = App::parse(app)?;
    let skill_path = app.skill_path(paths.xdg_root()?, paths.home_dir()?, name);
    if !yes {
        let plan = format!(
            "deregistration plan for {name:?}\n  skill: {} ({} reads it from this machine-global path)\n  permission key: removed\n",
            skill_path.display(),
            app.name()
        );
        confirm::confirm(&plan)?;
    }
    let d = skill::deregister(app, paths.xdg_root()?, paths.home_dir()?, name)?;
    Ok(Data::Deregister {
        app: d.app,
        path: d.path,
        removed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::testutil;

    #[test]
    fn deregister_ok_after_register() {
        let paths = testutil::meta_setup();
        crate::commands::register::register(&paths, "opencode", false, "carpenter")
            .expect("register");
        let Data::Deregister { app, removed, path } =
            deregister(&paths, "opencode", true, "carpenter").expect("deregister")
        else {
            panic!("Deregister");
        };
        assert_eq!(app, "opencode");
        assert!(removed);
        assert!(path.ends_with("opencode/skills/carpenter/SKILL.md"));
        assert!(!paths
            .xdg_root()
            .unwrap()
            .join("opencode/skills/carpenter/SKILL.md")
            .exists());
        let _ = std::fs::remove_dir_all(paths.root);
    }

    #[test]
    fn deregister_not_found_when_absent() {
        let paths = testutil::meta_setup();
        let err = deregister(&paths, "opencode", true, "carpenter").unwrap_err();
        assert!(matches!(err, CarpenterError::NotFound(_)));
        let _ = std::fs::remove_dir_all(paths.root);
    }
}
