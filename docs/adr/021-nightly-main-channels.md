# ADR-021: nightly + main release channels

Date: 2026-08-29 · Status: Accepted

## Context

ADR-020 governed publishing with three long-lived branches (`main` trunk →
`pre-release` → `release`) and an `edge` canary tag. In practice that means two
promotion hops, three protected branches, and a naming scheme users and
contributors must keep straight. On top of that, the owner wanted merge approval
into the canary channel to be his alone, and every merge into the canary to
clear an explicit quality bar: tests green, tests for new features, an updated
QA-agent checklist when the surface moves, a plain explanation of the feature,
and a QA report proving the subject-learning simulation
still runs smoothly.

## Decision

Two long-lived **channel branches** govern publishing (`release.yml`):

```
ivan/<topic> ──PR──▶ nightly   integration trunk; ground rules + owner-only approval;
                               push rolls the rolling `nightly` prerelease (canary)
nightly ──PR──▶ main           promotion; publishes immutable stable vX.Y.Z (Latest)
```

- Feature branches PR into `nightly` only. `CODEOWNERS` (`* @meolord29`) plus
  branch protection (required codeowner review) makes every merge into
  `nightly` owner-approved.
- `main` is frozen: ci.yml's `guard` job fails any PR into `main` whose head is
  not `nightly`, so `nightly` is the only path to stable.
- Push to `nightly` re-creates the rolling **`nightly`** prerelease (canary).
  Push to `main` (a promotion merge) publishes immutable **stable `vX.Y.Z`**
  from `Cargo.toml` (bumped in the promotion PR); GitHub's **Latest** pointer
  follows it. Each release attaches a channel-correct installer (stable's is
  tag-patched, so it can never fetch nightly bits).
- PR **ground rules** (`.github/PULL_REQUEST_TEMPLATE.md`, verified by the owner
  at review):
  1. all unit tests pass;
  2. new features ship with unit tests;
  3. the QA agent checklist/prompt is updated when the CLI
     surface or study workflow changes;
  4. the PR explains what the feature does;
  5. a QA report is attached — the learning simulation ran
     smoothly over existing and new features.
  The validate gate is deliberately manual: the agent is interactive and
  LLM-driven, so CI cannot run it honestly; the owner's review is the
  enforcement.
- The README is end-user-only (stable install + getting started, no channel
  talk); the nightly install path lives in docs/README.md. `upgrade` renames
  `--channel edge` → `--channel nightly` (`core/release.rs::Channel`); `edge`
  is rejected.

## Consequences

+ One promotion hop instead of two; two protected branches instead of three.
+ Owner-gated canary with an explicit, checklist-visible quality bar.
+ Users never see nightly — the README stays install-and-go.
+ "Only nightly merges into main" is CI-enforced (`guard`), not convention.
− The canary trunk (`nightly`) drifts from stable between promotions; the
  promotion PR must absorb any drift.
− The validate gate depends on human discipline (checklist + review), not CI.
− Renaming `edge` ripples through installer/upgrade/docs surfaces (one-time).

## Rejected

- **Timed/cron nightly cuts** — still rejected (as in adr/020): a merge encodes
  "this is ready" better than a clock. `nightly` is the channel name, not a
  schedule.
- **Automating the QA agent in CI** — needs an LLM key, is slow and
  non-deterministic, and the agent is interactive by design; a CI rerun would
  test something other than what the report requires.
- **Keeping `edge` as an alias** — two names for one channel; no clarity gain.

## Update (2026-08-30, adr/022)

Versioning is now automated: every nightly merge bumps a patch, every
promotion bumps a minor (a release-bot GitHub App lands it on PR open, the
guard enforces it), and a recut job fast-forwards `nightly` to the promotion
merge. See [adr/022](022-automated-version-ladder.md).

*(Revision 2026-10-05: reverted by [adr/025](025-standard-release-flow.md) —
the ladder is retired; versions are bumped by humans via `cargo xtask bump`
and CI commits nothing.)*

Supersedes [adr/020](020-branch-governed-channels.md).
