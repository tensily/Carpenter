# ADR-026: prune merged branches on promotion

Date: 2026-10-07 · Status: Accepted

## Context

The branch ground rules (adr/021) say short-lived `ivan/<topic>` branches are
deleted after merge — but nothing enforced it, so merged head branches
accumulated on the remote across promotions. Cleanup was a manual
`git push origin --delete` sweep after every `nightly → main` merge, and the
only person who knew which branches were safe to drop was whoever ran it.

## Decision

A `prune` job at the end of `release.yml` deletes every branch head that
reached nightly through a merged PR, automatically, when a promotion lands:

- **Trigger** — after `release` succeeds on a push to `main` (plus
  `workflow_dispatch` on main for retrying a partial prune without
  re-publishing). The trigger is the event that makes branches disposable:
  the promotion.
- **Selection is PR-based, not ancestry** — `gh pr list --base nightly
  --state merged` yields the candidate head branches. Ancestry
  (`git merge-base --is-ancestor`) would miss squash/rebase merges, whose
  commits are not ancestors of the merged result. PR-based selection also
  makes the safety argument: a pruned tip is by definition a commit of
  nightly's, so deleting the ref loses nothing.
- **Exclusions** — `nightly` and `main` never; nor any branch that is the
  head of an open PR (an earlier PR merged, work continued under a new PR →
  the branch survives until that PR resolves too). Multiple merged PRs per
  head branch dedupe; a ref that is already gone is a warning, not a failure
  (idempotent re-runs).
- **Actor is GITHUB_TOKEN** — deleting a ref on an unprotected branch needs
  only `permissions: contents: write`, which the workflow already holds.
  This keeps adr/025's rule intact: CI commits nothing to a protected branch
  and holds no bypass credentials. Pruning touches only disposable
  feature-branch refs.
- **Auditability** — every keep/delete decision is a `::notice` in the run
  log: the log is the record of what was pruned and why.

## Consequences

+ Branch hygiene is enforced by the promotion pipeline instead of memory;
  "delete the branch after merge" stops depending on a human.
+ Zero new secrets or credentials — the job runs on the workflow's existing
  token within adr/025's constraints.
+ The keep list is visible before anything is deleted — a bad selection is
  diagnosable from the log after the fact.
− A branch merged into nightly with no PR is invisible to the sweep (PR-based
  selection); direct merges are rare by policy (adr/021 requires PRs), and
  the ancestry-based union was rejected as complexity for a case the ground
  rules already forbid.

## Rejected

- **Release-bot App token for deletions** — the first draft used the App as
  an adr/023 bypass actor; adr/025 retired the App and the bypass model
  entirely. Reintroducing a credential-holder to delete unprotected refs
  would buy nothing GITHUB_TOKEN can't do.
- **Git-ancestry selection** — cheap, but silently spares squash-merged
  branches, which is exactly the mode PRs use; the safe-looking option was
  the incomplete one.
- **Scheduled (cron) sweep** — decoupled from promotions, so it could prune
  while work is mid-flight with no promotion to justify it. The trigger
  should be the event that makes branches disposable.
- **Delete-on-merge via nightly-push hook** — tempting, but a branch merged
  into nightly is still the live head of follow-up PRs; the open-PR
  exclusion needs the full PR list the promotion-time job already gathers.
