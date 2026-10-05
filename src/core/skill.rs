//! The carpenter `SKILL.md` renderer + register/deregister (agent-app skill
//! integration). Assembled from typed fields — no template
//! ([adr/009](../../docs/adr/009-skill-assembled-from-fields.md),
//! [design/15](../../docs/design/15-opencode-integration.md)).
//!
//! Authored consts are the only hand-written atoms; the command surface is the
//! generated `howto` manual ([`crate::manual::MANUAL`]), inlined into the body at
//! render time (DRY — same generated artifact, never hand-duplicated). See the
//! Update block in [adr/009](../../docs/adr/009-skill-assembled-from-fields.md).

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::core::error::CarpenterError;
use crate::core::store;

/// The skill name (also the skills-dir leaf). Always `carpenter`.
pub const NAME: &str = "carpenter";

/// Frontmatter matcher (authored once).
const DESCRIPTION: &str =
    "Activate when the user is building, authoring, or running Python/Jupyter learning material with carpenter. Drives the carpenter CLI (SQLite source of truth, rendered notebooks, scored practice/quiz).";

/// Authored prose — exists nowhere else in code to derive from.
const WHAT_THIS_IS: &str = "carpenter is an agent-driven CLI that builds Python/Jupyter learning material. SQLite is the source of truth; notebooks render from it. You (the agent) are the tutor — carpenter is deterministic storage, rendering, and execution.";

/// Authored learner-intake interview. Runs before ANY carpenter command — the
/// answers shape the curriculum itself (course goal, plan goals, lesson count).
/// `course create` is gated on the user agreeing to the course goal, mirroring
/// the `lesson create` gate in [`WALKTHROUGH`].
const INTAKE: &str = "Before any carpenter command, interview the user and **agree on the course goal**. Do NOT call `course create` until the answers below are in and the user approves the proposed goal. Ask, don't assume — a curriculum built on a guessed background is a process bug.\n\n1. **What do they want to learn, and why?** — the target outcome (project, role, course, curiosity) and what \"done\" looks like to them.\n2. **Background** — languages known, math/tools experience, prior exposure to the topic; what they already know well enough to skip.\n3. **Constraints** — time budget, pace, appetite for lesson count and depth.\n\nPropose the course goal and plan goals *from the answers* and confirm before scaffolding. Carry the answers into the content walkthrough (level, prerequisites, lesson count) — do not re-interrogate.";

/// Authored workflow. Step 0 (learner intake) gates `course create` on the
/// user's agreement ([`INTAKE`]); step 3 (content walkthrough) gates
/// `lesson create` the same way — see [`WALKTHROUGH`].
const WORKFLOW: &str = "0. **Learner intake** — run the interview in the next section and agree on the course goal. Do NOT call `course create` until the user approves it.\n1. `carpenter course create` scaffolds a course (goal shaped by the intake).\n2. `carpenter plan create` + `plan confirm` set goals and link covering lessons.\n3. **Content walkthrough** — run the Q&A in the next section with the user and get explicit confirmation of the lesson outline. Do NOT call `lesson create` until the user approves.\n4. `carpenter lesson create` authors each lesson (renders a notebook + a verification-only helper).\n5. The learner fills practice/quiz stubs; `carpenter quiz run` / `lesson execute` score them live.\n6. Use `progress`, `notes`, `skip`, `bug`, and `feature` to track and refine.";

/// Authored pedagogy.
const PEDAGOGY: &str = "One concept per lesson. Practice is attached to its teaching section; quizzes assess at the end. State is live only (no attempt history) — a verification-only helper writes `pass_or_fail`/`last_check` on each check. Skipped items are excluded from status derivation. Notes capture gaps, mistakes, strengths, patterns, and progress.";

