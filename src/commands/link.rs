//! `link` — emit the carpenter manifest for a future CLI registry. Compute-only;
//! no filesystem effect.

use crate::core::error::CarpenterError;
use crate::core::store::Paths;
use crate::models::Data;

/// One-line manifest summary (authored; the rest is derived).
const SUMMARY: &str = "Agent-driven CLI that builds Python/Jupyter learning material.";

/// Short howto excerpt (defers to `carpenter howto` for the real surface).
const HOWTO_EXCERPT: &str = "Run `carpenter howto` for the full, always-current command manual.";

/// `link register`.
pub fn register(_paths: &Paths) -> Result<Data, CarpenterError> {
    let bin = std::env::current_exe()
        .map_err(|e| CarpenterError::StoreError(format!("current_exe failed: {e}")))?;
    let commands: Vec<String> = crate::app::cli()
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();
    Ok(Data::LinkRegister {
        name: String::from("carpenter"),
        version: env!("CARGO_PKG_VERSION").into(),
        bin: bin.display().to_string(),
        summary: SUMMARY.into(),
        howto_excerpt: HOWTO_EXCERPT.into(),
        commands,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::testutil;

    #[test]
    fn register_emits_manifest_with_command_surface() {
        let paths = testutil::meta_setup();
        let Data::LinkRegister {
            name,
            version,
            commands,
            summary,
            ..
        } = register(&paths).expect("register")
        else {
            panic!("LinkRegister");
        };
        assert_eq!(name, "carpenter");
        assert!(!version.is_empty());
        assert!(!summary.is_empty());
        assert!(commands.contains(&String::from("course")), "{commands:?}");
        assert!(commands.contains(&String::from("self")), "{commands:?}");
        let _ = std::fs::remove_dir_all(&paths.root);
    }
}
