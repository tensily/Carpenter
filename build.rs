//! Compile-time gate (adr/007): every command fn in `src/commands/` is
//! self-documenting and tested. A miss aborts the build — no binary is produced.
//!
//! A "command" is any `pub fn` in `src/commands/` whose return type ends in
//! `Result`. Helpers must live in `core/`, never in `commands/`.
//!
//! For each command fn `<module>::<name>` we require:
//! - a worked-example file at `docs/examples/<module>/<name>.md` (the single
//!   source scraped into `howto` by `xtask gen-howto`); AND
//! - a paired `#[test] fn <name>_*` in the same module.
//!
//! The example file (not an inline `///` fence) is the atom — see the Update
//! block in adr/007.
//!
//! Under the `dev` feature (adr/016) every gate below — plus the scenario gate
//! (adr/013) — is skipped, and `#![deny(missing_docs)]` is relaxed, so a command
//! can be compiled and run to capture a real envelope before its atom/test
//! exist. `dev` + `release` is rejected (no relaxed binary ships).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=src/commands");
    println!("cargo:rerun-if-changed=docs/examples");
    println!("cargo:rerun-if-changed=examples");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // The `dev` feature (adr/016) relaxes the gates below for the authoring
    // loop. It must never ship: a release binary built with `dev` would bypass
    // the self-documentation contract (adr/007) and the scenario floor (adr/013).
    let dev = std::env::var_os("CARGO_FEATURE_DEV").is_some();
    if dev && std::env::var("PROFILE").as_deref() == Ok("release") {
        eprintln!(
            "error: the `dev` feature relaxes the doc/example/scenario gates \
             and must not be used in a release build (adr/016)"
        );
        std::process::exit(1);
    }
    if dev {
        println!("cargo:warning=dev build: doc/example/scenario gates relaxed (adr/016)");
        return;
    }

    let dir = Path::new("src/commands");
    let examples = Path::new("docs/examples");
    if !dir.exists() {
        return;
    }
    let mut errors: Vec<String> = Vec::new();
    // Known command set as `<stem>::<name>` (the same signature-based
    // identification used for the per-command gate below), reused by the
    // scenario gate to resolve `carpenter <group> <fn>` invocations.
    let mut known: HashSet<String> = HashSet::new();
    for file in walk_rs(dir) {
        let Ok(src) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Ok(parsed) = syn::parse_file(&src) else {
            continue;
        };
        let (cmds, tests) = scan(&parsed.items);
        let stem = file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        for name in &cmds {
            known.insert(format!("{stem}::{name}"));
        }
        for name in cmds {
            let prefix = format!("{name}_");
            let has_test = tests.iter().any(|t| t.starts_with(&prefix));
            let example = examples.join(&stem).join(format!("{name}.md"));
            if !example.exists() {
                errors.push(format!(
                    "commands/{stem}.rs:{name}: missing example file {}",
                    example.display()
                ));
            }
            if !has_test {
                errors.push(format!(
                    "commands/{stem}.rs:{name}: missing #[test] fn {name}_*"
                ));
            }
        }
    }
    gate_scenarios(&known, &mut errors);
    if errors.is_empty() {
        return;
    }
    for e in &errors {
        eprintln!("{e}");
    }
    std::process::exit(1);
}

/// Minimum number of distinct command fns a scenario file must invoke.
const MIN_DISTINCT_FNS: usize = 3;

/// Global value-taking flags whose following argument must be skipped when
/// parsing `carpenter` invocations. (Boolean globals like `--version` need no
/// value-skip.) Keep in sync with the global flag set in `app.rs`.
const GLOBAL_VALUE_FLAGS: &[&str] = &["-c", "--course", "--root"];

