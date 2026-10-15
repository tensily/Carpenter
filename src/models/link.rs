//! Output examples for `link` (docs/specs/17-link.md).

/// Representative examples for spec generation (adr/008).
pub mod examples {
    use crate::models::Data;

    /// `(cmd, input, note, data)` rows for the `link` output contract.
    pub fn rows() -> Vec<(&'static str, &'static str, &'static str, Data)> {
        vec![(
            "self link",
            "—",
            "emits a manifest for a future CLI registry; no filesystem effect",
            Data::LinkRegister {
                name: String::from("carpenter"),
                version: env!("CARGO_PKG_VERSION").into(),
                bin: String::from("~/.local/bin/carpenter"),
                summary: String::from(
                    "Agent-driven CLI that builds Python/Jupyter learning material.",
                ),
                howto_excerpt: String::from("Run `carpenter howto` for the full command manual."),
                commands: vec![
                    String::from("course"),
                    String::from("lesson"),
                    String::from("plan"),
                    String::from("quiz"),
                    String::from("howto"),
                ],
            },
        )]
    }
}
