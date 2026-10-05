# Architecture

```
build.rs          syn-scan commands/ -> fail build if a command fn lacks a /// example
                  block or a paired #[test] fn <name>_* (adr/007). Also gates
                  examples/*.md scenarios: each must reference >=3 distinct command
                  fns; >=1 must exist (adr/013).
app.rs            clap wiring + _emit envelope harness. No logic.
manual.rs         howto text = include_str!("howto.gen.md") (xtask-produced).
core/
  store.rs        root resolution; slugify; path helpers; course open/init
  db.rs           rusqlite repo: typed accessors + relation joins (sole SQL surface)
  compare.rs      compare(actual, expected, mode) — single source of compare
                  semantics (parity-tested against the helper's Python impl)
  notebook.rs     render lesson.ipynb from DB; idempotent sync; managed cells
                  (section/practice/quiz/check/skip-config)
  helper.rs       render verification-only helper.py (reads+writes course.db: sets
                  pass_or_fail/last_check on each check — adr/010)
  skill.rs        render SKILL.md from typed fields (NAME/DESCRIPTION/WHAT_THIS_IS/
                   INTAKE/WORKFLOW/WALKTHROUGH/PEDAGOGY) — no template (adr/009)
  output.rs       envelope structs -> JSON
  error.rs        CarpenterError hierarchy + codes
models/           serde structs: Data (command success-payload enum, one variant
                  per command — serialized as the envelope `data`), Course/Spec,
                  Lesson/Spec, Section, Checkable, TestCase, Note, Plan, Goal,
                  enums — each *Spec/Data type carries a co-located
                  `mod examples` feeding gen-specs (adr/008)
commands/         mod.rs + course, lesson, plan, goal, quiz, progress, notes,
                  bug, feature, config, venv, skip, link, build, install, upgrade,
                  register, deregister, howto — holds ONLY command fns (helpers in core/)
xtask/            gen-howto: introspect clap Command + docs/examples/* + examples/*.md
                  (scenarios) -> howto.gen.md
                  gen-specs: *Spec/Data types -> docs/specs/*.md
                  build: gen-howto + gen-specs + cargo build (--release for `upgrade`)
```

Rules: `app.rs` is wiring only. Commands return `Result<Data, CarpenterError>`;
`_emit` wraps into one envelope (spec parsing is centralized so a bad spec maps to
`ValidationError`, not a crash). Commands never write SQL except through
`core/db.rs`. One compare module (mirrored by the generated `helper.py`); semantics
locked by [specs/20-helper-contract.md](../specs/20-helper-contract.md).
`register` and `upgrade` share a `write_skill()` helper (renders `SKILL.md` + merges
the permission entry) — `upgrade` calls it conditionally to refresh a registered
skill. `#![deny(missing_docs)]` at the crate root forces a `///` on every public
item (clap reads it as `--help`); `build.rs` fails the build if a command fn lacks
an example block or a paired `#[test] fn <name>_*` (see [adr/007](../adr/007-compile-enforced-command-docs.md)).
`core/skill.rs` is the single source of the `SKILL.md` body — this doc references
it, never duplicates (see [adr/009](../adr/009-skill-assembled-from-fields.md)).
`dirs` is the only platform surface (`config_dir` per-OS, `bin_dir` from
`home_dir`) — zero `#[cfg]` in the codebase — see [design/17](17-cross-platform.md),
[adr/012](../adr/012-cross-platform-paths.md).

## Stack

_(merged from the former `02-stack.md`)_

| concern | crate |
|---------|-------|
| CLI | `clap` (derive; `--help` is free and is what `howto` scrapes) |
| models / serialization | `serde` + `serde_json` + `serde_yml` (YAML `--spec` input — [adr/014](../adr/014-yaml-spec-input.md)) |
| storage | `rusqlite` (bundled SQLite) |
| notebooks | `serde_json` (build `.ipynb` v4 cells; metadata as `Value`) |
| subprocess (uv + nbconvert) | `std::process::Command` via `core/exec.rs` (`uv run jupyter nbconvert` for lesson execute + quiz run) |
| config dirs | `dirs` (per-OS: `~/.config` Linux, `~/Library/Application Support` macOS, `%APPDATA%` Windows — `store::config_dir`) |
| platform paths | `core/platform.rs` — `#[cfg(target_os)]` `bin_dir` default + executable name ([adr/012](../adr/012-cross-platform-paths.md)) |
| slugs | `unicode-normalization` (NFKD fold in `store::slugify`) |
| errors | `thiserror` |
| howto codegen | `xtask` binary ([adr/003](../adr/003-howto-buildstep.md)) |
| spec codegen | `xtask gen-specs` from `*Spec`/`Data` serde types ([adr/008](../adr/008-specs-generated-from-types.md)) |
| compile-time doc/test gate | `build.rs` + `syn` build-dep ([adr/007](../adr/007-compile-enforced-command-docs.md)) |

Tooling: `cargo`, `rustfmt`, `clippy --workspace --all-targets -- -D warnings`,
`cargo test` / `cargo-nextest`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
--workspace` (the deny flag goes via `RUSTDOCFLAGS` — `cargo doc` rejects
`-- -D warnings` in this toolchain).
