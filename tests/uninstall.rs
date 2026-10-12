//! e2e: `uninstall` as a subprocess against the real binary — the self-delete
//! path (uninstalling the running installed copy), the `NotFound` contract,
//! `--purge-config`, and the installer pairing (`scripts/install.sh` →
//! `uninstall` fully reverses the footprint). Sandbox: per-test `HOME` +
//! `XDG_CONFIG_HOME`, so nothing touches real user config on Linux or macOS.

mod common;

use std::path::Path;
use std::process::Command;

use common::{config_file, opencode_paths, release_fixture, sandboxed, unique};
use serde_json::Value;

fn carpenter_bin() -> &'static str {
    env!("CARGO_BIN_EXE_carpenter")
}

fn setup(tag: &str) -> std::path::PathBuf {
    let root = unique(tag);
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// Run sandboxed carpenter; return (exit-ok, parsed stdout envelope).
/// Retries the transient `ETXTBSY` (a just-written executable can race exec).
fn run(cmd: &mut Command) -> (bool, Value) {
    let mut out = cmd.output();
    for _ in 0..20 {
        match &out {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(50));
                out = cmd.output();
            }
            _ => break,
        }
    }
    let out = out.unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let env: Value = serde_json::from_str(stdout.trim()).unwrap_or(Value::Null);
    (out.status.success(), env)
}

