//! Output examples for `register`/`deregister` (docs/specs/21-register-deregister.md).

/// Representative examples for spec generation (adr/008).
pub mod examples {
    use crate::models::Data;

    /// `(cmd, input, note, data)` rows for the register/deregister output contract.
    pub fn rows() -> Vec<(&'static str, &'static str, &'static str, Data)> {
        vec![
            (
                "self register [--app opencode]",
                "--app (default `opencode`)",
                "writes `SKILL.md` + merges `\"skill\":{\"carpenter\":\"allow\"}`; `agents` ⇒ `ValidationError`",
                Data::Register {
                    app: String::from("opencode"),
                    path: String::from("~/.config/opencode/skills/carpenter/SKILL.md"),
                    version: env!("CARGO_PKG_VERSION").into(),
                    installed: true,
                },
            ),
            (
                "self register --app claude-code",
                "--app",
                "writes `SKILL.md` into `~/.claude/skills/` (auto-discovered — no permission merge)",
                Data::Register {
                    app: String::from("claude-code"),
                    path: String::from("~/.claude/skills/carpenter/SKILL.md"),
                    version: env!("CARGO_PKG_VERSION").into(),
                    installed: true,
                },
            ),
            (
                "self register --print-skill",
                "--app",
                "prints the rendered `SKILL.md` bytes; no filesystem change",
                Data::PrintSkill {
                    skill: String::from("…"),
                },
            ),
            (
                "self deregister [--app opencode]",
                "--app (default `opencode`)",
                "removes `SKILL.md` (+ dir if empty) + the `carpenter` allow key (apps that have one); `NotFound` if absent",
                Data::Deregister {
                    app: String::from("opencode"),
                    path: String::from("~/.config/opencode/skills/carpenter/SKILL.md"),
                    removed: true,
                },
            ),
        ]
    }
}
