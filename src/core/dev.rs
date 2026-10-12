//! Dev-build-only helpers (adr/016). Compiled exclusively under the `dev`
//! feature, so none of this surface reaches a release binary, the `--help`
//! scrape, the generated howto, or the inlined `SKILL.md`.
//!
//! The `--capture-example <PATH>` global flag (added in `app::cli` under the
//! same feature) drives the authoring loop: run a command for real, capture its
//! envelope, and write the worked-example atom ([adr/007](../../docs/adr/007-compile-enforced-command-docs.md)
//! update) straight to `docs/examples/<module>/<fn>.md`. That file is the single
//! source scraped into the howto, so the loop closes by running `cargo xtask
//! build` (strict) — which now passes because the atom (and a paired `#[test]`)
//! exist.
//!
//! The one piece the loop cannot author is the behavioral note (it is not
//! derivable from a run); it is emitted as a TODO for a human/LLM to fill.

use std::path::{Path, PathBuf};

use clap::ArgMatches;
use serde_json::json;

use crate::core::error::CarpenterError;
use crate::core::exec;
use crate::core::skill;
use crate::core::store;
use crate::models::dev::DevCheckItem;
use crate::models::Data;

/// The dev-only global flag name (`--capture-example <PATH>`).
pub const CAPTURE_FLAG: &str = "capture-example";

/// Build the worked-example atom for a real invocation and write it to
/// `out_path`. `envelope` is the already-rendered stdout string for the run;
/// `matches` is the top-level clap matches (used to recover the `--spec` file
/// content for the atom's YAML block). Errors are swallowed (best-effort): the
/// command's own envelope on stdout is authoritative; capture is a convenience.
pub fn write_capture_example(matches: &ArgMatches, out_path: &str, envelope: &str) {
    let invocation = reconstruct_invocation();
    let spec = find_spec_file_content(matches);
    let atom = assemble_atom(&invocation, spec.as_deref(), envelope);
    let _ = store::atomic_write(Path::new(out_path), atom.as_bytes());
}

/// Rebuild the `carpenter …` invocation line from `std::env::args`, dropping the
/// `--capture-example <PATH>` flag (and its value) so the atom shows the command
/// the agent will actually re-run.
fn reconstruct_invocation() -> String {
    let args: Vec<String> = std::env::args().collect();
    filter_capture_from_argv(&args)
}

/// Pure core of [`reconstruct_invocation`]: strip arg[0], the capture flag, and
/// its value; prefix `carpenter `. Separated for unit testing (argv is
/// environment-dependent).
fn filter_capture_from_argv(args: &[String]) -> String {
    let mut keep: Vec<&str> = Vec::new();
    let mut skip_next = false;
    let flag = format!("--{CAPTURE_FLAG}");
    let flag_eq = format!("--{CAPTURE_FLAG}=");
    for (i, arg) in args.iter().enumerate() {
        if i == 0 {
            continue; // binary name → replaced by the literal `carpenter`
        }
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == &flag {
            skip_next = true; // space-separated value
            continue;
        }
        if arg.starts_with(&flag_eq) {
            continue; // `=`-joined value
        }
        keep.push(arg.as_str());
    }
    format!("carpenter {}", keep.join(" "))
}

/// Walk the subcommand matches to find a `--spec <FILE>` whose value is a real
/// path (not `-` for stdin), and read it. Returns `None` for commands with no
/// `--spec` or for stdin specs (the atom then omits the YAML block).
fn find_spec_file_content(matches: &ArgMatches) -> Option<String> {
    let path = find_spec_value(matches)?;
    if path == "-" {
        return None;
    }
    std::fs::read_to_string(&path).ok()
}

/// Recursively locate the first `spec` arg value anywhere in the match tree.
/// Uses `try_get_one` because `spec` is registered per-leaf, not globally:
/// `get_one` would panic on a level where the id is absent.
fn find_spec_value(matches: &ArgMatches) -> Option<String> {
    if let Ok(Some(v)) = matches.try_get_one::<String>("spec") {
        return Some(v.clone());
    }
    if let Some((_, sub)) = matches.subcommand() {
        return find_spec_value(sub);
    }
    None
}

/// Compose the worked-example markdown atom (the exact shape `xtask gen-howto`
/// embeds verbatim from `docs/examples/`).
fn assemble_atom(invocation: &str, spec: Option<&str>, envelope: &str) -> String {
    let mut out = String::new();
    out.push_str("**example:**\n\n```sh\n");
    out.push_str(invocation);
    out.push_str("\n```\n\n");
    if let Some(s) = spec {
        out.push_str("Input spec (`--spec <FILE|->`):\n```yaml\n");
        out.push_str(s.trim_end());
        out.push_str("\n```\n\n");
    }
    out.push_str("Result (one envelope on stdout):\n```json\n");
    out.push_str(envelope);
    out.push_str("\n```\n\n");
    out.push_str("<!-- TODO: author the behavioral note -->\n");
    out
}

