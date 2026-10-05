# ADR-025: Human version bumps — retire the CI version ladder

Date: 2026-10-05 · Status: Accepted

## Context

adr/022's automated version ladder required CI to commit version bumps
directly onto `nightly` (patch ladder, promotion minor bump, post-promotion
recut). Under the trunk rulesets (adr/023) that meant CI identities needed
bypass. The org transfer (adr/023's bypass list was stripped) exposed how
fragile that foundation was:

- The **release-bot GitHub App** was re-created org-owned and given an
  `Integration` bypass entry with `always` — GitHub **silently did not honor
  it for git pushes** (the Sept 5 failures predated the transfer; the
  failures recurred identically after).
- Retried as **merge-time bypass** (bot opens `nightly-bump` → `nightly` PR
  and merges it): GitHub rejected that too — *"base branch policy prohibits
  the merge."*
- A built-in **GitHub Actions** integration bypass could not even be added
  (REST 422 "must be part of the owner organization"; not offered in the UI
  for this org).
- Bypass is honored only for **users/roles/teams**. The remaining option —
  a `tensily-release` machine user — works, but adds a long-lived credential
  to manage purely to serve the ladder.

Conclusion: the ladder's requirement (CI commits to a protected trunk) is
the anomaly. Standard OSS flows make version bumps **human decisions
delivered as ordinary PRs**; CI only reads the repo and publishes artifacts.

## Decision

1. **CI never commits to a protected branch and holds no bypass
   credentials.** The ladder (`bump`, `promote-bump`, `recut`), the
   `nightly-bump` scratch branch, and all release credentials
   (`RELEASE_BOT_*`, the release-bot App, `RELEASE_PAT`) are removed.
2. **Version bumps are human PRs**: `cargo xtask bump patch|minor` as a
   commit in a normal PR to `nightly` (merged by the admin), folded into a
   feature PR or a standalone release-prep PR.
3. **`nightly` push** → build → roll the `nightly` rolling prerelease,
   short-sha in the title. The artifact reports the last human-bumped
   version; builds are distinguished by sha.
4. **Promotion** (`nightly` → `main`, admin-merged) unchanged: `ci.yml`'s
   `guard` still requires the only-`nightly` channel and an exact
   main.minor+1.0.0 bump — the bump is now a human commit in that flow.
5. **`main` push** → build → publish immutable stable `vX.Y.Z` from
   `Cargo.toml`, as before.
6. **Post-promotion reconcile**: without `recut`, `nightly` stays behind
   `main` after a promotion. The next promotion's `guard` will demand
   main.minor+1 and fail with instructions — reconcile by merging `main`
   into `nightly` via a PR (admin), then bump.
7. Branch protection philosophy is unchanged (adr/021): every change to
   either trunk arrives as an admin-approved PR; no force-pushes; required
   checks gate merges.

## Consequences

- The release pipeline holds **zero secrets**; `permissions: contents:
  write` on `GITHUB_TOKEN` is its only publish power, and it cannot touch
  branch history.
- One human commit per release (the bump) replaces full automation — the
  standard trade.
- The release-bot App and its secrets can be uninstalled/deleted; the
  `Integration` bypass entries can be removed from both trunk rulesets.
