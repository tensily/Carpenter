# ADR-019: `uninstall` semantics

Date: 2026-08-18 · Status: Accepted (revised 2026-10-12)

## Context

`install`/`upgrade`/`register` had no inverse: removing carpenter meant manually
deleting `~/.local/bin/carpenter`, the opencode `SKILL.md` + permission key, and
(optionally) `~/.config/carpenter/config.json`. Supported platforms are Linux and
macOS only — Windows is explicitly out of scope (design/17), which makes
self-deletion tractable.

## Decision

`carpenter uninstall [--bin-dir <p>] [--purge-config]`:

- **Resolution:** `--bin-dir` → config `bin_dir` → default — identical to
  `install` (one resolution path, no drift).
- **Ordering:** skill first, binary last. The skill is recoverable via
  `register`; the binary delete is the point of no return, so it goes last and
  its failures abort before anything else is reported done.
- **Skill removal is best-effort** (mirrors `upgrade`'s `skill_outcome_for`):
  outcome JSON `{removed:true,app,path}` or `{removed:false,
  reason:"not_registered"}`; it never fails the run.
- **Self-delete:** plain `remove_file` on the running binary — safe on
  Linux/macOS, where unlink keeps the inode alive for the running process (the
  same property `upgrade`'s copy-replace and `strip_deleted` rely on). No
  rename dance needed because Windows (locked images) is unsupported.
- **Nothing found** (no skill file and no binary) → `NotFound` error envelope —
  mirrors `deregister`'s contract on an absent skill.
- **Config is kept by default;** `--purge-config` removes it (the apt
  `remove`/`purge` split — package managers that delete user config by default
  are the outlier). Course data (`<root>/courses`) is **never** touched: it is
  user-owned learning material, not installation state.

## Consequences

+ One command reverses the full `install.sh`/`register` footprint; the envelope
  reports exactly what was (`bin:null` when no binary existed) and wasn't
  (`config_purged:false`) removed.
+ `Data::Uninstall` joins spec 18's generated table via the `models::build`
  example rows (adr/008); envelope-smoke coverage comes with them.

− A re-install after `uninstall` without `--purge-config` finds stale config
  (intended — settings survive reinstall).

## Revision — confirmation plan, purge target, removability parity (2026-10-12)

QA on PR #55 (branch installs) found four defects in the self-removal surface;
this revision amends the decision without overturning it:

- **Consent plan (mirrors [adr/024](024-install-consent-and-banner.md),
  inverted for destruction).** `self remove`, `self uninstall`, and
  `self deregister` print a removal plan to **stderr** — binary, per-app
  skill paths actually registered, the config path, and what is kept — then
  read one confirmation line from **stdin**: explicit `y`/`Y` proceeds; Enter,
  any other answer, or EOF declines with a `Conflict` envelope and nothing is
  removed. An installer may proceed unattended (additive); a remover may not
  (destructive) — no stream to confirm from means abort. `--yes` is the
  non-interactive bypass (the CLI-side sibling of `CARPENTER_INSTALL_YES`).
  The plan never touches stdout, so the one-JSON-envelope contract holds.
- **`--purge-config` targets the removed tag's config**
  (`<config base>/<tag>/config.json`), not the calling install's — it
  previously resolved the caller's `config_file()`, so a tagged target's
  config could never be purged. IO failures now propagate as error envelopes
  instead of silently reporting `config_purged:false`; `config_purged:false`
  hence means exactly "nothing to purge".
- **Removability parity with discovery.** `NotFound` now requires *all three*
  legs absent (registered skill, binary, data dir) — a data-dir-only install
  (e.g. a `carpenter-release-bot` config leaf) is listed by `self list` and
  removes cleanly, reporting per-leg outcomes with the data dir kept. The
  data dir itself is never deleted (course data is user-owned, unchanged).
- **Skills are machine-global per tag** ([adr/027](027-install-identity-from-binary-stem.md)):
  one skill dir per app per tag, independent of `--bin-dir`, which scopes
  only the binary. This was always true; the removal plan now states it and
  enumerates the real paths before confirmation, so a sandboxed-looking
  invocation can never silently remove a live skill.
- **`self install` reports `already_present`** when it re-copies over an
  existing binary — observability only; install behavior is unchanged.

Consequences of the revision:

+ Destructive self-ops are safe to hand to agents: the default (and the
  non-interactive default) is decline.
+ The plan shown and the removal executed derive from one discovery pass —
  they cannot disagree.
− Scripted removals must pass `--yes` (installer script, release smoke) —
  caught by the `tests/uninstall.rs` CLI-level confirmation tests.