// ---- sandbox lifecycle (`carpenter dev check|setup|clean`, adr/016) ----

/// The validation sandbox directory name (cwd-relative). Fixed convention so
/// `setup`/`clean` and the agent's `--root`/`HOME` overrides agree.
const SANDBOX: &str = ".sandbox";

/// `carpenter dev check`: probe the prerequisites the validation loop needs.
/// Today: `uv` (reuses [`exec::uv_available`]). A missing prerequisite is env
/// state, not a carpenter error — the envelope is `status:ok` with a `checks[]`
/// array the caller inspects. Future prereqs append to `checks`.
pub fn check() -> Result<Data, CarpenterError> {
    let uv_ok = exec::uv_available();
    let detail = if uv_ok {
        exec::run_uv(&["--version"], Path::new("."))
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| String::from("uv present"))
    } else {
        String::from("not on PATH")
    };
    Ok(Data::DevCheck {
        checks: vec![DevCheckItem {
            name: String::from("uv"),
            ok: uv_ok,
            detail,
        }],
    })
}

/// `carpenter dev setup`: create the validation sandbox at `./.sandbox`
/// (cwd-relative). Idempotent (`created:false` if it already existed).
pub fn setup() -> Result<Data, CarpenterError> {
    let dir = sandbox_path()?;
    let existed = dir.exists();
    std::fs::create_dir_all(&dir).map_err(store::io_to_store)?;
    Ok(Data::DevSetup {
        path: dir.display().to_string(),
        created: !existed,
    })
}

/// `carpenter dev clean`: remove the validation sandbox at `./.sandbox`.
/// Idempotent (`removed:false` if it was already absent).
pub fn clean() -> Result<Data, CarpenterError> {
    let dir = sandbox_path()?;
    if !dir.exists() {
        return Ok(Data::DevClean {
            removed: false,
            path: dir.display().to_string(),
        });
    }
    std::fs::remove_dir_all(&dir).map_err(store::io_to_store)?;
    Ok(Data::DevClean {
        removed: true,
        path: dir.display().to_string(),
    })
}

/// Resolve `<cwd>/.sandbox`, mapping a cwd failure to a `StoreError`.
fn sandbox_path() -> Result<PathBuf, CarpenterError> {
    std::env::current_dir()
        .map(|d| d.join(SANDBOX))
        .map_err(|e| CarpenterError::StoreError(format!("current_dir failed: {e}")))
}

// ---- local skill lifecycle (`carpenter dev register|upgrade`, adr/016) ----
//
// These mirror the release `register`/`upgrade` exactly in name, behavior, and
// envelope shape — only the *target* differs (the repo's `.opencode/` instead of
// the global `~/.config/opencode/`) and the *build* (`--features dev` instead of
// `--release`). Same semantics across stages.

/// The local opencode root (`<cwd>/.opencode`).
fn opencode_root() -> Result<PathBuf, CarpenterError> {
    Ok(current_dir()?.join(".opencode"))
}

/// `carpenter dev register`: render the carpenter skill and write it to the
/// repo's `.opencode/skills/carpenter/SKILL.md`. Idempotent; re-run to refresh.
/// Mirrors release `register`'s skill write; the permission auto-allow is
/// intentionally NOT merged here (it would dirty the tracked `opencode.json` —
/// the dev-validate agent already carries `skill: allow`, and a globally
/// `register`-ed carpenter skill covers the `carpenter` name for other agents).
pub fn register_local() -> Result<Data, CarpenterError> {
    let root = opencode_root()?;
    let skill_dir = root.join("skills").join(skill::NAME);
    std::fs::create_dir_all(&skill_dir).map_err(store::io_to_store)?;
    let skill_path = skill_dir.join("SKILL.md");
    let content = skill::render(skill::NAME)?;
    store::atomic_write(&skill_path, content.as_bytes())?;
    Ok(Data::Register {
        app: String::from("opencode"),
        path: skill_path.display().to_string(),
        version: env!("CARGO_PKG_VERSION").into(),
        installed: true,
    })
}

/// `carpenter dev upgrade`: rebuild the dev binary (`cargo build --features dev`
/// from `cwd`) and refresh the local skill via [`register_local`]. The dev analog
/// of release `upgrade` (rebuild + refresh skill) — the binary lands in
/// `target/debug/` (no install step).
pub fn upgrade_local() -> Result<Data, CarpenterError> {
    let cwd = current_dir()?;
    exec::run_cargo_or_store(&["build", "--features", "dev"], &cwd)?;
    let bin = cwd.join("target").join("debug").join("carpenter");
    let r = register_local()?;
    let Data::Register {
        path: skill_path, ..
    } = r
    else {
        unreachable!("register_local returns Data::Register");
    };
    Ok(Data::Upgrade {
        upgraded: true,
        version: env!("CARGO_PKG_VERSION").into(),
        bin: bin.display().to_string(),
        source: cwd.display().to_string(),
        skill: Some(json!({
            "refreshed": true,
            "app": "opencode",
            "path": skill_path,
        })),
    })
}

