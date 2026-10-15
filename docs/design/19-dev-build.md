# Dev build stage (the authoring loop)

There are **two build stages** ([adr/016](../adr/016-dev-feature.md)):

| stage | command | gates | doc regen | profile |
|---|---|---|---|---|
| **dev** | `cargo xtask build --dev` | **relaxed** | no | debug |
| **release** | `cargo xtask build --release` | **strict** | yes (gen-howto + gen-specs) | release |

The everyday `cargo xtask build` (no flag) is the strict stage in a debug profile
(gen-howto + gen-specs + `cargo build`) — the canonical verification. Plain
`cargo build` is also strict (a safety net: an undocumented command still fails a
casual build). The relaxed behavior is **opt-in only** via `--dev`.

## Why two stages

The strict gates (adr/007, adr/013) make a command un-compilable until its
worked-example file, its `#[test]`, and (for scenarios) ≥3 distinct fns all
exist. That is the right default — it is how the docs stay drift-proof. But it
creates a chicken-and-egg for authoring: to write a *faithful* example you want
to run the command and capture its real envelope, which requires a binary, which
the gate refuses to produce until the example exists.

The dev stage breaks the loop: build relaxed, run for real, capture the atom,
then pass the strict build.

## The `dev` feature (mechanism)

`[features] dev = []` on the root package. Under it:

- `build.rs` skips the example-presence, test-presence, and scenario gates
  (`gate_scenarios`), and prints `cargo:warning=dev build: … relaxed`.
- `#![deny(missing_docs)]` is relaxed via `#![cfg_attr(feature = "dev",
  allow(missing_docs))]` (placed *after* the `deny` — `deny` is overridable by a
  later `allow`; only `forbid` is sticky).
- `src/core/dev.rs` compiles (it is `#[cfg(feature = "dev")]`), adding the
  `--capture-example <PATH>` global flag to `app::cli`.
- `--version` reports `… (dev build)` so an agent can detect a relaxed binary.

### Safety: dev never ships

`build.rs` rejects `--features dev` combined with `PROFILE=release`:

```
error: the `dev` feature relaxes the doc/example/scenario gates and must not be
used in a release build (adr/016)
```

So `cargo build --features dev --release` aborts. `cargo xtask build --release`
(the ship command) never enables `dev`.

### Dev surface never reaches the docs

`xtask gen-howto` calls `app::cli()`. xtask links carpenter **without** `dev`
(`xtask/Cargo.toml` enables no such feature), so the manual it generates excludes
`--capture-example` and the `(dev build)` version. The committed
`src/howto.gen.md` — and the `SKILL.md` that inlines it — therefore never carry
dev surface. `howto_excludes_dev_surface` pins this invariant.

## `--capture-example <PATH>` (dev only)

A global flag, present only in a dev build. It runs the command **normally** (the
real envelope still prints on stdout) and additionally writes the worked-example
atom — the exact shape `xtask gen-howto` embeds from `docs/examples/` — to
`<PATH>`. The atom contains:

- the invocation line (`carpenter …`, with `--capture-example` itself stripped);
- the `--spec` YAML block, read from the file the command parsed (omitted for
  commands without `--spec`, or for stdin specs);
- the real result envelope;
- a `<!-- TODO: author the behavioral note -->` placeholder (the one piece a run
  cannot derive — a human/LLM fills it).

### The loop

```sh
cargo xtask build --dev                                       # 1. relaxed binary
./target/debug/carpenter -c ds lesson create --spec lesson.yaml \
    --capture-example docs/examples/lesson/create.md          # 2. run + capture atom
# 3. add `#[test] fn create_*`; author the TODO note line
cargo xtask build                                             # 4. strict verify (now passes)
```

Step 4 (strict) is where the atom/test are validated. `cargo xtask build
--release` does the same with optimization for a final ship check.

> **Target footgun:** the dev and strict debug builds share `target/debug/`, so a
> later `cargo build` (strict) overwrites the dev binary — re-run
> `cargo xtask build --dev` before the next `--capture-example`. Inherent to
> Cargo (same dir + profile), not a carpenter bug.

## The `dev` command group (dev only)

All five leaves are `#[cfg(feature = "dev")]` (never in a release binary, the
howto, or the skill). The agent holds **no** fs/uv permission directly — every
sandbox/skill lifecycle op goes through the CLI.

| cmd | behavior |
|---|---|
| `dev check` | probe prerequisites (uv via `core::exec::uv_available`); `status:ok` + `checks[]`. |
| `dev setup` | `create_dir_all(./.sandbox)`; idempotent. |
| `dev clean` | `remove_dir_all(./.sandbox)`; idempotent (`removed:false` if absent). |
| `dev register` | render the carpenter skill (`core::skill::render()`) and write `.opencode/skills/carpenter/SKILL.md`. The dev analog of release `register` (skill write only — it does **not** touch the tracked `opencode.json`; the QA agent already carries `skill: allow`). |
| `dev upgrade` | `cargo build --features dev` (rebuilds `target/debug/carpenter`) + `dev register` (refresh the local skill). The dev analog of release `upgrade`: rebuild + refresh skill. |

`dev register` / `dev upgrade` reuse the release `Data::Register` / `Data::Upgrade`
variants — same command names, same envelope shapes across stages; only the target
(global prod → local dev) and the build (`--release` → `--features dev`) differ.

## Testing

- Strict suite (`cargo test --workspace`): the drift tests
  (`howto_gen_md_is_fresh`, `specs_marker_regions_are_fresh`,
  `howto_includes_scenarios`, `howto_excludes_dev_surface`) run and assert the
  committed surfaces match a strict-view regeneration. `core/dev.rs` is not
  compiled.
- Dev suite (`cargo test --workspace --features dev`): the drift tests **skip**
  (they are strict-only — xtask carries a matching signal `dev` feature so
  `#[cfg(not(feature = "dev"))]` disables them); the `core::dev::tests` unit
  tests run.

## What is NOT relaxed

Even in dev, the runtime invariants hold: one envelope on stdout, `--force`
never prompts, atomic writes, the verification-only helper guardrail. The dev
feature relaxes only the *compile-time documentation gates* — never correctness.