/// Authored content-walkthrough checklist. Runs after `plan confirm`, before any
/// `lesson create`. The agent must present the agreed outline back to the user and
/// wait for confirmation — generating lessons before approval is a process bug.
const WALKTHROUGH: &str = "Before generating any lesson, run this Q&A with the user and **confirm the proposed outline** (lesson list + per-lesson practice/quizzes + conventions). Do not call `lesson create` until the user explicitly approves.\n\n1. **Audience & scope** — level (beginner/intermediate/advanced), prerequisites, total lesson count, and grouping into parts/batches. Carry the learner-intake answers in — do not re-interrogate.\n2. **Outline** — for each lesson: title, the concepts it teaches, the practice functions (name + signature + return type), and the end-of-lesson quizzes. Propose the full list and let the user edit it.\n3. **Grading conventions** — agree these once and apply everywhere:\n   - Compare is **exact** by default (`compare:\"exact\"`); use `sorted`/`set` only when order or multiplicity is irrelevant.\n   - Cases return **plain Python** (`float(...)`, `.tolist()`, `int`, `bool`, `str`, nested `list`) — never raw NumPy arrays.\n   - **Float outputs round to 8 decimals** (`np.round(x, 8).tolist()`) so LAPACK noise (`inv`/`eig`/`svd`/`lstsq`) passes exact equality. Prefer integer-valued cases (bit-exact without rounding).\n   - Design cases to be **deterministic**: avoid sign/scale ambiguity (eigenvectors, SVD `U`/`V` columns) — grade on eigenvalues, singular values, ranks, traces, determinants, reconstructions, or `argmax` instead.\n4. **Stack & IDs** — language, the venv deps (`venv create` + `venv add`), and the slug/ID conventions. Author specs as **YAML** (block scalars keep multi-line markdown/code readable); `carpenter lesson new` emits a YAML template. Quote any value containing `:` (signatures always do).\n5. **Verification contract** — per lesson: (a) lock the answer key with `lesson verify` — each practice/quiz may carry a `solution` (author reference Python) that carpenter runs against its own cases in the course venv (`lesson verify --spec -` before create, `lesson verify <id>` to re-verify); (b) `lesson execute --allow-errors` reports `errored:0` (teaching cells run clean; empty stubs define functions and don't raise at define time); (c) `quiz run` on the fresh notebook reports all quizzes `pass_or_fail:false`. **Never hand-edit a rendered `lesson.ipynb`** — regenerate only via `lesson create` / `lesson update` / `lesson sync`.\n6. **Domain** — pick a concrete running example domain.\n\nPresent the agreed outline back as a checklist and proceed only after the user confirms.";

/// Validate a skill name against opencode's `^[a-z0-9]+(-[a-z0-9]+)*$`.
pub fn name_is_valid(s: &str) -> bool {
    let mut prev_dash = false;
    let mut alnum = 0;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() && !ch.is_ascii_uppercase() {
            prev_dash = false;
            alnum += 1;
        } else if ch == '-' {
            if prev_dash || alnum == 0 {
                return false;
            }
            prev_dash = true;
        } else {
            return false;
        }
    }
    !prev_dash && alnum > 0
}

/// Escape `s` for a YAML double-quoted scalar.
fn yaml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The howto manual with its leading H1 line stripped, so inlining it under the
/// skill's own structure doesn't produce a second `#` heading. The manual is the
/// single source of the command surface ([adr/009] update).
fn manual_body() -> &'static str {
    let m = crate::manual::MANUAL;
    match m.find('\n') {
        Some(i) => m[i..].trim_start_matches('\n'),
        None => m,
    }
}

/// Strip Linux's `" (deleted)"` suffix. The kernel appends it to `/proc/self/exe`
/// (read by [`std::env::current_exe`]) when the running binary's file has been
/// replaced in place — which happens during `upgrade` (it copies the new binary
/// over its own path, then refreshes the skill). Without this the embedded path
/// reads `…/carpenter (deleted)`.
fn strip_deleted(s: &str) -> &str {
    s.strip_suffix(" (deleted)").unwrap_or(s)
}

/// Render the full `SKILL.md` (frontmatter + body). Embeds the running version,
/// binary path, and the full generated howto manual. Deterministic within a
/// process (same `current_exe`).
pub fn render() -> Result<String, CarpenterError> {
    let exe = std::env::current_exe()
        .map_err(|e| CarpenterError::StoreError(format!("current_exe failed: {e}")))?;
    let exe = exe.display().to_string();
    let bin = strip_deleted(&exe);
    let version = env!("CARGO_PKG_VERSION");
    let frontmatter = format!(
        "---\nname: {NAME}\ndescription: \"{}\"\n---\n\n",
        yaml_escape(DESCRIPTION)
    );
    let body = format!(
        "# {NAME}\n\n\
         {WHAT_THIS_IS}\n\n\
          ## Learner intake\n\n{INTAKE}\n\n\
          ## Workflow\n\n{WORKFLOW}\n\n\
         ## Content walkthrough\n\n{WALKTHROUGH}\n\n\
         ## Pedagogy\n\n{PEDAGOGY}\n\n\
         ## Command manual\n\n\
         Sourced from `{NAME} howto` at render time — always matches the installed \
         binary. Includes end-to-end worked **scenarios** (course → plan → lesson \
         → quiz → progress) under its `## Scenarios` heading — consult them before \
         authoring a new course; the scenarios are illustrative (paths and slugs \
         are placeholders, not real files to read or navigate to). Installed \
         version: {version} (`{bin}`).\n\n\
         {manual}",
        manual = manual_body(),
    );
    Ok(format!("{frontmatter}{body}"))
}

