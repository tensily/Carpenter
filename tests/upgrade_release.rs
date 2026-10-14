//! e2e: `upgrade` release mode against a `file://` fixture (adr/018) — the
//! fixture reproduces the release layout (`tarball` + `SHA256SUMS`) using the
//! real test binary, then `upgrade` must download-verify-extract-probe-replace
//! and refresh the registered apps' skills.

mod common;

use std::path::Path;

use carpenter::commands::upgrade::upgrade;
use carpenter::core::store::Paths;
use carpenter::models::Data;

use common::{release_fixture, unique};

#[test]
fn upgrade_release_mode_installs_and_refreshes_registered_skill() {
    let Some(target) = common::platform_target() else {
        return; // no published asset on this platform (mapping covered in unit tests)
    };
    let dir = release_fixture(target);
    std::env::set_var(
        "CARPENTER_DOWNLOAD_BASE",
        format!("file://{}", dir.display()),
    );
    let root = unique("root");
    let paths = Paths {
        config_dir: Some(root.join("xdg").join("carpenter")),
        home: Some(root.join("home")),
        root,
    };
    let bin_dir = paths.root.join("bin");
    // registered apps get refreshed; nothing is registered by surprise
    carpenter::commands::register::register(&paths, "opencode", false, "carpenter")
        .expect("pre-register");

    let Data::Upgrade {
        upgraded,
        version,
        bin,
        source,
        skill,
    } = upgrade(
        &paths,
        None,
        Some(bin_dir.to_str().unwrap()),
        false,
        "stable",
        "carpenter",
    )
    .expect("upgrade")
    else {
        panic!("Upgrade");
    };
    assert!(upgraded);
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
    assert!(bin.ends_with("bin/carpenter"), "{bin}");
    assert!(
        source.ends_with(&format!("carpenter-{target}.tar.gz")),
        "{source}"
    );
    assert!(Path::new(&bin).is_file(), "binary at {bin}");
    let outcomes = skill.expect("skill written");
    let outcomes = outcomes
        .as_array()
        .unwrap_or_else(|| panic!("per-app outcomes, got {outcomes}"));
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0]["app"], "opencode");
    assert_eq!(outcomes[0]["refreshed"], true, "{outcomes:?}");
    let skill_md = paths
        .root
        .join("xdg")
        .join("opencode")
        .join("skills")
        .join("carpenter")
        .join("SKILL.md");
    assert!(skill_md.is_file(), "skill at {}", skill_md.display());

    // --no-skill variant against the same fixture: binary replaced, skill null
    let Data::Upgrade { skill, .. } = upgrade(
        &paths,
        None,
        Some(bin_dir.to_str().unwrap()),
        true,
        "stable",
        "carpenter",
    )
    .expect("upgrade again") else {
        panic!("Upgrade");
    };
    assert!(skill.is_none());

    std::env::remove_var("CARPENTER_DOWNLOAD_BASE");
    let _ = std::fs::remove_dir_all(&paths.root);
    let _ = std::fs::remove_dir_all(&dir);
}
