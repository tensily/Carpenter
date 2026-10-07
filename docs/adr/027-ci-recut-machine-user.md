# ADR-027: CI-owned nightly recut via the release machine user

Date: 2026-10-07 · Status: Accepted · Amends [adr/025](025-standard-release-flow.md)

## Context

The release cycle (adr/021) is: PRs group into `nightly` → a promotion PR
merges the group into `main` → the trunk should start clean for the next
cycle. But promotion merge #51 left `nightly` behind `main` with severed
ancestry (promotions squash-merge), and adr/025's answer — §6, reconcile by
hand — made trunk cleanup a recurring human step. The owner wants the whole
cycle machine-driven: promotion lands → CI cleans up `nightly` and recreates
it at `main`'s head → new PRs group into it → next promotion.

adr/025 removed every CI bypass credential, and for good reason: the
release-bot App's bypass was silently not honored for git pushes, and
merge-time bypass was rejected by branch policy. It also named the one
mechanism that works: a dedicated machine user. This ADR spends that option
on the single operation that cannot be done any other way.

## Decision

1. **A `recut` job at the end of `release.yml`** recreates `nightly` at
   `main`'s head after a promotion publishes: force-push `main` →
   `refs/heads/nightly`. A force-push (ref update), never a delete +
   recreate — the ruleset forbids deletions, and an update achieves the same
   state.
2. **The push runs as the `tensily-release` machine user** — the one
   sanctioned bypass credential: a ruleset bypass actor on `nightly`
   (User, always), authenticated by a fine-grained PAT stored as the
   `RECUT_TOKEN` secret, scoped to this repository and contents-only.
   GITHUB_TOKEN stays powerless over trunk refs (adr/025's rule holds for
   everything else).
3. **Safety rails before the force-push**: skip when `nightly` already
   contains `main`'s head; fail loudly when `nightly`'s *tree* differs from
   `main`'s — that is unmerged work, not the harmless dangling
   squash-originals a promotion leaves behind. Recut replaces history, so it
   must only ever run when no content would be lost.
4. **Ordering**: `recut` runs after `release`, before `prune` (adr/026) —
   the trunk is clean before merged branches are swept. The cascade
   nightly-push run rolls the `nightly` prerelease at the new head, so the
   canary and stable point at the same content.
5. **Idempotent and dispatchable**: skips silently on re-runs; runs on
   `workflow_dispatch` against `main` to recut a stale trunk (or a missed
   run) without re-publishing. A missing `RECUT_TOKEN` is a loud
   configuration error, not a silent skip.

## Consequences

+ The full cycle is CI-driven: promotion → publish → recut → prune, with the
  trunk recreated for the next group of PRs. No recurring human step.
+ The credential is minimal and impersonal: one fine-grained PAT, one repo,
  contents-only, owned by a dedicated account — an owner PAT would tie
  release automation to a personal login.
− adr/025's zero-credential rule is violated in exactly one place, by one
  actor, for one ref. That narrowness is the point: `tensily-release` can
  force `nightly` and nothing else.
− A PAT is long-lived; expiry must be tracked (fine-grained PATs cap at a
  year). An expired token fails the recut job loudly at the next promotion,
  not silently.

## Rejected

- **GitHub App (the old release-bot)** — proven broken in this org: its
  `Integration` bypass was silently not honored for git pushes, and
  merge-time bypass was rejected outright (adr/025's Context).
- **Owner PAT** — works, but couples release automation to a personal
  account; the machine user exists precisely to avoid that.
- **PR-based reconcile (adr/025 §6, automated)** — CI can open a
  `main → nightly` PR, but merging it still needs a human admin click; that
  is the recurring human step this ADR exists to remove.
- **`cargo xtask recut` (human-run command)** — automates the mechanics but
  leaves the trigger human; rejected once the owner chose full CI ownership
  of the step.