/// Which agent app to integrate with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum App {
    /// opencode (`~/.config/opencode/`).
    Opencode,
    /// claude code (`~/.claude/skills/`).
    ClaudeCode,
}

impl App {
    /// Every supported app (iteration order = registration order).
    pub fn all() -> [App; 2] {
        [App::Opencode, App::ClaudeCode]
    }

    /// Parse + validate an `--app` value.
    pub fn parse(s: &str) -> Result<App, CarpenterError> {
        match s {
            "opencode" => Ok(App::Opencode),
            "claude-code" => Ok(App::ClaudeCode),
            "agents" => Err(CarpenterError::ValidationError(format!(
                "app {s:?} not yet supported (only `opencode` and `claude-code`)"
            ))),
            other => Err(CarpenterError::ValidationError(format!(
                "unknown app {other:?} (opencode|claude-code|agents)"
            ))),
        }
    }

    /// Human label.
    pub fn name(self) -> &'static str {
        match self {
            App::Opencode => "opencode",
            App::ClaudeCode => "claude-code",
        }
    }

    /// The skill file path. `root` is the XDG root (e.g. `~/.config` — opencode
    /// is a sibling of `carpenter/`); `home` is the user home dir (claude code
    /// anchors at `~/.claude/`, not the XDG root).
    pub fn skill_path(self, root: &Path, home: &Path) -> PathBuf {
        let base = match self {
            App::Opencode => root.join("opencode"),
            App::ClaudeCode => home.join(".claude"),
        };
        base.join("skills").join(NAME).join("SKILL.md")
    }

    /// The permission file path, if the app needs one. opencode loads skills
    /// behind a permission entry; claude code auto-discovers `~/.claude/skills/`
    /// — no permission store to merge.
    pub fn permission_path(self, root: &Path) -> Option<PathBuf> {
        match self {
            App::Opencode => Some(root.join("opencode").join("opencode.json")),
            App::ClaudeCode => None,
        }
    }
}

/// A successful registration.
#[derive(Debug)]
pub struct Registered {
    /// app name.
    pub app: String,
    /// skill file path.
    pub path: String,
    /// embedded version.
    pub version: String,
}

/// A successful deregistration.
#[derive(Debug)]
pub struct Deregistered {
    /// app name.
    pub app: String,
    /// skill file path that was removed.
    pub path: String,
}

fn read_json(path: &Path) -> Result<Option<Value>, CarpenterError> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| CarpenterError::StoreError(format!("corrupt {}: {e}", path.display()))),
        Err(_) => Ok(None),
    }
}

fn write_json(path: &Path, v: &Value) -> Result<(), CarpenterError> {
    let bytes = serde_json::to_string_pretty(v)
        .unwrap_or_else(|_| v.to_string())
        .into_bytes();
    store::atomic_write(path, &bytes)
}