/// Resolve `current_dir`, mapping a failure to a `StoreError`.
fn current_dir() -> Result<PathBuf, CarpenterError> {
    std::env::current_dir()
        .map_err(|e| CarpenterError::StoreError(format!("current_dir failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atom_with_spec_has_all_blocks() {
        let a = assemble_atom(
            "carpenter -c ds lesson create --spec lesson.yaml",
            Some("title: X\nslug: x\n"),
            r#"{"status":"ok","data":{}}"#,
        );
        assert!(a.contains("```sh\ncarpenter -c ds lesson create --spec lesson.yaml\n```"));
        assert!(a.contains("```yaml\ntitle: X\nslug: x\n```"));
        assert!(a.contains("```json\n{\"status\":\"ok\",\"data\":{}}\n```"));
        assert!(a.contains("TODO: author the behavioral note"));
    }

    #[test]
    fn atom_without_spec_omits_yaml_block() {
        let a = assemble_atom("carpenter course list", None, r#"{"status":"ok"}"#);
        assert!(!a.contains("```yaml"));
        assert!(a.contains("```sh\ncarpenter course list\n```"));
        assert!(a.contains("```json"));
    }

    #[test]
    fn filter_strips_space_separated_and_equals_forms() {
        let argv: Vec<String> = [
            "target/debug/carpenter",
            "-c",
            "ds",
            "lesson",
            "create",
            "--spec",
            "lesson.yaml",
            "--capture-example",
            "docs/examples/lesson/create.md",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            filter_capture_from_argv(&argv),
            "carpenter -c ds lesson create --spec lesson.yaml"
        );

        let argv2: Vec<String> = ["carpenter", "course", "list", "--capture-example=out.md"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(filter_capture_from_argv(&argv2), "carpenter course list");
    }

    // ---- sandbox lifecycle ----

    #[test]
    fn check_reports_uv() {
        let Data::DevCheck { checks } = check().expect("check") else {
            panic!("expected DevCheck");
        };
        let uv = checks.iter().find(|c| c.name == "uv").expect("a uv check");
        // This machine has uv installed (see core::exec::tests).
        assert!(uv.ok, "uv reported absent: {}", uv.detail);
        assert!(!uv.detail.is_empty());
    }

    #[test]
    fn setup_then_clean_roundtrip() {
        // Run against a throwaway cwd so we never touch the real repo root.
        let tmp = std::env::temp_dir().join(format!(
            "carpenter-dev-sandbox-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp).unwrap();

        // setup creates it.
        let Data::DevSetup { path, created } = setup().expect("setup") else {
            panic!("expected DevSetup");
        };
        assert!(created, "first setup should create");
        assert!(path.ends_with(".sandbox"));
        assert!(std::path::Path::new(&path).exists());

        // second setup is idempotent.
        let Data::DevSetup { created, .. } = setup().expect("setup 2") else {
            panic!("expected DevSetup");
        };
        assert!(!created, "second setup should report created:false");

        // clean removes it.
        let Data::DevClean { removed, .. } = clean().expect("clean") else {
            panic!("expected DevClean");
        };
        assert!(removed, "clean should remove an existing sandbox");

        // clean again is idempotent.
        let Data::DevClean { removed, .. } = clean().expect("clean 2") else {
            panic!("expected DevClean");
        };
        assert!(!removed, "second clean should report removed:false");

        std::env::set_current_dir(prev).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn register_local_writes_skill() {
        let tmp = std::env::temp_dir().join(format!(
            "carpenter-dev-register-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp).unwrap();

        let Data::Register {
            app,
            path,
            version,
            installed,
        } = register_local().expect("register_local")
        else {
            panic!("expected Data::Register");
        };
        assert_eq!(app, "opencode");
        assert!(
            path.ends_with(".opencode/skills/carpenter/SKILL.md"),
            "{path}"
        );
        assert!(!version.is_empty());
        assert!(installed);
        // SKILL.md written (rendered skill, not empty); no opencode.json change.
        let skill_text = std::fs::read_to_string(&path).unwrap();
        assert!(
            skill_text.starts_with("---\nname: carpenter\n"),
            "{skill_text}"
        );
        assert!(!tmp.join(".opencode/opencode.json").exists());

        std::env::set_current_dir(prev).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
}
