//! e2e: `scripts/install.sh` as a subprocess against a `file://` release
//! fixture — the installer UX contract (adr/024): branded banner, install
//! plan, channel-correct tagline (the stable release's tag-patched copy must
//! drop the `nightly` mark), non-interactive auto-proceed, and the interactive
//! consent gate (declining installs nothing). Sandbox: per-test `HOME` +
//! `XDG_CONFIG_HOME`, so nothing touches real user config. Plain lanes run
//! detached (`setsid`, stdin nulled) so the `/dev/tty` consent gate takes the
//! CI non-interactive path regardless of the runner's terminal.

mod common;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use common::{config_root, platform_target, release_fixture, unique};

fn installer_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("install.sh")
}

fn setup(tag: &str) -> PathBuf {
    let root = unique(tag);
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn unix_chmod(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).unwrap();
}

/// The `PATH` value every installer lane runs under: the stub dir first, so
/// the script's sole app probe (`command -v opencode`) resolves to the stub
/// and never to a real host `opencode`.
fn stub_path(root: &Path) -> String {
    format!(
        "{}:{}",
        root.join("stub").display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

/// Fake `opencode` on PATH so app detection is deterministic. Guards its own
/// invariant: `command -v opencode` under [`stub_path`] must resolve to the
/// stub — if a future edit drops the PATH prepend, the lane fails here
/// immediately instead of silently running against the host's real
/// `opencode` (the hang class behind issue #69).
fn stub_opencode(root: &Path) {
    let stub = root.join("stub");
    std::fs::create_dir_all(&stub).unwrap();
    std::fs::write(stub.join("opencode"), "#!/bin/sh\nexit 0\n").unwrap();
    unix_chmod(&stub.join("opencode"));
    let probe = Command::new("sh")
        .arg("-c")
        .arg("command -v opencode")
        .env("PATH", stub_path(root))
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&probe.stdout).trim(),
        stub.join("opencode").display().to_string(),
        "PATH must resolve `opencode` to the stub, not a host install — \
         otherwise app detection hits the real opencode and a lane can hang \
         on its output"
    );
}

fn run_installer(script: &Path, root: &Path, fixture: &Path) -> std::process::Output {
    run_bounded(
        detached_sh(script)
            .env("HOME", root)
            .env("XDG_CONFIG_HOME", config_root(root))
            .env(
                "CARPENTER_DOWNLOAD_BASE",
                format!("file://{}", fixture.display()),
            )
            .env("CARPENTER_INSTALL_DIR", root.join("bin"))
            .env("PATH", stub_path(root)),
        "install.sh",
    )
}