/// Ensure each `path` segment is a JSON object, descend, and set the leaf to `value`.
fn set_nested(root: &mut Value, path: &[&str], value: Value) -> Result<(), CarpenterError> {
    let mut cur = root;
    for (i, key) in path.iter().enumerate() {
        if !cur.is_object() {
            *cur = Value::Object(serde_json::Map::new());
        }
        let obj = cur.as_object_mut().ok_or_else(|| {
            CarpenterError::StoreError(format!("config segment `{}` not an object", path[i]))
        })?;
        cur = obj
            .entry((*key).to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
    }
    *cur = value;
    Ok(())
}

/// Register: render + write `SKILL.md` (idempotent) and merge the allow entry
/// (apps that have one). `root` is the XDG root containing both `carpenter/`
/// and `opencode/`; `home` is the user home dir.
pub fn register(app: App, root: &Path, home: &Path) -> Result<Registered, CarpenterError> {
    let content = render()?;
    let skill_path = app.skill_path(root, home);
    store::atomic_write(&skill_path, content.as_bytes())?;
    if let Some(perm_path) = app.permission_path(root) {
        let mut perm_root =
            read_json(&perm_path)?.unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        set_nested(
            &mut perm_root,
            &["permission", "skill", NAME],
            Value::String(String::from("allow")),
        )?;
        write_json(&perm_path, &perm_root)?;
    }
    Ok(Registered {
        app: app.name().into(),
        path: skill_path.display().to_string(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

/// Deregister: remove `SKILL.md` (+ dir if empty) and the allow key (apps that
/// have one). `NotFound` if the skill file is absent. `root` is the XDG root;
/// `home` is the user home dir.
pub fn deregister(app: App, root: &Path, home: &Path) -> Result<Deregistered, CarpenterError> {
    let skill_path = app.skill_path(root, home);
    if !skill_path.exists() {
        return Err(CarpenterError::NotFound(format!(
            "skill for app {}",
            app.name()
        )));
    }
    std::fs::remove_file(&skill_path).map_err(store::io_to_store)?;
    if let Some(dir) = skill_path.parent() {
        let empty = dir.read_dir().map_err(store::io_to_store)?.next().is_none();
        if empty {
            let _ = std::fs::remove_dir(dir);
        }
    }
    if let Some(perm_path) = app.permission_path(root) {
        if let Some(mut perm_root) = read_json(&perm_path)? {
            if let Some(skill) = perm_root
                .get_mut("permission")
                .and_then(|p| p.get_mut("skill"))
            {
                if let Some(obj) = skill.as_object_mut() {
                    obj.remove(NAME);
                }
            }
            let _ = write_json(&perm_path, &perm_root);
        }
    }
    Ok(Deregistered {
        app: app.name().into(),
        path: skill_path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_is_deterministic() {
        let a = render().expect("render");
        let b = render().expect("render");
        assert_eq!(a, b, "re-render must be byte-equal (adr/009)");
    }

    #[test]
    fn strip_deleted_handles_upgrade_artifact() {
        assert_eq!(
            strip_deleted("/home/u/.local/bin/carpenter"),
            "/home/u/.local/bin/carpenter"
        );
        assert_eq!(
            strip_deleted("/home/u/.local/bin/carpenter (deleted)"),
            "/home/u/.local/bin/carpenter"
        );
        // only a true suffix match is stripped
        assert_eq!(strip_deleted("/x (deleted) y"), "/x (deleted) y");
        assert_eq!(strip_deleted(""), "");
    }

    #[test]
    fn frontmatter_validates() {
        let s = render().expect("render");
        assert!(s.starts_with("---\n"), "missing opening fence:\n{s}");
        let close = s.find("\n---\n").expect("frontmatter close fence");
        let fm = &s[4..close];
        let name = fm
            .lines()
            .find(|l| l.starts_with("name:"))
            .expect("name line");
        let name_val = name.trim_start_matches("name:").trim();
        assert_eq!(name_val, NAME);
        assert!(name_is_valid(NAME), "name must match the regex");
        assert!(fm.contains("description:"), "missing description");
        assert!(
            s.contains("carpenter howto"),
            "body must reference howto (the inlined manual's source)"
        );
        assert!(
            s.contains("## Content walkthrough"),
            "body must carry the walkthrough section"
        );
        assert!(
            s.contains("## Learner intake"),
            "body must carry the learner-intake section"
        );
        assert!(
            s.contains("Do NOT call `course create`"),
            "intake must gate course creation on user agreement"
        );
        assert!(
            s.contains("Do not call `lesson create`"),
            "walkthrough must gate lesson creation on user confirmation"
        );
        assert!(
            s.contains("Never hand-edit"),
            "verification contract must forbid hand-editing rendered notebooks"
        );
        assert!(
            !s.contains("fill stubs"),
            "verification contract must not instruct the mutating fill/restore pattern"
        );
        assert!(
            s.contains("lesson verify"),
            "walkthrough must steer to lesson verify (not the obsolete throwaway script)"
        );
        assert!(
            s.contains("YAML"),
            "walkthrough must steer to YAML spec authoring"
        );
        assert!(
            !s.contains("throwaway script"),
            "walkthrough must not reference the obsolete throwaway-script pattern"
        );
        // The generated howto manual is inlined verbatim (adr/009 update).
        assert!(
            s.contains("## Command manual"),
            "body must carry the inlined command manual section"
        );
        assert!(
            s.contains("## plan") && s.contains("### create"),
            "body must inline the per-command sections from the howto"
        );
        assert!(
            s.contains("goal_index_"),
            "body must inline the worked-example specs (not just terse envelopes)"
        );
        // Scenarios are inlined via the manual (adr/013); the skill points at them.
        assert!(
            s.contains("## Scenarios"),
            "body must inline the ## Scenarios section from the howto"
        );
        assert!(
            s.contains("consult them before authoring"),
            "body must carry the authored pointer to the inlined scenarios"
        );
    }

    #[test]
    fn name_regex_branches() {
        assert!(name_is_valid("carpenter"));
        assert!(name_is_valid("a"));
        assert!(name_is_valid("a-b-2"));
        assert!(!name_is_valid(""));
        assert!(!name_is_valid("-a"));
        assert!(!name_is_valid("a-"));
        assert!(!name_is_valid("a--b"));
        assert!(!name_is_valid("A"));
        assert!(!name_is_valid("a_b"));
    }

    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn anchors() -> (PathBuf, PathBuf) {
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let base = std::env::temp_dir().join(format!("carpenter-skill-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let xdg = base.join("xdg");
        let home = base.join("home");
        std::fs::create_dir_all(&xdg).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        (xdg, home)
    }

    #[test]
    fn register_writes_skill_and_merges_permission() {
        let (d, home) = anchors();
        let Registered { app, path, version } =
            register(App::Opencode, &d, &home).expect("register");
        assert_eq!(app, "opencode");
        assert!(
            path.ends_with("opencode/skills/carpenter/SKILL.md"),
            "{path}"
        );
        assert!(!version.is_empty());
        assert!(d.join("opencode/skills/carpenter/SKILL.md").exists());
        let perm: Value = serde_json::from_str(
            &std::fs::read_to_string(d.join("opencode/opencode.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(perm["permission"]["skill"]["carpenter"], "allow");
        // re-register is idempotent (no error, no duplicate)
        register(App::Opencode, &d, &home).expect("idempotent");
        let perm2: Value = serde_json::from_str(
            &std::fs::read_to_string(d.join("opencode/opencode.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(perm2["permission"]["skill"]["carpenter"], "allow");
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn register_claude_code_writes_skill_without_permission_store() {
        let (d, home) = anchors();
        let Registered { app, path, .. } = register(App::ClaudeCode, &d, &home).expect("register");
        assert_eq!(app, "claude-code");
        assert!(
            path.ends_with(".claude/skills/carpenter/SKILL.md"),
            "{path}"
        );
        assert!(home.join(".claude/skills/carpenter/SKILL.md").exists());
        // claude code auto-discovers skills — nothing is written under the XDG root
        assert!(!d.join("opencode").exists() && !d.join("opencode.json").exists());
        deregister(App::ClaudeCode, &d, &home).expect("deregister");
        assert!(!home.join(".claude/skills/carpenter").exists());
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn register_preserves_other_permission_keys() {
        let (d, home) = anchors();
        let perm_path = d.join("opencode/opencode.json");
        std::fs::create_dir_all(perm_path.parent().unwrap()).unwrap();
        std::fs::write(
            &perm_path,
            br#"{"permission":{"skill":{"other-tool":"allow"},"bash":{"*":"allow"}},"theme":"dark"}"#,
        )
        .unwrap();
        register(App::Opencode, &d, &home).expect("register");
        let perm: Value =
            serde_json::from_str(&std::fs::read_to_string(&perm_path).unwrap()).unwrap();
        assert_eq!(perm["permission"]["skill"]["carpenter"], "allow");
        assert_eq!(perm["permission"]["skill"]["other-tool"], "allow"); // preserved
        assert_eq!(perm["permission"]["bash"]["*"], "allow"); // preserved
        assert_eq!(perm["theme"], "dark"); // preserved
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn deregister_removes_skill_and_key_then_not_found() {
        let (d, home) = anchors();
        register(App::Opencode, &d, &home).expect("register");
        let Deregistered { app, path } = deregister(App::Opencode, &d, &home).expect("deregister");
        assert_eq!(app, "opencode");
        assert!(path.ends_with("opencode/skills/carpenter/SKILL.md"));
        assert!(!d.join("opencode/skills/carpenter/SKILL.md").exists());
        assert!(
            !d.join("opencode/skills/carpenter").exists(),
            "empty dir removed"
        );
        let perm: Value = serde_json::from_str(
            &std::fs::read_to_string(d.join("opencode/opencode.json")).unwrap(),
        )
        .unwrap();
        assert!(perm["permission"]["skill"].get("carpenter").is_none());
        // second deregister is NotFound
        let err = deregister(App::Opencode, &d, &home).unwrap_err();
        assert!(matches!(err, CarpenterError::NotFound(_)));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn app_parse_branches() {
        assert_eq!(App::parse("opencode").unwrap(), App::Opencode);
        assert_eq!(App::parse("claude-code").unwrap(), App::ClaudeCode);
        assert!(App::parse("agents").is_err());
        assert!(App::parse("nope").is_err());
    }
}
