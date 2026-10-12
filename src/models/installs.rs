//! Output types + examples for `installs list`/`installs remove`
//! (docs/specs/24-installs.md).

use serde::Serialize;

/// One carpenter install discovered on this machine (`installs list` row).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct InstallRow {
    /// The install tag (binary stem — `carpenter`, `carpenter-<branch>`).
    pub tag: String,
    /// The binary path (`null` if no binary is present).
    pub bin: Option<String>,
    /// Agent apps holding a registered skill for this tag.
    pub skill_apps: Vec<String>,
    /// The install's data dir (may not exist yet).
    pub data_dir: String,
}

/// Representative examples for spec generation (adr/008).
pub mod examples {
    use super::InstallRow;
    use crate::models::Data;

    /// `(cmd, input, note, data)` rows for the installs output contract.
    pub fn rows() -> Vec<(&'static str, &'static str, &'static str, Data)> {
        vec![
            (
                "installs list",
                "--bin-dir (default config `bin_dir`)",
                "union of `carpenter*` binaries, registered skills, and data dirs; sorted by tag",
                Data::InstallsList {
                    installs: vec![
                        InstallRow {
                            tag: String::from("carpenter"),
                            bin: Some(String::from("~/.local/bin/carpenter")),
                            skill_apps: vec![String::from("opencode"), String::from("claude-code")],
                            data_dir: String::from("~/.config/carpenter"),
                        },
                        InstallRow {
                            tag: String::from("carpenter-ivan-branch-installs"),
                            bin: Some(String::from("~/.local/bin/carpenter-ivan-branch-installs")),
                            skill_apps: vec![String::from("opencode")],
                            data_dir: String::from("~/.config/carpenter-ivan-branch-installs"),
                        },
                    ],
                },
            ),
            (
                "installs remove carpenter-ivan-topic",
                "tag (positional)",
                "same removal as `uninstall`, aimed at any install by tag; `NotFound` when absent",
                Data::InstallsRemove {
                    removed: true,
                    tag: String::from("carpenter-ivan-topic"),
                    bin: Some(String::from("~/.local/bin/carpenter-ivan-topic")),
                    skill: serde_json::json!([
                        {"removed": true, "app": "opencode", "path": "~/.config/opencode/skills/carpenter-ivan-topic/SKILL.md"}
                    ]),
                    config_purged: false,
                },
            ),
        ]
    }
}
