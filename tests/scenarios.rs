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
//! The runner gives each script a hermetic scratch env (`HOME`,
//! `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, `XDG_DATA_HOME` pinned inside a
//! per-test tmp dir, `CARPENTER_INSTALL_NAME` stripped so a dev shell can't
//! override the tagged identity, `CARPENTER_BIN` = the built binary,
//! `SCENARIO_HOME` = the scratch), a null stdin (confirmation reads hit
//! deterministic EOF), and a per-script timeout: the script runs in its own
//! process group, so the timeout kills the whole tree — a hang fails the
//! test, never wedges the suite. Scripts must never touch anything outside
//! the scratch — no real `HOME`, no real skills, no real config.

mod common;

use std::os::unix::process::CommandExt;
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
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("CARPENTER_INSTALL_NAME")
        .process_group(0)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn scenario script");
    // Drain both pipes on threads — a script whose output exceeds the OS pipe
    // buffer would otherwise block on write, `try_wait` would never flip, and
    // the run would burn the whole timeout as a false failure.
    fn drain<I: std::io::Read + Send + 'static>(io: Option<I>) -> std::thread::JoinHandle<String> {
        std::thread::spawn(move || {
            let mut buf = String::new();
            if let Some(mut io) = io {
                let _ = io.read_to_string(&mut buf);
            }
            buf
        })
    }
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let deadline = Instant::now() + SCRIPT_TIMEOUT;
    let (status, timed_out) = loop {
        match child.try_wait().expect("poll scenario script") {
            Some(status) => break (Some(status), false),
            None if Instant::now() >= deadline => {
                // killpg, not kill(): a surviving grandchild (uv, python3)
                // inherits the pipes and would block the joins below past
                // the deadline. ESRCH (group already gone) is fine.
                let _ = unsafe { libc::killpg(child.id() as libc::pid_t, libc::SIGKILL) };
                break (Some(child.wait().expect("reap killed scenario")), true);
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    };
    let transcript = format!(
        "{}{}",
        stdout.join().unwrap_or_default(),
        stderr.join().unwrap_or_default()
    );
    match (status, timed_out) {
        (Some(status), false) => Outcome {
            success: status.success(),
            code: status.code(),
            transcript,
            timed_out: false,
        },
        _ => Outcome {
            success: false,
            code: None,
            transcript,
            timed_out: true,
        },
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
