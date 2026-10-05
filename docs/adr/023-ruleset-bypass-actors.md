# ADR-023: Ruleset branch protection with explicit bypass actors

Date: 2026-08-30 · Status: Accepted (revised same day after the live drill)

## Context

adr/021 documented branch protection on both trunks; reality was softer.
`main` carried *two* drifting layers — classic branch protection (review
count 0, non-strict checks) and an active ruleset (count 1, strict) — and
neither listed the `guard` job as required, so "only `nightly` merges into
`main`" went red but never blocked a merge. `nightly` had no protection at
all (adr/022's constraint 3 leaned on that: the owner direct-pushes
generated-surface fixes). Meanwhile three automation pushes *must* keep
landing on `nightly` directly: the patch ladder's `chore(release): bump`
commits, the promotion minor bump, and the post-promotion recut
fast-forward — plus the owner's merges and drift-fix pushes.

A `pull_request` rule blocks every direct push unless the pusher is a bypass
actor — protection that forgets the bots breaks the adr/022 ladder on the
first merge.

## Decision

Rulesets only, one per trunk:

- **`trunk protection (main)`** (id 20971401, `~DEFAULT_BRANCH`): PR rule
  (1 code-owner approval), 8 required checks — the seven lane contexts plus
  `guard (only nightly merges into main)`, making the channel rule
  merge-blocking — strict up-to-date, no force-pushes, no deletions. Owner
  bypass: `pull_request` (merges via PR only).
- **`trunk protection (nightly)`** (id 21858277, `refs/heads/nightly`):
  PR rule (1 code-owner approval — with `CODEOWNERS * @meolord29`, every
  merge is owner-approved), the 7 lane checks (strict), no force-pushes, no
  deletions.
- **Bypass actors on `nightly`**: the owner (User 53997283, `always`) and
  the release-bot App (`Integration` 4769844, `always`) — see the post-drill
  revision: on a user-owned repo these cover human pushes only.
- Classic branch protection on `main` deleted; the ruleset already carried
  the stricter settings. One source of truth per trunk.
- **No-cascade by suppression, not token semantics.** App-token pushes fire
  workflows, so the `bump` job carries an `if` that skips it when a
  nightly push is bot-authored and its head commit is itself a
  `chore(release): bump` commit (the pushing run already built it); build /
  release / smoke depend on `bump` and skip with it. Bot-triggered runs
  also get their own `concurrency` group so a cascade run can never cancel
  the mainline run mid-build.

## Post-drill revision (same day)

The first cut of this ADR tried `github-actions[bot]` (User 41898282,
`always`) as a bypass actor so `bump`/`recut` could keep pushing with
`GITHUB_TOKEN`. The live drill (PR #33) falsified both halves of that:

- the API rejects `Integration` 15368 ("GitHub Actions") on user-owned
  repos ("must be part of the ruleset source or owner organization"), and
  the User-typed bot actor, while accepted, **does not match `GITHUB_TOKEN`
  pushes** — rules evaluate them as the Actions integration. The bump push
  was declined five times ("Changes must be made through a pull request"),
  breaking the patch ladder.
- a plain owner merge is also refused (sole code owner cannot self-approve)
  — bypassing is an explicit act (`gh pr merge --admin`), which is
  desirable: it makes "green before merge" a visible policy choice.

Fix: `bump` and `recut` switched to the release-bot App token (the standard
Integration-bypass pairing), with the suppression `if` above killing the
would-be cascade. Two documented behavior deltas:

1. promote-bump's *branch-push* run is suppressed too (its head is a bump
   commit) — the nightly channel no longer rolls at the pre-promotion
   minor; the promotion PR's checks still re-run via `synchronize`.
2. recut's push now fires one nightly run that resumes the patch ladder
   immediately at `X.(Y+1).1` (adr/022's own wording) instead of waiting
   for the next merge; that run's bump push is itself suppressed from
   cascading further.

A second drill round (PR #34's merge run) falsified the App-token fix as
well: ruleset evaluation attributes an App-token `git push` to the bot
account (`carpenter-release-bot[bot]`), and bot accounts match neither the
`Integration` bypass (4769844) nor a `User`-typed actor (322772521) — the
push was declined identically. On user-owned repositories, bypass lists
effectively cover **human users only** ("Actors may only be added to bypass
lists when the repository belongs to an organization" — GitHub docs). The
ladder's machine pushes therefore cannot bypass a check-gated ruleset on
this repo shape; `bump` and `recut` pushes are declined and the ladder is
broken until this ADR's mechanism is revised (org transfer, a
human-identity token for the ladder, or weaker nightly rules). The
owner-bypass merge path (`gh pr merge --admin`) works and is unaffected.

## Consequences

+ `nightly` merges are owner-only in mechanism, not convention: non-bypass
  actors can open PRs but cannot merge, push, force-push, or delete either
  trunk; even the owner must explicitly `--admin`-merge past the review
  requirement.
+ The ladder keeps its no-cascade property via one `if`; every merge bumps
  the patch, builds the bumped sha, and rolls the channel exactly once.
+ `recut`'s "recreates nightly if it was deleted" branch is now unreachable
  (deletion rule) — kept as harmless dead text.
− Bypass is per-actor all-or-nothing: the owner *can* merge with pending
  checks or push unverified commits; "green before merge" stays policy
  (visible in CI), not mechanism.
− The App's bypass means any workflow in the repo can push `nightly`
  (same-repo PR branches run workflows with `contents: write`); accepted in
  a solo repo, revisit before adding collaborators.
− Two actors must stay in sync with reality: the App must remain installed
  and its id correct in the ruleset's bypass list, or every ladder push
  fails loudly (visible within one merge).

## Rejected

- **`Integration` 15368 as bypass** — the API rejects it for user-owned
  repos; the ruleset UI only offers it on org repos.
- **`github-actions[bot]` as a User-typed bypass** — accepted by the API,
  silently ignored at push time (drill evidence above).
- **GITHUB_TOKEN for bump/recut** — incompatible with any PR/check rule on
  a user-owned repo; would force dropping merge-gating on `nightly` (the
  point of this ADR).
- **No PR rule on `nightly`** (direct pushes allowed) — guts owner-only
  merges.
- **Keeping classic protection on `main`** — two drifting layers; rulesets
  are the maintained path.

Extends [adr/021](021-nightly-main-channels.md) and
[adr/022](022-automated-version-ladder.md).
