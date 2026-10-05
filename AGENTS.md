# AGENTS.md — carpenter

Guidance for any agent or human working on carpenter. Read before editing.

## What this is
carpenter is a Rust CLI an LLM agent drives to build Python/Jupyter learning
material. SQLite is the source of truth; notebooks render from it. No embedded
LLM. Authoritative docs: `docs/design/`, `docs/data-model/`, `docs/specs/`,
`docs/adr/` (each directory has a `README.md` index; read a section as one chunk).

## How it works (structure)

Module map:

```
build.rs      syn-scan commands/ -> fail build if a command fn lacks docs/tests;
              also gate examples/*.md scenarios (>=3 distinct fns each; adr/013)
app.rs        clap wiring + emit harness. No logic.
manual.rs     howto text = include_str!("howto.gen.md") (xtask-produced)
core/         store · db · compare · notebook · helper · skill · status · output · error
models/       serde structs (Data enum = one variant per command; *Spec types)
commands/     ONLY command fns (helpers live in core/)
examples/     scenario files (one multi-command workflow each; .md only)
xtask/        gen-howto · gen-specs · build  (codegen pipelines)
```

Request flow: `clap` parse → `app::run` resolves `Paths` + the active course →
dispatches to a `commands::*` fn → the command returns `Result<Data,
CarpenterError>` → `app::emit` wraps **one** JSON envelope on stdout (ok → exit 0,
error → exit 1). Spec parsing is centralized in the `_emit` path, so a bad
`--spec` maps to a `ValidationError` envelope, not a crash.

Project layout:

```
src/
  app.rs, main.rs, lib.rs, manual.rs
  core/         store, db, notebook, helper, compare, status, skill, output, error, …
  models/       serde Data/Spec structs + co-located `examples`
  commands/     one module per command group (command fns only)
  howto.gen.md  generated — never hand-edit
xtask/          gen-howto, gen-specs, build
docs/
  design/       architecture + rationale (read a section as one chunk)
  data-model/   ER, DDL, conventions, status derivation
  specs/        per-command I/O contracts (tables generated from types)
  adr/          architecture decision records
  examples/     one worked example per CLI leaf (the howto's source)
examples/        scenario files (one multi-command workflow each; .md only — gated, adr/013)
```

## Working principles
How to think and communicate on this project. Read-deeply, YAGNI, and
never-cut-safety are the ponytail ladder below; these are the rest.

- **Engineering only.** No filler, preamble, or recap of what you just did. Lead
  with the decision/answer. Dense over prose — tables, code, terse bullets.
- **Design before code.** Research → plan → confirm → execute. Settle the design
  before implementing; don't jump to abstractions.
- **Automate everything verifiable.** Generate docs from the code surface (`howto`
  from `clap`); add tests that fail on drift (docs vs. code), hangs (subprocess
  timeouts), and dead/unused code (`clippy -D warnings`, the envelope smoke test).
  If a human must keep two things in sync, that's a bug.
- **One concern per file.** Small, self-contained docs (see `docs/` layout); no
  monoliths.
- **DRY across code and docs.** Anything repeated has one source and is generated or
  referenced from it — never copy-pasted. Same rule for prose as for code.
- **Source of truth & ownership.** Always state what's authoritative and who owns
  what (DB vs. notebook; agent vs. learner). See Conventions.
- **Record rationale.** Non-obvious decisions go in `docs/adr/` so they aren't
  relitigated.

