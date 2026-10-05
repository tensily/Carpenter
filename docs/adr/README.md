# ADRs

Architecture decisions — context, decision, consequences. Numbered; gaps in the
sequence are retired decisions (not listed). Read topically; start with the index.

| # | file | topic |
|---|------|-------|
| 001 | [001-rust-sqlite.md](001-rust-sqlite.md) | Rust binary + SQLite store, no embedded LLM |
| 002 | [002-db-source-of-truth.md](002-db-source-of-truth.md) | DB is the source of truth; notebooks are rendered |
| 003 | [003-howto-buildstep.md](003-howto-buildstep.md) | howto generated at build time from the clap surface |
| 004 | [004-build-install-split.md](004-build-install-split.md) | build / install / upgrade split |
| 006 | [006-skill-integration.md](006-skill-integration.md) | integrate via a SKILL.md, not custom tools |
| 007 | [007-compile-enforced-command-docs.md](007-compile-enforced-command-docs.md) | compile-enforced command self-documentation (missing_docs + build.rs scan) |
| 008 | [008-specs-generated-from-types.md](008-specs-generated-from-types.md) | specs generated from `*Spec`/`Data` types (region-with-markers) |
| 009 | [009-skill-assembled-from-fields.md](009-skill-assembled-from-fields.md) | SKILL.md assembled from code fields, no template |
| 010 | [010-live-check-state.md](010-live-check-state.md) | live check state via `pass_or_fail` (no `attempts` table); helper rw |
| 011 | [011-skip-db-authored.md](011-skip-db-authored.md) | skip is DB-authored + rendered (not notebook-parsed) |
| 012 | [012-cross-platform-paths.md](012-cross-platform-paths.md) | per-OS paths via `core/platform.rs` (`#[cfg(target_os)]`) + 3-OS CI |
| 013 | [013-compile-enforced-scenarios.md](013-compile-enforced-scenarios.md) | compile-enforced multi-command scenarios (≥3 distinct fns per `examples/*.md`) |
| 014 | [014-yaml-spec-input.md](014-yaml-spec-input.md) | YAML is the single `--spec` format (no JSON code path; `serde-yml`) |
| 015 | [015-reference-solution-verify.md](015-reference-solution-verify.md) | author reference `solution` field + `lesson verify` (answer-key lock) |
| 016 | [016-dev-feature.md](016-dev-feature.md) | `dev` build stage (relaxed gates) + `--capture-example` authoring loop |
| 017 | [017-concurrency-ords-slugs-exec.md](017-concurrency-ords-slugs-exec.md) | concurrency semantics: atomic+unique lesson ords, slug validation, per-course execution lock |
| 018 | [018-upgrade-fetches-release.md](018-upgrade-fetches-release.md) | `upgrade` defaults to fetching a published release (checksum-verified) |
| 019 | [019-uninstall-semantics.md](019-uninstall-semantics.md) | `uninstall`: best-effort skill removal, self-delete binary, keep config unless `--purge-config` |
| 020 | [020-branch-governed-channels.md](020-branch-governed-channels.md) | branch-governed release channels: `pre-release`→`edge` canary, `release`→stable `vX.Y.Z` (Latest) — superseded by 021 |
| 021 | [021-nightly-main-channels.md](021-nightly-main-channels.md) | nightly + main channels: owner-gated `nightly` trunk, frozen `main`, PR ground rules + dev-validate gate |
| 022 | [022-automated-version-ladder.md](022-automated-version-ladder.md) | automated version ladder: patch per nightly merge, minor per promotion, post-promotion recut — superseded by 025 |
| 023 | [023-ruleset-bypass-actors.md](023-ruleset-bypass-actors.md) | ruleset-only branch protection on both trunks; explicit bypass actors (owner, `github-actions[bot]` as User, release-bot App) |
| 024 | [024-install-consent-and-banner.md](024-install-consent-and-banner.md) | installer consent plan + branded banner (channel-correct tagline, deck palette, non-interactive lanes proceed) |
| 025 | [025-standard-release-flow.md](025-standard-release-flow.md) | standard human-bump flow: CI commits nothing to the trunks, holds no credentials; ladder retired |