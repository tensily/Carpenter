# ADR-009: skill assembled from code fields (no template)

Date: 2026-08-08 · Status: Accepted

## Context
The `SKILL.md` written by `register`/`upgrade` was previously a static string in
the binary, with a **second inline copy** in `design/15-opencode-integration.md` —
two hand-maintained copies that can drift. adr/006 made the skill lean and defer to
`howto`, but the body was still a monolithic prose blob / template.

## Decision
`core/skill.rs` owns **typed fields** and a renderer. There is no template file and
no monolithic prose block — the markdown is assembled by code.

Authored consts (the only hand-authored atoms; they exist nowhere else in code to
derive from):
- `NAME` — `"carpenter"`
- `DESCRIPTION` — the frontmatter matcher (authored once)
- `WHAT_THIS_IS`, `WORKFLOW`, `PEDAGOGY` — authored once each

Derived at render time:
- `version` — `CARGO_PKG_VERSION`
- `bin` — `std::env::current_exe`
- the howto-reference line — built from `NAME`

`write_skill()` composes frontmatter (serde/typed) + sections via `format!`;
`register` and `upgrade` both call it. `carpenter register --print-skill` emits the
rendered bytes for inspection/CI.

Gate: a `#[test]` asserting **render determinism** (re-render == same bytes) +
frontmatter validation (`name` matches `^[a-z0-9]+(-[a-z0-9]+)*$`, required fields
present). No committed artifact is diffed — determinism + validation is the gate.

`design/15` documents the field set + renderer and **stops inlining a body**.

## Consequences
+ Single source: the skill lives in code fields; `design/15` references rather than
  duplicates. No template file to keep in sync — the renderer is the only path.
+ Determinism + validation test catches renderer regressions without a committed
  artifact.
+ Uniform with the rest of the self-documentation design (adr/007, adr/008): the
  source is code; output is generated/tested.
− The `WHAT_THIS_IS` / `WORKFLOW` / `PEDAGOGY` content is still authored prose — but
as **named consts**, not a blob. It cannot be derived from other code; those ideas
exist nowhere else.

## Update (2026-08-10): skill embeds the full howto at render time

The skill body no longer *defers* to `carpenter howto` for the command surface — it
**inlines** the generated manual (`crate::manual::MANUAL`) into `SKILL.md` at render
time, under a `## Command manual` heading (the manual's own H1 is stripped to avoid a
double `#` heading).

This supersedes the "never duplicate the command list" sub-rule stated above. That rule
existed to prevent a **hand-maintained** second copy from drifting. Inlining the
generated artifact is *mechanical* duplication: the howto is still the single authored
source (assembled by `xtask gen-howto` from `clap` + `docs/examples/`); the skill simply
embeds it via `render()`, exactly as it already embedded `version` and `bin`. No human
keeps two things in sync — the gate is `render_is_deterministic` + the
`frontmatter_validates` assertions that the inlined per-command sections (`## plan`,
`### create`, `goal_index_`) are present.

Driver: agents had to run `carpenter howto` as a separate step to see command detail
(spec shapes like `goal_index_<i>` were invisible in the skill). Inlining removes that
round-trip. Accepted cost: `SKILL.md` is large (~hundreds of lines) and loads into the
agent's context on every carpenter turn — traded for always-on command detail. Easy to
reverse (drop the `{manual}` line in `render()`).

*(Revision 2026-10-15: the composing fn is `skill::render` — no `write_skill()`
exists; `upgrade` refreshes via `skill::register` only when the skill is already
registered, adr/018.)*