/// The scenario gate (adr/013): each `examples/*.md` must invoke ≥
/// [`MIN_DISTINCT_FNS`] distinct command fns (resolved against `known`), and at
/// least one scenario file must exist. Parses only fenced ```sh / ```bash
/// blocks; `carpenter` invocations elsewhere (e.g. inside ```json envelopes) are
/// ignored. A miss aborts the build.
fn gate_scenarios(known: &HashSet<String>, errors: &mut Vec<String>) {
    let dir = Path::new("examples");
    let mut files: Vec<PathBuf> = walk_md_flat(dir);
    files.sort();
    if files.is_empty() {
        errors.push("examples/: no scenario files (≥1 required, adr/013)".to_string());
        return;
    }
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else {
            continue;
        };
        let mut seen: HashSet<String> = HashSet::new();
        let mut unknown: Vec<String> = Vec::new();
        let mut in_fence = false;
        let mut fence_lang = "";
        for line in text.lines() {
            let t = line.trim_start();
            if t.starts_with("```") {
                if !in_fence {
                    in_fence = true;
                    fence_lang = t
                        .strip_prefix("```")
                        .unwrap_or("")
                        .split_whitespace()
                        .next()
                        .unwrap_or("");
                } else {
                    in_fence = false;
                    fence_lang = "";
                }
                continue;
            }
            if !in_fence || !(fence_lang == "sh" || fence_lang == "bash") {
                continue;
            }
            let Some(key) = parse_invocation(line, known) else {
                continue;
            };
            if known.contains(&key) {
                seen.insert(key);
            } else if !unknown.contains(&key) {
                unknown.push(key);
            }
        }
        let name = f
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if seen.len() < MIN_DISTINCT_FNS {
            errors.push(format!(
                "examples/{name}: references only {} distinct command fn(s) (≥{MIN_DISTINCT_FNS} required, adr/013)",
                seen.len()
            ));
        }
        for u in &unknown {
            errors.push(format!(
                "examples/{name}: unknown invocation `{u}` (not a known command fn)"
            ));
        }
    }
}

/// Parse one `carpenter …` line into a `<group>::<fn>` (or `<name>::<name>` for
/// a top-level command) key. Returns `None` for non-invocations / empty lines.
/// A second token resolves as a subcommand only when `<t0>::<t1>` is a known
/// command fn; otherwise the line is treated as a top-level fn with its
/// positional argument (e.g. `skip q1 --scope quiz`, `build /courses/ds`).
fn parse_invocation(line: &str, known: &HashSet<String>) -> Option<String> {
    let toks: Vec<&str> = line.split_whitespace().collect();
    if toks.first() != Some(&"carpenter") {
        return None;
    }
    let rest = strip_globals(&toks[1..]);
    let t0 = *rest.first()?;
    if let Some(t1) = rest.get(1) {
        let sub = format!("{t0}::{t1}");
        if known.contains(&sub) {
            return Some(sub);
        }
        // The `self` umbrella (adr/027): `carpenter self <verb>` dispatches to
        // the install-lifecycle command fns that live in their own modules
        // (upgrade::upgrade, installs::list, link::register, …). Resolve by
        // unique fn-name suffix across the known set.
        if t0 == "self" {
            let matches: Vec<&String> = known
                .iter()
                .filter(|k| k.ends_with(&format!("::{t1}")))
                .collect();
            if matches.len() == 1 {
                return Some((*matches[0]).clone());
            }
        }
    }
    Some(format!("{t0}::{t0}"))
}

/// Drop leading global flags (and the value of any value-taking one) so the
/// first returned token is the command group / top-level fn.
fn strip_globals<'a>(toks: &'a [&'a str]) -> Vec<&'a str> {
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i];
        if !t.starts_with('-') {
            break;
        }
        if t.contains('=') {
            i += 1;
        } else if GLOBAL_VALUE_FLAGS.contains(&t) {
            i += 2;
        } else {
            i += 1;
        }
    }
    toks[i..].to_vec()
}

/// Top-level `examples/*.md` files (scenarios are flat — one file per workflow).
fn walk_md_flat(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_file() && p.extension().map(|x| x == "md").unwrap_or(false) {
            out.push(p);
        }
    }
    out
}

/// Recursively collect `command_name`s and `test_name`s from items.
fn scan(items: &[syn::Item]) -> (Vec<String>, Vec<String>) {
    let mut cmds = Vec::new();
    let mut tests = Vec::new();
    for item in items {
        match item {
            syn::Item::Fn(f) => {
                let name = f.sig.ident.to_string();
                if f.attrs.iter().any(|a| a.path().is_ident("test")) {
                    tests.push(name.clone());
                }
                if is_public(f) && returns_result(&f.sig) {
                    cmds.push(name);
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    let (c, t) = scan(nested);
                    cmds.extend(c);
                    tests.extend(t);
                }
            }
            _ => {}
        }
    }
    (cmds, tests)
}

fn is_public(f: &syn::ItemFn) -> bool {
    matches!(f.vis, syn::Visibility::Public(_))
}

fn returns_result(sig: &syn::Signature) -> bool {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return false;
    };
    matches!(peel(ty), syn::Type::Path(tp) if tp.path.segments.last().map(|s| s.ident == "Result").unwrap_or(false))
}

fn peel(mut ty: &syn::Type) -> &syn::Type {
    while let syn::Type::Paren(p) = ty {
        ty = &p.elem;
    }
    ty
}

fn walk_rs(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk_rs(&p));
        } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
            out.push(p);
        }
    }
    out
}
