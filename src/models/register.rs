//! Output examples for `register`/`deregister` (docs/specs/21-register-deregister.md).

/// Representative examples for spec generation (adr/008).
pub mod examples {
    use crate::models::Data;

    /// `(cmd, input, note, data)` rows for the register/deregister output contract.
    pub fn rows() -> Vec<(&'static str, &'static str, &'static str, Data)> {
        vec![
            (
                "self register [--print-skill]",
                "no flags (opencode is the only agent app)",
                "writes `SKILL.md` + merges `\"skill\":{\"carpenter\":\"allow\"}` into `opencode.json`",
                Data::Register {
                    app: String::from("opencode"),
                    path: String::from("~/.config/opencode/skills/carpenter/SKILL.md"),
                    version: env!("CARGO_PKG_VERSION").into(),
                    installed: true,
                },
            ),
            (
                "self register --print-skill",
                "--print-skill",
                "prints the rendered `SKILL.md` bytes; no filesystem change",
                Data::PrintSkill {
                    skill: String::from("…"),
                },
            ),
            (
                "self deregister [--yes]",
                "--yes skips the confirmation plan",
                "removes `SKILL.md` (+ dir if empty) + the `carpenter` allow key; `NotFound` if absent",
                Data::Deregister {
                    app: String::from("opencode"),
                    path: String::from("~/.config/opencode/skills/carpenter/SKILL.md"),
                    removed: true,
                },
            ),
        ]
    }
}