/// Write a config file so the keep/purge assertions are meaningful. Writes the
/// stock path; tests that purge also cover the tagged identity path when their
/// installed copy runs under a non-stock name.
fn write_config(root: &Path) {
    let cfg = config_file(root);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    std::fs::write(&cfg, br#"{"bin_dir":"/nonexistent-default"}"#).unwrap();
}

/// The config file for the tagged identity (`carpenter_nightly` runs as
/// `carpenter-nightly`, adr/027).
fn tagged_config_file(root: &Path) -> std::path::PathBuf {
    common::config_root(root)
        .join("carpenter-nightly")
        .join("config.json")
}

/// Install a copy of the real binary into `<root>/bin/carpenter`.
fn install_copy(root: &Path) -> std::path::PathBuf {
    let bin = root.join("bin").join("carpenter_nightly");
    std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
    std::fs::copy(carpenter_bin(), &bin).unwrap();
    bin
}

fn opencode_has_carpenter_key(perms: &Path) -> bool {
    let text = std::fs::read_to_string(perms).unwrap_or_default();
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    v.get("permission")
        .and_then(|p| p.get("skill"))
        .map(|s| s.get("carpenter").is_some() || s.get("carpenter-nightly").is_some())
        .unwrap_or(false)
}

#[test]
fn uninstall_self_deletes_running_binary() {
    let root = setup("uninstall-self");
    let bin = install_copy(&root);
    let (skill_md, perms) = opencode_paths(&root);
    write_config(&root);

    // register via the installed copy (skill + permission land in the sandbox)
    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("register")
        .arg("--app")
        .arg("opencode"));
    assert!(ok, "{env}");
    assert!(skill_md.is_file(), "skill at {}", skill_md.display());
    assert!(opencode_has_carpenter_key(&perms), "permission key present");

    // uninstall FROM the installed copy — the self-delete path
    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("uninstall")
        .arg("--yes")
        .arg("--bin-dir")
        .arg(bin.parent().unwrap()));
    assert!(ok, "{env}");
    assert_eq!(env["status"], "ok", "{env}");
    assert_eq!(env["data"]["uninstalled"], true, "{env}");
    assert_eq!(env["data"]["bin"], bin.to_str().unwrap(), "{env}");
    assert_eq!(env["data"]["skill"][0]["app"], "opencode", "{env}");
    assert_eq!(env["data"]["skill"][0]["removed"], true, "{env}");
    assert_eq!(env["data"]["config_purged"], false, "{env}");
    assert!(!bin.exists(), "binary self-deleted");
    assert!(!skill_md.exists(), "skill removed");
    assert!(config_file(&root).is_file(), "config kept by default");
    let perms_text = std::fs::read_to_string(&perms).unwrap_or_default();
    assert!(
        serde_json::from_str::<Value>(&perms_text).is_ok(),
        "opencode.json stays valid JSON, got: {perms_text}"
    );
    assert!(
        !opencode_has_carpenter_key(&perms),
        "permission key removed"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn uninstall_twice_is_notfound() {
    let root = setup("uninstall-twice");
    let bin = install_copy(&root);
    let bin_dir = bin.parent().unwrap().to_path_buf();

    let (ok, _) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("uninstall")
        .arg("--yes")
        .arg("--bin-dir")
        .arg(&bin_dir));
    assert!(ok);

    // second run (from the still-existing test binary): no skill, no binary →
    // error envelope, exit 1
    let (ok, env) = run(sandboxed(Path::new(carpenter_bin()), &root)
        .arg("self")
        .arg("uninstall")
        .arg("--yes")
        .arg("--bin-dir")
        .arg(&bin_dir));
    assert!(!ok, "{env}");
    assert_eq!(env["status"], "error", "{env}");
    assert_eq!(env["code"], "NotFound", "{env}");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn uninstall_purges_config_with_flag() {
    let root = setup("uninstall-purge");
    let bin = install_copy(&root);
    write_config(&root);
    let tagged_cfg = tagged_config_file(&root);
    std::fs::create_dir_all(tagged_cfg.parent().unwrap()).unwrap();
    std::fs::write(&tagged_cfg, br#"{"bin_dir":"/nonexistent-default"}"#).unwrap();
    let bin_dir = bin.parent().unwrap().to_path_buf();

    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("uninstall")
        .arg("--yes")
        .arg("--bin-dir")
        .arg(&bin_dir)
        .arg("--purge-config"));
    assert!(ok, "{env}");
    assert_eq!(env["data"]["config_purged"], true, "{env}");
    assert!(!tagged_cfg.exists(), "tagged config purged");
    assert!(
        config_file(&root).is_file(),
        "the calling stock install's config survives (purge targets the removed tag only)"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn uninstall_notfound_with_default_bin_dir_when_nothing_installed() {
    // no --bin-dir: default resolves under the sandboxed HOME → nothing there
    let root = setup("uninstall-default");

    let (ok, env) = run(sandboxed(Path::new(carpenter_bin()), &root)
        .arg("self")
        .arg("uninstall")
        .arg("--yes"));
    assert!(!ok, "{env}");
    assert_eq!(env["code"], "NotFound", "{env}");

    let _ = std::fs::remove_dir_all(&root);
}

/// The installer pairing: `scripts/install.sh` (file:// fixture + fake
/// `opencode` on PATH so the auto-register branch runs) → `carpenter uninstall`
/// reverses the full footprint. Requires `sh`/`curl`/`tar`/checksum (Linux +
/// macOS runners); skips gracefully elsewhere.
#[test]
fn install_sh_then_uninstall_reverses_footprint() {
    let Some(target) = common::platform_target() else {
        return;
    };
    if !Command::new("sh").arg("-c").arg("exit 0").status().is_ok() {
        return;
    }
    let fixture = release_fixture(target);
    let root = setup("uninstall-pair");
    let bin_dir = root.join("bin");
    // fake opencode so install.sh takes the auto-register branch deterministically
    let stub = root.join("stub");
    std::fs::create_dir_all(&stub).unwrap();
    std::fs::write(stub.join("opencode"), "#!/bin/sh\nexit 0\n").unwrap();
    unix_chmod(&stub.join("opencode"));

    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("install.sh");
    let out = Command::new("sh")
        .arg(&script)
        .env("HOME", &root)
        .env("XDG_CONFIG_HOME", common::config_root(&root))
        .env(
            "CARPENTER_DOWNLOAD_BASE",
            format!("file://{}", fixture.display()),
        )
        .env("CARPENTER_INSTALL_DIR", &bin_dir)
        .env(
            "PATH",
            format!(
                "{}:{}",
                stub.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "install.sh failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let bin = bin_dir.join("carpenter_nightly");
    let (skill_md, perms) = opencode_paths(&root);
    assert!(
        bin.is_file(),
        "installer placed binary at {}",
        bin.display()
    );
    assert!(skill_md.is_file(), "installer registered skill");
    assert!(opencode_has_carpenter_key(&perms), "permission key present");

    // uninstall from the installed copy — reverses the installer footprint
    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("uninstall")
        .arg("--yes")
        .arg("--bin-dir")
        .arg(&bin_dir));
    assert!(ok, "{env}");
    assert!(!bin.exists(), "binary removed");
    assert!(!skill_md.exists(), "skill removed");
    assert!(
        !opencode_has_carpenter_key(&perms),
        "permission key removed"
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&fixture);
}

/// The installer also detects claude code (`claude` on PATH) and registers into
/// `~/.claude/skills/` (non-interactive lane: auto-register every detection);
/// `uninstall` reverses that footprint too.
#[test]
fn install_sh_detects_and_registers_claude_code() {
    let Some(target) = common::platform_target() else {
        return;
    };
    if !Command::new("sh").arg("-c").arg("exit 0").status().is_ok() {
        return;
    }
    let fixture = release_fixture(target);
    let root = setup("uninstall-claude");
    let bin_dir = root.join("bin");
    let stub = root.join("stub");
    std::fs::create_dir_all(&stub).unwrap();
    std::fs::write(stub.join("claude"), "#!/bin/sh\nexit 0\n").unwrap();
    unix_chmod(&stub.join("claude"));

    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("install.sh");
    let out = Command::new("sh")
        .arg(&script)
        .env("HOME", &root)
        .env("XDG_CONFIG_HOME", common::config_root(&root))
        .env(
            "CARPENTER_DOWNLOAD_BASE",
            format!("file://{}", fixture.display()),
        )
        .env("CARPENTER_INSTALL_DIR", &bin_dir)
        .env(
            "PATH",
            format!(
                "{}:{}",
                stub.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "install.sh failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let bin = bin_dir.join("carpenter_nightly");
    let skill_md = root
        .join(".claude")
        .join("skills")
        .join("carpenter-nightly")
        .join("SKILL.md");
    assert!(
        bin.is_file(),
        "installer placed binary at {}",
        bin.display()
    );
    assert!(
        skill_md.is_file(),
        "installer registered claude-code skill at {}",
        skill_md.display()
    );

    // uninstall from the installed copy — reverses the claude-code footprint
    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("uninstall")
        .arg("--yes")
        .arg("--bin-dir")
        .arg(&bin_dir));
    assert!(ok, "{env}");
    assert!(!bin.exists(), "binary removed");
    assert!(!skill_md.exists(), "claude-code skill removed");

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&fixture);
}

/// Without `--yes` and with no interactive stdin (`Command::output` closes it),
/// the confirmation reads EOF and the removal aborts with a `Conflict`
/// envelope — and nothing is removed (adr/019 revision).
#[test]
fn uninstall_without_yes_aborts_when_non_interactive() {
    let root = setup("uninstall-noconfirm");
    let bin = install_copy(&root);
    let (skill_md, _) = opencode_paths(&root);
    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("register")
        .arg("--app")
        .arg("opencode"));
    assert!(ok, "{env}");

    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("uninstall")
        .arg("--bin-dir")
        .arg(bin.parent().unwrap()));
    assert!(!ok, "{env}");
    assert_eq!(env["status"], "error", "{env}");
    assert_eq!(env["code"], "Conflict", "{env}");
    assert!(bin.exists(), "binary untouched without confirmation");
    assert!(skill_md.exists(), "skill untouched without confirmation");

    let _ = std::fs::remove_dir_all(&root);
}

/// Same non-interactive abort for the `remove` and `deregister` surfaces —
/// the whole destructive self-group shares the confirmation contract.
#[test]
fn remove_and_deregister_abort_without_yes_when_non_interactive() {
    let root = setup("remove-noconfirm");
    let bin = install_copy(&root);
    let (skill_md, _) = opencode_paths(&root);
    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("register")
        .arg("--app")
        .arg("opencode"));
    assert!(ok, "{env}");

    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("remove")
        .arg("carpenter-nightly")
        .arg("--bin-dir")
        .arg(bin.parent().unwrap()));
    assert!(!ok, "{env}");
    assert_eq!(env["code"], "Conflict", "{env}");
    assert!(bin.exists(), "binary untouched without confirmation");
    assert!(skill_md.exists(), "skill untouched without confirmation");

    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("deregister")
        .arg("--app")
        .arg("opencode"));
    assert!(!ok, "{env}");
    assert_eq!(env["code"], "Conflict", "{env}");
    assert!(skill_md.exists(), "skill untouched without confirmation");

    let _ = std::fs::remove_dir_all(&root);
}

