# ADR-018: `upgrade` fetches the published release

Date: 2026-08-17 · Status: Accepted

## Context

`upgrade` was source-only (adr/004): resolve `--source` → config `source_dir` →
`ValidationError`; rebuild via `cargo xtask build --release`. That fit
source-checkout users, but `.github/workflows/release.yml` now publishes a
checksum-verified `edge` release for every push to `main`, and
`scripts/install.sh` installs it — so binary-installed users had **no upgrade
path** (`upgrade` errored without a checkout; the installer's "update" story was
"re-run install.sh", which never updated the skill file).

ADR-004 also deliberately kept "git out of upgrade" — fetching a release does
not reintroduce git: it fetches **tarballs over HTTPS with checksum
verification**, not a clone.

## Decision

`upgrade` gains a **release mode** and becomes the default:

- **Mode resolution:** `--source <p>` → config `source_dir` → **release mode**.
  Source users keep their flow untouched; binary users get self-update.
- **Release pipeline** (`core/release.rs`): map the running platform to an asset
  (`x86_64-unknown-linux-musl`, `aarch64-apple-darwin`; anything else →
  `ValidationError` pointing at `--source`) → `curl` tarball + `SHA256SUMS` →
  verify via `sha256sum`/`shasum -a 256` → `tar -xzf` → probe the new binary
  (`--version` — it must execute before anything is replaced) → atomic
  tmp+rename into the bin dir. Subprocess tools (`curl`, `tar`, checksum),
  no HTTP/sha2 crates — the pipeline mirrors `scripts/install.sh` line for line,
  so the two paths cannot drift. `CARPENTER_DOWNLOAD_BASE` overrides the URL
  (tests/mirrors).
- **Skill:** release mode **always (re-)registers** (`skill::register`) —
  installer parity (`install.sh` auto-registers when opencode exists), and it
  makes `upgrade` the one-command update for skill + binary. Source mode keeps
  the adr/004 contract: refresh only if registered (`not_registered` warning
  otherwise). `--no-skill` still yields `skill:null`.

## Consequences

+ Binary-installed users self-update with `carpenter upgrade`; the `skill` file
  ships the new embedded howto with it.
+ `Data::Upgrade.source` now carries the origin (source dir or tarball URL) —
  no envelope-shape change; spec 18 regenerated.
+ amd64-musl + Apple Silicon covered; Intel Mac / Windows / linux-arm64 get a
  clear `ValidationError` naming `--source` (matches installer gate).

− `upgrade` now needs `curl` + `tar` + a checksum tool on PATH in release mode
  (macOS/Linux always have them; a missing tool is a clear `StoreError`).
− Rollback safety: a failed download/verify/probe aborts **before** the binary
  is touched; a failed skill write never rolls back a successful binary swap
  (unchanged).

## Rejected

- **In-process HTTP (`ureq` + `sha2`)** — two new supply-chain deps to avoid
  subprocess tools every target platform already ships; also risks drifting
  from `install.sh`.
- **`gh`/GitHub-API aware upgrades** — needs auth for some flows; plain release
  URLs are anonymous and mirror-able via `CARPENTER_DOWNLOAD_BASE`.
- **Pinning versions / channels beyond `edge`** — YAGNI until versioned
  releases exist.

## Update (2026-08-27)

The skill rule: **both** modes now refresh only a **registered** skill (skill
file present) and never register a new one — release mode no longer
force-registers opencode. Parity is preserved with the installer, which
registers only what the user confirmed (non-interactive lanes auto-register a
detected opencode). `skill` is the refresh outcome when registered. See
[adr/006](006-skill-integration.md) update.

## Update (2026-08-27, channels)

The release fetched by release mode is now channel-governed (adr/020): default
`--channel stable` follows the **Latest** release (`releases/latest/download`)
published from the `release` branch; `--channel edge` follows the rolling
prerelease this ADR introduced. `edge` is no longer the default anywhere.
The `edge` tag itself stopped rolling on every push to `main` — it now rolls on
pushes to `pre-release`.

## Update (2026-08-29, nightly)

adr/021 renamed the canary: `--channel nightly` follows the rolling prerelease
published from the `nightly` branch (`edge` is rejected); stable still follows
**Latest**, now published by the nightly → main promotion merge.
