# ADR-016: a `dev` build stage for the authoring loop

Date: 2026-08-13 · Status: Accepted

## Context

[adr/007](007-compile-enforced-command-docs.md) and
[adr/013](013-compile-enforced-scenarios.md) make the strict gates — missing
example file, missing `#[test]`, sub-floor scenario — abort `cargo build`. That
is the right default: the docs cannot drift, by construction. But it creates a
chicken-and-egg for authoring. To write a *faithful* worked-example atom (the
real `--spec` YAML + the real result envelope), you want to run the command and
capture its output, which requires a binary — and the gate refuses to build one
until the example file exists. The same loop applies to scenarios.

We want a relaxed build stage an author/agent can use to compile and run a
not-yet-documented command, capture a real envelope, inject it into the right
location, and then pass the strict (release) build. The project principle
"automate everything verifiable" applies: the loop should *produce* the example
atom mechanically from a real run, not by hand-guessing an envelope.

## Decision

Two build stages, surfaced as `cargo xtask build --dev` and `cargo xtask build
--release`, backed by a Cargo feature:

1. **`[features] dev = []`** on the root package. Under it:
   - `build.rs` skips the example-presence, test-presence, and scenario gates
     (the whole of the adr/007/adr/013 enforcement), and relaxes
     `#![deny(missing_docs)]` via `#![cfg_attr(feature = "dev",
     allow(missing_docs))]` (ordered *after* the `deny` so the `allow` overrides;
     `deny` is overridable — only `forbid` is sticky).
   - `src/core/dev.rs` compiles (`#[cfg(feature = "dev")]`), adding a
     `--capture-example <PATH>` global flag to `app::cli`.

2. **`cargo xtask build --dev`** runs `cargo build --features dev` only — no
   `gen-howto`/`gen-specs` (atoms may be mid-authoring, and regen must always
   run against the strict view so dev surface never enters the manual).

3. **`cargo xtask build --release`** is unchanged: gen-howto + gen-specs +
   `cargo build --release` (strict). The everyday `cargo xtask build` / `cargo
   build` stay strict (safety net: a casual build still catches an undocumented
   command).

4. **`--capture-example <PATH>`** runs the command normally (real envelope on
   stdout) and additionally writes the worked-example atom to `<PATH>` — the
   invocation (flag stripped), the `--spec` YAML (read from the parsed file),
   the real envelope, and a TODO note placeholder. The atom is the exact shape
   `xtask gen-howto` embeds from `docs/examples/`.

5. **Safety guard**: `build.rs` rejects `dev` + `PROFILE=release` → no relaxed
   binary ships. `--dev` and `--release` are mutually exclusive in xtask.

6. **Dev surface isolation**: xtask links carpenter *without* `dev`, so
   `app::cli()` it scrapes for the manual excludes `--capture-example` and the
   `(dev build)` version. The committed `howto.gen.md` and the inlined
   `SKILL.md` never carry dev surface. `howto_excludes_dev_surface` pins this.

7. **Test split**: the xtask drift tests (strict-only) are gated
   `#[cfg(not(feature = "dev"))]`; xtask carries a matching signal `dev` feature
   so `cargo test --workspace --features dev` skips them and runs the
   `core::dev` unit tests instead.

8. **`dev` command group** (`check` / `setup` / `clean` / `register` / `upgrade`,
   all cfg-gated): the dev operational surface. The agent holds no fs/uv
   permission directly — `dev setup`/`clean` own the `.sandbox` lifecycle and
   `dev check` probes uv. `dev register` / `dev upgrade` mirror the release
   `register` / `upgrade` in **name, behavior, and envelope shape** (they reuse
   `Data::Register` / `Data::Upgrade`): only the *target* differs (the repo's
   `.opencode/` instead of global `~/.config/opencode/`) and the *build*
   (`--features dev` instead of `--release`). `dev register` writes the skill
   only (it does not merge permission into the tracked `opencode.json` — the
   QA agent already carries `skill: allow`). Same semantics across
   stages; `.opencode/skills/carpenter/` is gitignored (machine-specific
   `current_exe` path).

### Alternatives rejected

- **Profile-based gating** (tie relaxation to debug vs release profile, via
  `debug_assertions`). Rejected: xtask regenerates docs from a *debug* build of
  carpenter, so the dev surface would leak into the committed manual. The
  feature is opt-in and xtask never opts in — that is what keeps the manual
  clean. (Also `debug_assertions` is a poor signal for "authoring mode.")
- **`dev capture` subcommand** that re-dispatches the target fn in-process.
  Rejected: requires refactoring `app::run` to expose dispatch-by-key, and
  re-parsing the target's args. The `--capture-example` flag instead reuses the
  *existing* dispatch (it runs the command normally and captures the rendered
  envelope afterward) — zero dispatch refactor, real envelope guaranteed.
- **A global env var (`CARPENTER_DEV`)**. Rejected: less discoverable than a
  Cargo feature, and `CARGO_FEATURE_DEV` is the natural signal `build.rs`
  already receives. The feature is the mechanism; the two xtask stages are the
  only surface a user needs.

## Consequences

+ The authoring chicken-and-egg is broken: build relaxed → run → capture → pass
  strict. The example atom is produced from a real run, not hand-guessed.
+ The strict guarantee narrows from "impossible to *compile* an undocumented
  command" (adr/007) to "impossible to *release* one" — a casual `cargo build`
  is still strict (safety net), but `--dev` is a deliberate, marked escape hatch.
  CI runs the strict build, so regressions are still caught before merge.
+ Dev surface is structurally invisible to release users and to the agent's
  in-context manual/skill — gated by `cfg` + isolated by xtask's feature-free
  link, with a drift test as a backstop.
+ − Small ongoing cost: a new command fn still needs its `#[test]` authored by
  hand (capture produces the example atom, not the test), and the TODO note line
  needs a human. Capture closes the envelope-fidelity gap, not the whole gate.
+ − Two cfg axes (feature `dev`, and the matching xtask signal feature) must stay
  in sync. Bounded: one is the mechanism, the other only disables the
  strict-only drift tests under `--features dev`.