/// A piped explicit `y` satisfies the confirmation — the interactive path,
/// driven through stdin (`remove` variant).
#[test]
fn remove_accepts_piped_confirmation() {
    use std::io::Write;
    use std::process::Stdio;

    let root = setup("remove-piped-yes");
    let bin = install_copy(&root);
    let (skill_md, _) = opencode_paths(&root);
    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("register")
        .arg("--app")
        .arg("opencode"));
    assert!(ok, "{env}");

    let mut child = sandboxed(&bin, &root)
        .arg("self")
        .arg("remove")
        .arg("carpenter-nightly")
        .arg("--bin-dir")
        .arg(bin.parent().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"y\n").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let env: Value = serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).unwrap();
    assert_eq!(env["status"], "ok", "{env}");
    assert_eq!(env["data"]["removed"], true, "{env}");
    assert!(!bin.exists(), "binary removed after piped confirmation");
    assert!(!skill_md.exists(), "skill removed after piped confirmation");

    let _ = std::fs::remove_dir_all(&root);
}

/// `--purge-config` on `self remove <tag>` purges the removed tag's config —
/// never the calling install's (adr/019 revision).
#[test]
fn remove_purges_only_the_target_tags_config() {
    let root = setup("remove-purge-target");
    let bin = install_copy(&root);
    let tagged_cfg = tagged_config_file(&root);
    std::fs::create_dir_all(tagged_cfg.parent().unwrap()).unwrap();
    std::fs::write(&tagged_cfg, br#"{"bin_dir":"/nonexistent-default"}"#).unwrap();
    write_config(&root);

    let (ok, env) = run(sandboxed(&bin, &root)
        .arg("self")
        .arg("remove")
        .arg("carpenter-nightly")
        .arg("--yes")
        .arg("--bin-dir")
        .arg(bin.parent().unwrap())
        .arg("--purge-config"));
    assert!(ok, "{env}");
    assert_eq!(env["data"]["config_purged"], true, "{env}");
    assert!(!tagged_cfg.exists(), "tagged config purged");
    assert!(
        config_file(&root).is_file(),
        "the calling stock install's config survives"
    );

    let _ = std::fs::remove_dir_all(&root);
}

fn unix_chmod(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).unwrap();
}