## Build & test
```bash
cargo xtask build                 # = gen-howto + gen-specs + build  (the canonical build)
cargo build                       # fails if a command lacks /// docs, an example file, or a test; or a scenario fails the >=3-fn gate
cargo test --workspace            # or: cargo nextest run --workspace  (howto+spec stale-checks, skill-determinism, compare-parity, sync-goldens, envelope-smoke)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace
```
`--workspace`/`--all` is required: this is a non-virtual workspace (root has
`[package]`), so bare `cargo test`/`clippy` skip the `xtask` crate and its
stale-check tests. `cargo doc` rejects `-- -D warnings` in this toolchain, so
the deny flag goes via `RUSTDOCFLAGS`.
Pre-commit lint hook (per clone): `git config core.hooksPath .githooks` —
`.githooks/pre-commit` runs `cargo fmt --all -- --check` when Rust files are
staged (clippy/tests stay in CI). Guarded by `pre_commit_hook_is_present_and_executable`.
`cargo build` is self-documenting by construction:
- `#![deny(missing_docs)]` — every public item needs a `///` (clap reads it as `--help`).
- `build.rs` (syn scan) — every command fn needs a worked-example file at
  `docs/examples/<module>/<fn>.md` (the howto's single example source) and a paired
  `#[test] fn <name>_*` in the same module. It also gates `examples/*.md` scenarios:
  each must invoke ≥3 distinct command fns (resolved via the same signature-based
  set), and ≥1 must exist (adr/013).

Generated — **never hand-edit**: `src/howto.gen.md` (whole file), and the
`<!-- BEGIN GENERATED -->`…`<!-- END GENERATED -->` table region in `docs/specs/*.md`
(surrounding narrative is hand-maintained). Drift is caught inside `cargo test`:
`howto_gen_md_is_fresh` and `specs_marker_regions_are_fresh` regenerate each to a
buffer and assert byte-equality with the committed file (no `git`/CI needed). Run
`cargo xtask build` (or `gen-howto`/`gen-specs`) to refresh them after a change.
`src/howto.gen.md` is generated from `docs/examples/<module>/<fn>.md` — one
hand-authored worked example per CLI leaf (invocation + spec + envelope). Add a
command fn → add its example file, or `cargo build` fails.
Rationale: [adr/007](docs/adr/007-compile-enforced-command-docs.md),
[adr/008](docs/adr/008-specs-generated-from-types.md),
[adr/009](docs/adr/009-skill-assembled-from-fields.md).

CI (`.github/workflows/ci.yml`) mirrors these gates on an
`ubuntu-latest`/`macos-14` matrix (the two supported OSes —
[design/17](docs/design/17-cross-platform.md),
[adr/012](docs/adr/012-cross-platform-paths.md)) — `rust-toolchain.toml`
pins stable and `uv` is installed. Releases (`.github/workflows/release.yml`)
are branch-governed channels ([adr/021](docs/adr/021-nightly-main-channels.md))
in the standard human-bump shape ([adr/025](docs/adr/025-standard-release-flow.md)):
feature PRs target `nightly`; a merge into `nightly` rolls the rolling
`nightly` prerelease at that sha (version unchanged — the title carries the
short-sha), and a `nightly → main` promotion publishes an immutable stable
`vX.Y.0`. Versions are bumped by humans via `cargo xtask bump` as a commit
in a normal PR — fold it into the release-prep PR; CI never commits to a
protected branch and holds no credentials. ci.yml's `guard` job
fails any PR into `main` whose head is not `nightly` or whose version is not
exactly main's minor+1. Each
publish ships `x86_64-unknown-linux-musl` + `aarch64-apple-darwin` tarballs,
`SHA256SUMS`, and a channel-correct `scripts/install.sh` (stable's is
tag-patched so it can never fetch nightly bits; the `curl | sh` one-liner
auto-registers into detected agent apps; Intel Mac users build from source) —
then a **smoke job** verifies the published artifact through the real one-liner
(version, howto, register, uninstall) against a **real opencode** (curl
installer on Linux; curl + brew lanes on macOS), plus a never-failing
path-report diagnostic capturing which skill path each side chose (evidence for
the macOS skill-path mismatch; assertions stay dual-path until triaged). The
same smoke lanes run **pre-merge on PRs** against PR-built artifacts
(`CARPENTER_DOWNLOAD_BASE=file://`), and the release job is push-only — so a PR
can never publish nor merge with red smoke. `carpenter upgrade` (no flags)
fetches the Latest stable release — checksum-verified via the same pipeline —
and refreshes registered apps' skills; `--channel nightly` opts into the canary
(adr/018, adr/021).

## Integration & release (adr/021, adr/025)
`nightly` is the integration trunk: always green; a merge rolls the rolling
`nightly` prerelease at that sha. `main` is the frozen release branch —
stable `vX.Y.0` publishes only from a `nightly → main` promotion PR whose
content carries the minor bump (human commit; `guard` checks head ==
`nightly` and the exact version). After a promotion `nightly` stays behind
`main`; reconcile before the next promotion by merging `main` back into
`nightly` via a PR, then bump. Bootstrap: `nightly` was cut from `main` HEAD
when the model landed (adr/021's PR merged into `nightly`).
- **Short-lived branches only**: `ivan/<topic>`, target ≤1 day of work, one
  concern per branch. No long-lived branches — unfinished work lands dark
  behind the `dev` feature flag (adr/016) instead of aging on a branch. The one
  sanctioned exception is the **channel branch** `nightly`, which exists to cut
  releases, not to develop on.
- **PR ground rules** (into `nightly`;
  `.github/PULL_REQUEST_TEMPLATE.md`): unit tests green; new features ship with
  unit tests; `.opencode/agents/carpenter-dev-validate.md` updated when the CLI
  surface/study workflow moves; the PR explains the feature; a
  carpenter-dev-validate report (learning simulation smooth over existing +
  new features) is attached. `CODEOWNERS` (`* @meolord29`) + branch protection
  make every nightly merge owner-approved.
- **Merge green or don't merge**: ci.yml must pass on the branch head; rebase
  onto `nightly` before merging if it has moved. Delete the branch after merge.
- **Branch protection** ([adr/023](docs/adr/023-ruleset-bypass-actors.md),
  revised by [adr/025](docs/adr/025-standard-release-flow.md) —
  rulesets-only; no classic branch protection): `nightly` + `main` each
  require a PR, code-owner review, and the checks (gates, build, smoke
  lanes; strict/up-to-date), forbid force-pushes and deletions; `main`
  additionally requires the `guard` job (only `nightly` merges into it).
  Bypass actors: the owner alone — nothing automated pushes to either
  trunk, so no CI identity holds bypass or any other credential. Even the
  owner's plain merge is refused (sole code owner can't self-approve) —
  bypassing is explicit (`gh pr merge --admin`).

## Dev authoring loop (`--dev`)
Two build stages ([design/19](docs/design/19-dev-build.md),
[adr/016](docs/adr/016-dev-feature.md)):
- **dev** — `cargo xtask build --dev`: compiles with the `dev` feature, which
  **relaxes** the doc/example/scenario gates and `missing_docs` (so a command fn
  compiles before its atom/test exist). No doc regen.
- **release** — `cargo xtask build --release`: gen-howto + gen-specs + the strict
  `cargo build` (the ship build). Plain `cargo build` / `cargo xtask build` are
  also strict — a casual build still fails on an undocumented command (safety
  net). `dev` + `release` together is **rejected** (no relaxed binary ships).

Authoring a new command fn (the chicken-and-egg adr/007/adr/013 create):
```sh
cargo xtask build --dev                                          # 1. relaxed binary
./target/debug/carpenter <group> <fn> … --capture-example docs/examples/<module>/<fn>.md
                                                                 # 2. run for real + write the atom
# 3. add the `#[test] fn <fn>_*` by hand; author the atom's TODO note line
cargo xtask build                                                # 4. strict verify (now passes)
```
`--capture-example` (dev-only) writes the worked-example atom from a real run
(real envelope); the behavioral note stays a TODO for a human/LLM. The dev
surface is `cfg`-gated out of release, and xtask regenerates docs from a
non-dev link so `--capture-example` never appears in `howto.gen.md`/`SKILL.md`
(`howto_excludes_dev_surface` pins it).

## The ponytail ladder (apply before writing any code)
Before writing code, stop at the first rung that holds:
1. Does this need to exist? → no: skip it (YAGNI).
2. Already in this codebase? → reuse it, don't rewrite.
3. Stdlib / a crate does it? → use it.
4. One line? → one line.
5. Only then: the minimum that works.

Lazy about the solution, never about reading. Read the code the change touches
and trace the real flow before picking a rung. **Never cut** validation, the
subprocess isolation boundary, the verification-only guardrail (helper must never
print `expected`), or atomic writes. Code is small because it is necessary, not
golfed.

## Conventions
- **Rust:** `clap` derive for CLI, `serde` for models, `rusqlite` for storage,
  `thiserror` for errors. No `unwrap`/`expect` outside tests; use `?` and typed
  `CarpenterError`.
- **Layering:** `app.rs` = wiring only. Commands return `Result<Data,
  CarpenterError>`; `_emit` wraps one envelope. Commands never touch SQL except
  via `core/db.rs`. `commands/` holds **only** command fns (helpers live in
  `core/`) — build.rs scans by signature, so a helper there with the right
  signature breaks the build. One compare module (`core/compare.rs`); helper's
  Python compare must match it (see `docs/specs/20-helper-contract.md`).
- **Source of truth:** the DB. Notebooks are rendered; we do not parse notebooks
  back into rows. Sync is idempotent and must preserve learner-edited stubs.
- **Runtime:** learner Python runs in the course venv (`uv run`); `lesson execute`
  and `quiz run` require `carpenter venv create` first (else `StoreError`).
  `helper.py` is stdlib-only (no venv needed). See `docs/design/16-execution.md`.
- **Paths:** per-OS — `config_dir` via `dirs` (`~/.config` Linux, `~/Library/…`
  macOS); `bin_dir` default `~/.local/bin` from `dirs::home_dir()`. `dirs` is
  the only platform surface — zero `#[cfg]` in the codebase (Linux/macOS only;
  Windows unsupported). See `docs/design/17-cross-platform.md`,
  `docs/adr/012-cross-platform-paths.md`.
