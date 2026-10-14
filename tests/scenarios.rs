//! e2e scenario scripts (`tests/scenarios/*.sh`) — recorded, validated probe
//! sequences from carpenter-dev-validate passes. **Local-only by design**:
//! every test here is `#[ignore]`, so plain `cargo test` (and therefore CI)
//! skips them; the agent runs them selectively before manual fault-hunting:
//!
//! ```text
//! cargo test --test scenarios scenario_self_guard -- --ignored --nocapture  # one
//! cargo test --test scenarios -- --ignored --nocapture                      # all
//! ```
//!
//! The runner gives each script a hermetic scratch env (`HOME`/
//! `XDG_CONFIG_HOME` pinned inside a per-test tmp dir, `CARPENTER_BIN` = the
//! built binary, `SCENARIO_HOME` = the scratch), a null stdin (confirmation
//! reads hit deterministic EOF), and a per-script timeout: a hang fails the
//! test, never wedges the suite. Scripts must never touch anything outside
//! the scratch — no real `HOME`, no real skills, no real config.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::unique;

const SCRIPT_TIMEOUT: Duration = Duration::from_secs(600);

/// name → script file (`tests/scenarios/<file>`). Register a new scenario
/// here plus its `#[ignore]` test below — the only wiring a script needs.
const SCENARIOS: &[(&str, &str)] = &[
    ("self_guard", "self-guard.sh"),
    ("self_skill_leg", "self-skill-leg.sh"),
    ("self_purge_target", "self-purge-target.sh"),
    ("install_identity", "install-identity.sh"),
    ("study_chain", "study-chain.sh"),
];

fn script_path(file: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("scenarios")
        .join(file);
    assert!(p.is_file(), "scenario script missing: {}", p.display());
    p
}

struct Outcome {
    success: bool,
    code: Option<i32>,
    transcript: String,
    timed_out: bool,
}

fn run_script(script: &Path, home: &Path) -> Outcome {
    let mut child = Command::new("sh")
        .arg(script)
        .env("CARPENTER_BIN", env!("CARGO_BIN_EXE_carpenter"))
        .env("SCENARIO_HOME", home)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn scenario script");
    let deadline = Instant::now() + SCRIPT_TIMEOUT;
    loop {
        match child.try_wait().expect("poll scenario script") {
            Some(status) => {
                let out = child.wait_with_output().expect("reap scenario script");
                return Outcome {
                    success: status.success(),
                    code: status.code(),
                    transcript: format!(
                        "{}{}",
                        String::from_utf8_lossy(&out.stdout),
                        String::from_utf8_lossy(&out.stderr)
                    ),
                    timed_out: false,
                };
            }
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let out = child.wait_with_output().expect("reap killed scenario");
                return Outcome {
                    success: false,
                    code: None,
                    transcript: format!(
                        "{}{}",
                        String::from_utf8_lossy(&out.stdout),
                        String::from_utf8_lossy(&out.stderr)
                    ),
                    timed_out: true,
                };
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

/// Run one scenario against a fresh scratch dir; panic with the transcript on
/// any failure (a hang is a failure, never a stuck suite).
fn run_scenario(name: &str) {
    let (_, file) = SCENARIOS
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("unknown scenario {name:?} (not in SCENARIOS)"));
    let script = script_path(file);
    let home = unique("scenario");
    std::fs::create_dir_all(&home).expect("scenario scratch dir");
    let outcome = run_script(&script, &home);
    if outcome.success {
        let _ = std::fs::remove_dir_all(&home);
        return;
    }
    let why = if outcome.timed_out {
        format!("timed out after {SCRIPT_TIMEOUT:?}")
    } else {
        format!("failed (exit {:?})", outcome.code)
    };
    panic!("scenario {name} {why}:\n{}", outcome.transcript);
}

#[test]
#[ignore]
fn scenario_self_guard() {
    run_scenario("self_guard");
}

#[test]
#[ignore]
fn scenario_self_skill_leg() {
    run_scenario("self_skill_leg");
}

#[test]
#[ignore]
fn scenario_self_purge_target() {
    run_scenario("self_purge_target");
}

#[test]
#[ignore]
fn scenario_install_identity() {
    run_scenario("install_identity");
}

#[test]
#[ignore]
fn scenario_study_chain() {
    run_scenario("study_chain");
}
