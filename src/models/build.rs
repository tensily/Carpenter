//! Output examples for `build`/`install`/`upgrade`/`uninstall`
//! (docs/specs/18-build-install-upgrade.md).

use serde_json::json;

/// Representative examples for spec generation (adr/008).
pub mod examples {
    use super::json;
    use crate::models::Data;

    /// `(cmd, input, note, data)` rows for the build/install/upgrade output contract.
    pub fn rows() -> Vec<(&'static str, &'static str, &'static str, Data)> {
        vec![
            (
                "build <path>",
                "target dir",
                "scaffolds course.json + course.db + lessons/",
                Data::Build {
                    path: String::from("/courses/ds"),
                    slug: String::from("ds"),
                    created: vec![
                        String::from("course.json"),
                        String::from("course.db"),
                        String::from("lessons/"),
                    ],
                },
            ),
            (
                "self install [--bin-dir <p>]",
                "—",
                "`on_path` = whether `bin_dir` resolves on `$PATH`; `already_present` = the target binary existed before this install",
                Data::Install {
                    installed: true,
                    already_present: false,
                    bin: String::from("~/.local/bin/carpenter"),
                    on_path: true,
                },
            ),
            (
                "self upgrade [--channel stable|nightly] [--source <p>] [--bin-dir <p>] [--no-skill]",
                "no flag → latest **stable** release; `--channel nightly` → rolling prerelease; `--source` → config `source_dir` → local build",
                "`skill` = refresh outcome: `[{\"refreshed\":true,\"app\":\"opencode\",…}]` when registered · nothing registered ⇒ `{refreshed:false,reason:\"not_registered\",warning:\"…\"}` · `--no-skill` ⇒ `skill:null`",
                Data::Upgrade {
                    upgraded: true,
                    version: env!("CARGO_PKG_VERSION").into(),
                    bin: String::from("~/.local/bin/carpenter"),
                    source: String::from("https://github.com/tensily/Carpenter/releases/latest/download/carpenter-x86_64-unknown-linux-musl.tar.gz"),
                    skill: Some(json!([
                        {"refreshed": true, "app": "opencode", "path": "~/.config/opencode/skills/carpenter/SKILL.md"}
                    ])),
                },
            ),
            (
                "self uninstall [--bin-dir <p>] [--purge-config] [--yes]",
                "—",
                "prints a removal plan and confirms on stdin unless `--yes` (decline/EOF ⇒ `Conflict`) · `skill` = removal outcome: `[{\"removed\":true,\"app\":\"opencode\",…}]` when registered · nothing registered ⇒ `{removed:false,reason:\"not_registered\"}` · `bin:null` when no binary was present · `NotFound` when no skill, binary, or data dir exists",
                Data::Uninstall {
                    uninstalled: true,
                    bin: Some(String::from("~/.local/bin/carpenter")),
                    skill: json!([
                        {"removed": true, "app": "opencode", "path": "~/.config/opencode/skills/carpenter/SKILL.md"}
                    ]),
                    config_purged: false,
                },
            ),
        ]
    }
}