- **IDs:** stable strings (`s1`, `p1`, `q1`, …), not SQLite rowids. See
  `docs/data-model/`.
- **Errors:** never raise to the caller — always an error envelope. List/show
  surface corrupt rows in `errors[]`; never silent `unwrap`/`continue`.
- **Comments:** `///` doc comments are **mandatory** on all public items
  (`#![deny(missing_docs)]`); each command fn also needs a worked-example file at
  `docs/examples/<module>/<fn>.md` and the module a paired `#[test] fn <name>_*`
  (build.rs enforces both — adr/007). Regular `//` comments stay discouraged — code
  is self-documenting.

## When you change something
- Added/changed a command or flag → run `xtask gen-howto` + `xtask gen-specs`
  (spec **tables** are generated from types via `gen-specs` — adr/008; tables
  fill per-file as their `*Spec`/`Data` types land); update
  `docs/data-model/` if schema moved. The `register` skill body is assembled from
  `core/skill.rs` authored fields and **embeds the generated howto** (`manual::MANUAL`)
  at render time — never hand-write command details into the skill; they come from the
  generated manual (adr/009).
- Added a command fn → it must have `///` + a worked-example file
  `docs/examples/<module>/<fn>.md` + a `#[test] fn <name>_*`, or `cargo build` fails
  (adr/007). Then `xtask gen-howto`
  + `xtask gen-specs` regenerate the surfaces; commit the generated files.
- Added/changed a scenario → `examples/*.md` are multi-command workflows; each must
  invoke ≥3 distinct command fns or `cargo build` fails (adr/013). `xtask gen-howto`
  embeds them verbatim into a `## Scenarios` section (→ auto-inlined into `SKILL.md`
  by `render()`); commit the regenerated `src/howto.gen.md`. `.md` only under `examples/`.
- Schema change → add a migration in `core/db.rs`; update `docs/data-model/`.
- Keep tests green; add a case for new behavior. Every command fn has a mandated
  `#[test] fn <name>_*` (adr/007) — a new one lands by construction or the build
  fails. Every `Data` variant also needs a `rows()` example in its
  `models::<group>::examples`, or the generated spec table (adr/008) **and** the
  registry-driven `envelope_smoke_round_trips_every_example` won't cover it.