/// Wall-clock cap on one install.sh run. The script's prompts read `/dev/tty`,
/// so a spawn that inherits the session's controlling terminal — a local
/// `cargo test` from a terminal; CI has no ctty — can block on the
/// developer's keyboard. The cap turns such a regression into a
/// transcript-bearing failure in seconds instead of a silent suite-wide stall.
const INSTALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Whether `setsid` (util-linux) exists. It does not on macOS, where the
/// plain spawn stands in — macOS CI has no ctty either, and
/// [`run_bounded`] still bounds the run.
fn has_setsid() -> bool {
    Command::new("setsid")
        .arg("true")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// The installer launcher: `setsid sh <script>` where possible, else plain
/// `sh <script>`. Two layers keep install.sh deterministic, matching CI (no
/// ctty): the interactive gate itself is decided by piped stdout/stderr
/// (`[ -t 1 ] || [ -t 2 ]` — never a tty here), and setsid detaches the
/// session's ctty so any `read -r answer </dev/tty` that does run fails
/// immediately (no ctty → open gives ENXIO → abort path) instead of blocking
/// on the developer's keyboard. Verified locally: with a ctty, a plain spawn
/// blocks in that read; under setsid it fails fast. stdin is nulled
/// separately in [`run_bounded`]: the prompts read /dev/tty (stdin carries
/// the piped payload under `curl | sh`), so a closed stdin alone cannot
/// decide the gate.
fn detached_sh(script: &Path) -> Command {
    let setsid = has_setsid();
    let mut cmd = Command::new(if setsid { "setsid" } else { "sh" });
    if setsid {
        cmd.arg("sh");
    }
    cmd.arg(script);
    cmd
}

/// Poll `child` until it exits or `deadline` passes; on expiry kill it, reap
/// it, and return `None` so the caller can fail with its own context. The
/// direct child owns the whole subprocess lifetime: `setsid` execs the shell
/// in-place (a `Command` spawn never makes it a process-group leader, so it
/// never forks), and `script` owns the pty lane's lifetime.
fn wait_deadline(child: &mut Child, deadline: Instant) -> Option<std::process::ExitStatus> {
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return Some(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Join reader threads bounded by `deadline`; `false` once it passed before
/// all of them finished. A pump only sees EOF when every holder of the pipe
/// write-ends closes them, so a grandchild that inherited them and outlives
/// the child would block an unbounded `join` forever — past the deadline the
/// caller was already enforcing. Callers panic with the transcript so the
/// worst case is a red test with evidence, never a hang.
fn join_bounded(readers: Vec<JoinHandle<()>>, deadline: Instant) -> bool {
    for reader in readers {
        loop {
            if reader.is_finished() {
                let _ = reader.join();
                break;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    true
}

/// Spawn `cmd` with piped output and a hard deadline: kill the child and
/// panic with its transcript on expiry, else return the captured
/// [`std::process::Output`]. Reader threads drain the pipes while `try_wait`
/// polls, so a chatty child cannot deadlock on a full pipe — their joins are
/// bounded by the same deadline (a grandchild holding the write-ends must
/// not hang past it) — and stdin is nulled explicitly (spawn inherits it,
/// unlike `output()`).
fn run_bounded(cmd: &mut Command, what: &str) -> std::process::Output {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let stdout = Arc::new(Mutex::new(String::new()));
    let stderr = Arc::new(Mutex::new(String::new()));
    let pumps = vec![
        pump(child.stdout.take().unwrap(), Arc::clone(&stdout)),
        pump(child.stderr.take().unwrap(), Arc::clone(&stderr)),
    ];
    let deadline = Instant::now() + INSTALL_TIMEOUT;
    let status = wait_deadline(&mut child, deadline);
    if !join_bounded(pumps, deadline) {
        panic!(
            "{what} pipe readers must finish within {INSTALL_TIMEOUT:?} — a \
             grandchild likely inherited the pipe write-ends: {}{}",
            stdout.lock().unwrap(),
            stderr.lock().unwrap()
        );
    }
    let Some(status) = status else {
        panic!(
            "{what} must finish within {INSTALL_TIMEOUT:?} — a prompt likely grabbed a \
             tty: {}{}",
            stdout.lock().unwrap(),
            stderr.lock().unwrap()
        );
    };
    let out = std::mem::take(&mut *stdout.lock().unwrap()).into_bytes();
    let err = std::mem::take(&mut *stderr.lock().unwrap()).into_bytes();
    std::process::Output {
        status,
        stdout: out,
        stderr: err,
    }
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// The non-interactive lane (no TTY): banner + full install plan print, the
/// consent prompt is skipped, the run proceeds and installs.
#[test]
fn install_sh_prints_plan_and_proceeds_non_interactive() {
    let Some(target) = platform_target() else {
        return;
    };
    if !Command::new("sh").arg("-c").arg("exit 0").status().is_ok() {
        return;
    }
    let fixture = release_fixture(target);
    let root = setup("install-plan");
    stub_opencode(&root);
    let bin = root.join("bin").join("carpenter_nightly");

    let out = run_installer(&installer_script(), &root, &fixture);
    assert!(
        out.status.success(),
        "install.sh failed: {}{}",
        stdout(&out),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout(&out);
    assert!(text.contains("carpenter installer  ·  nightly"), "{text}");
    assert!(text.contains("install plan"), "{text}");
    assert!(
        text.contains(&format!("carpenter-{target}.tar.gz")),
        "{text}"
    );
    assert!(text.contains("register  opencode ->"), "{text}");
    assert!(
        text.contains("non-interactive — proceeding with the plan above"),
        "{text}"
    );
    assert!(text.contains("✓ installed carpenter"), "{text}");
    assert!(
        !text.contains("proceed with the install plan?"),
        "non-interactive run must not print the consent prompt: {text}"
    );
    assert!(bin.is_file(), "binary installed at {}", bin.display());

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&fixture);
}

/// The tagline is channel-correct: the repo script (TAG="nightly") shows the
/// canary mark; the stable release's tag-patched copy must show the plain
/// `carpenter installer` and no `nightly` anywhere. This also pins the
/// `TAG="nightly"` line shape that `.github/workflows/release.yml` seds — if
/// the anchor moves, this fails before a release ships a broken installer.
#[test]
fn install_sh_banner_is_channel_correct() {
    let Some(target) = platform_target() else {
        return;
    };
    if !Command::new("sh").arg("-c").arg("exit 0").status().is_ok() {
        return;
    }
    let script = installer_script();
    let text = std::fs::read_to_string(&script).unwrap();
    let anchor = "TAG=\"nightly\"";
    assert_eq!(
        text.matches(anchor).count(),
        1,
        "release.yml seds this exact line; keep one TAG anchor"
    );
    let patched = text.replacen(anchor, "TAG=\"v9.9.9\"", 1);
    let stable_script = root_stub("install-channel");
    std::fs::write(&stable_script, patched).unwrap();

    let fixture = release_fixture(target);
    let root = setup("install-channel-home");
    stub_opencode(&root);
    let out = run_installer(&stable_script, &root, &fixture);
    assert!(
        out.status.success(),
        "tag-patched install.sh failed: {}{}",
        stdout(&out),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout(&out);
    assert!(text.contains("carpenter installer"), "{text}");
    assert!(
        !text.contains("nightly"),
        "stable installer output must not mention nightly: {text}"
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&fixture);
    let _ = std::fs::remove_dir_all(stable_script.parent().unwrap());
}

/// The interactive consent gate: under a real pty (`script(1)`), answering
/// `n` aborts before anything is downloaded or installed. Prompt-synced: the
/// `n` is sent only after the consent prompt printed — an early stdin close
/// masquerades as a decline on macOS, so this pins real input delivery.
/// Skips where no pty helper exists; macOS `script` does not propagate the
/// child exit status, so assertions ride on output + filesystem state.
#[test]
fn install_sh_declined_consent_installs_nothing() {
    if platform_target().is_none() || !has_script_command() {
        return; // no pty helper (unusual container); CI covers Linux + macOS
    }
    let root = setup("install-decline");
    stub_opencode(&root);
    let bin_dir = root.join("bin");

    let mut pty = Pty::spawn(&installer_script(), &root, "file:///nonexistent", &bin_dir);
    pty.expect("consent prompt", "proceed with the install plan? [Y/n]");
    pty.send("n");
    let text = pty.finish();

    assert!(
        text.contains("aborted"),
        "declining must abort with a clear message: {text}"
    );
    assert!(
        !bin_dir.join("carpenter").exists(),
        "declined install must not write a binary"
    );
    assert!(
        !config_root(&root).join("opencode").exists(),
        "declined install must not register anything"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Enter takes the encouraged default at both prompts (adr/024 Y-defaults):
/// consent proceeds, registration registers. End-to-end under a real pty,
/// prompt-synced: each Enter is sent only after its prompt printed.
#[test]
fn install_sh_enter_defaults_proceed_and_register() {
    let Some(target) = platform_target() else {
        return;
    };
    if !has_script_command() {
        return; // no pty helper (unusual container); CI covers Linux + macOS
    }
    let fixture = release_fixture(target);
    let root = setup("install-enter");
    stub_opencode(&root);
    let bin_dir = root.join("bin");

    let mut pty = Pty::spawn(
        &installer_script(),
        &root,
        &format!("file://{}", fixture.display()),
        &bin_dir,
    );
    pty.expect("consent prompt", "proceed with the install plan? [Y/n]");
    pty.send(""); // Enter takes the encouraged default
    pty.expect("register prompt", "register the skill for opencode? [Y/n]");
    pty.send("");
    let text = pty.finish();

    assert!(
        text.contains("registered in opencode (skill)"),
        "Enter at the register prompt registers: {text}"
    );
    assert!(
        bin_dir.join("carpenter_nightly").is_file(),
        "Enter at the consent prompt installs the binary"
    );
    let (skill_md, _perms) = common::opencode_paths(&root);
    assert!(skill_md.is_file(), "skill registered at {skill_md:?}");

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&fixture);
}

// --- pty sessions (script(1)) — prompt-synced driving ------------------------
//
// Each answer is written only after its prompt appeared, and the stdin
// handle stays open until the child exited. Both are load-bearing on macOS:
// BSD `script(1)` reacts to an EOF on its stdin pipe by injecting EOF into
// the pty, so a pending `read … </dev/tty` in install.sh fails (the consent
// gate aborts) when stdin is closed early — e.g. by `wait_with_output`'s
// stdin drop racing ahead of the child consuming the buffered input. Sending
// late and closing late makes input delivery real on both platforms.

/// How long a pty session waits for a prompt before failing (never hangs).
const PTY_TIMEOUT: Duration = Duration::from_secs(30);

/// A `script(1)` pty session running install.sh, driven via [`Pty::expect`]
/// and [`Pty::send`]; [`Pty::finish`] collects the full transcript.
struct Pty {
    child: Child,
    stdin: Option<ChildStdin>,
    text: Arc<Mutex<String>>,
    readers: Vec<JoinHandle<()>>,
}

/// Whether a `script(1)` pty helper exists on this runner.
fn has_script_command() -> bool {
    Command::new("sh")
        .arg("-c")
        .arg("command -v script >/dev/null 2>&1")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Append everything `stream` yields to `text` until EOF.
fn pump(mut stream: impl Read + Send + 'static, text: Arc<Mutex<String>>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => text
                    .lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&buf[..n])),
            }
        }
    })
}

impl Pty {
    /// Run install.sh under `script(1)` (a real pty) with the sandboxed env
    /// of `run_installer`; `download_base` becomes `CARPENTER_DOWNLOAD_BASE`.
    fn spawn(script: &Path, root: &Path, download_base: &str, bin_dir: &Path) -> Pty {
        let mut cmd = Command::new("script");
        if cfg!(target_os = "macos") {
            cmd.args(["-q", "/dev/null", "sh"]).arg(script);
        } else {
            cmd.args(["-qec", &format!("sh {}", script.display()), "/dev/null"]);
        }
        let mut child = cmd
            .env("HOME", root)
            .env("XDG_CONFIG_HOME", config_root(root))
            .env("CARPENTER_DOWNLOAD_BASE", download_base)
            .env("CARPENTER_INSTALL_DIR", bin_dir)
            .env("PATH", stub_path(root))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let text = Arc::new(Mutex::new(String::new()));
        let readers = vec![
            pump(child.stdout.take().unwrap(), Arc::clone(&text)),
            pump(child.stderr.take().unwrap(), Arc::clone(&text)),
        ];
        Pty {
            stdin: child.stdin.take(),
            child,
            text,
            readers,
        }
    }

    /// Block until `pattern` shows up in the merged pty output; fail (never
    /// hang) with the transcript if it does not within `PTY_TIMEOUT`.
    fn expect(&self, what: &str, pattern: &str) {
        let deadline = Instant::now() + PTY_TIMEOUT;
        loop {
            let snapshot = self.snapshot();
            if snapshot.contains(pattern) {
                return;
            }
            if Instant::now() >= deadline {
                panic!("{what} must appear under a pty: {snapshot}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Answer the current prompt — an empty line is Enter (the Y default).
    fn send(&mut self, answer: &str) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{answer}").unwrap();
        stdin.flush().unwrap();
    }

    /// Wait for the child — bounded: a `script` wedged on a prompt no answer
    /// will reach fails with the transcript instead of hanging the binary —
    /// then close stdin (an early close tears the pty down on macOS), join
    /// the readers bounded by the same deadline (a grandchild that inherited
    /// the pipe write-ends must not hang past it), and return the full
    /// transcript (CRs stripped).
    fn finish(mut self) -> String {
        let deadline = Instant::now() + PTY_TIMEOUT;
        if wait_deadline(&mut self.child, deadline).is_none() {
            panic!(
                "install.sh must exit under script(1) within {PTY_TIMEOUT:?}: {}",
                self.snapshot()
            );
        }
        self.stdin.take(); // close stdin only after the child exited
        let readers = std::mem::take(&mut self.readers);
        if !join_bounded(readers, deadline) {
            panic!(
                "pty readers must finish within {PTY_TIMEOUT:?} — a grandchild \
                 likely inherited the pipe write-ends: {}",
                self.snapshot()
            );
        }
        self.snapshot()
    }

    fn snapshot(&self) -> String {
        self.text.lock().unwrap().replace('\r', "")
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        // A panic between prompts must not leak a pty-blocked child.
        let _ = self.child.kill();
    }
}

/// A scratch dir for the tag-patched script copy (sibling cleanup in the test).
fn root_stub(tag: &str) -> PathBuf {
    let dir = unique(tag);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("install.sh")
}
