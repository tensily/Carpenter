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
                "install [--bin-dir <p>]",
                "—",
                "`on_path` = whether `bin_dir` resolves on `$PATH`",
                Data::Install {
                    installed: true,
                    bin: String::from("~/.local/bin/carpenter"),
                    on_path: true,
                },
            ),
            (
                "upgrade [--channel stable|nightly] [--source <p>] [--bin-dir <p>] [--no-skill]",
                "no flag → latest **stable** release; `--channel nightly` → rolling prerelease; `--source` → config `source_dir` → local build",
                "`skill` = per-app refresh outcomes (one per registered app): `[{\"refreshed\":true,\"app\":\"opencode\",…},{\"refreshed\":true,\"app\":\"claude-code\",…}]` · nothing registered ⇒ `{refreshed:false,reason:\"not_registered\",warning:\"…\"}` · `--no-skill` ⇒ `skill:null`",
                Data::Upgrade {
                    upgraded: true,
                    version: env!("CARGO_PKG_VERSION").into(),
                    bin: String::from("~/.local/bin/carpenter"),
                    source: String::from("https://github.com/tensily/Carpenter/releases/latest/download/carpenter-x86_64-unknown-linux-musl.tar.gz"),
                    skill: Some(json!([
                        {"refreshed": true, "app": "opencode", "path": "~/.config/opencode/skills/carpenter/SKILL.md"},
                        {"refreshed": true, "app": "claude-code", "path": "~/.claude/skills/carpenter/SKILL.md"}
                    ])),
                },
            ),
            (
                "uninstall [--bin-dir <p>] [--purge-config]",
                "—",
                "`skill` = per-app removal outcomes (one per registered app): `[{\"removed\":true,\"app\":\"opencode\",…},{\"removed\":true,\"app\":\"claude-code\",…}]` · nothing registered ⇒ `{removed:false,reason:\"not_registered\"}`; `bin:null` when no binary was present; `NotFound` when neither skill nor binary exists",
                Data::Uninstall {
                    uninstalled: true,
                    bin: Some(String::from("~/.local/bin/carpenter")),
                    skill: json!([
                        {"removed": true, "app": "opencode", "path": "~/.config/opencode/skills/carpenter/SKILL.md"},
                        {"removed": true, "app": "claude-code", "path": "~/.claude/skills/carpenter/SKILL.md"}
                    ]),
                    config_purged: false,
                },
            ),
        ]
    }
}
