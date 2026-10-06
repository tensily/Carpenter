# ADR-025: prune merged branches on promotion

Date: 2026-10-07 · Status: Accepted

## Context

The branch ground rules (adr/021) say short-lived `ivan/<topic>` branches are
deleted after merge — but nothing enforced it, so merged head branches
accumulated on the remote across promotions. Cleanup was a manual
`git push origin --delete` sweep after every `nightly → main` merge, and the
only person who knew which branches were safe to drop was whoever ran it.

## Decision

A `prune` job at the end of `release.yml` (adr/022's pipeline) deletes every
branch head that reached nightly through a merged PR, automatically, when a
promotion lands:

- **Trigger** — after `release` and `recut` succeed on a push to `main`
  (plus `workflow_dispatch` on main for retrying a partial prune without
  re-publishing, the same retry path recut has). Running *after* recut is the
  safety ordering: nightly has been fast-forwarded to the promotion merge, so
  trunk provably holds every commit being pruned.
- **Selection is PR-based, not ancestry** — `gh pr list --base nightly
  --state merged` yields the candidate head branches. Ancestry
  (`git merge-base --is-ancestor`) would miss squash/rebase merges, whose
  commits are not ancestors of the merged result.
- **Exclusions** — `nightly` and `main` never; nor any branch that is the
  head of an open PR (an earlier PR merged, work continued under a new PR →
  the branch survives until that PR resolves too). Multiple merged PRs per
  head branch dedupe; a ref that is already gone is a warning, not a failure
  (idempotent re-runs).
- **Actor** — deletions go through the release-bot App token
  (`actions/create-github-app-token`), the sanctioned bypass actor of
  adr/023, so the job never fights the rulesets even if they are later
  widened past the two trunks. Requires the App to hold contents:write +
  pull-requests:read.
- **Auditability** — every keep/delete decision is a `::notice` in the run
  log: the log is the record of what was pruned and why.

## Consequences

+ Branch hygiene is enforced by the promotion pipeline instead of memory;
  "delete the branch after merge" stops depending on a human.
+ The keep list is visible before anything is deleted — a bad selection is
  diagnosable from the log after the fact.
− A branch merged into nightly with no PR is invisible to the sweep (PR-based
  selection); direct merges are rare by policy (adr/021 requires PRs), and
  the ancestry-based union was rejected as complexity for a case the ground
  rules already forbid.
− Deletion rides on the release-bot App's scopes; widening or narrowing App
  permissions later must keep pull-requests:read + contents:write.

## Rejected

- **Git-ancestry selection** — cheap, but silently spares squash-merged
  branches, which is exactly the mode PRs use; the safe-looking option was
  the incomplete one.
- **Scheduled (cron) sweep** — decoupled from promotions, so it could prune
  before `recut` has absorbed a merge, and a red release would not stop it.
  The trigger should be the event that makes branches disposable.
- **Delete-on-merge via nightly-push hook** — tempting, but a branch merged
  into nightly is still the live head of follow-up PRs mid-promotion; the
  open-PR exclusion needs the full PR list the promotion-time job already
  gathers.
