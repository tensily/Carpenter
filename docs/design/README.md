# Design

carpenter is a Rust CLI an LLM agent drives to build Python/Jupyter learning
material. SQLite is the source of truth; notebooks render from it. Read in order:

| # | file | topic |
|---|------|-------|
| 01 | [overview.md](01-overview.md) | what carpenter is, domain model, IDs, status derivation, non-goals |
| 03 | [architecture.md](03-architecture.md) | module map + layering rules + stack (crates + tooling) |
| 04 | [storage.md](04-storage.md) | filesystem layout |
| 05 | [notebook-sync.md](05-notebook-sync.md) | render & idempotent sync contract |
| 06 | [helper.md](06-helper.md) | verification-only self-check helper |
| 07 | [compare.md](07-compare.md) | compare modes (single source) |
| 08 | [quiz-run.md](08-quiz-run.md) | nbconvert quiz execution |
| 09 | [agent-interface.md](09-agent-interface.md) | JSON envelope, errors, feedback loops |
| 10 | [cli-surface.md](10-cli-surface.md) | command groups |
| 11 | [data-flow.md](11-data-flow.md) | end-to-end flow |
| 12 | [testing.md](12-testing.md) | test strategy + gates |
| 14 | [build-order.md](14-build-order.md) | phased build plan (historical — completed) |
| 15 | [opencode-integration.md](15-opencode-integration.md) | skill-based agent-app integration (register/deregister) |
| 16 | [execution.md](16-execution.md) | course venv (uv) + lesson execute + venv-backed quiz run |
| 17 | [cross-platform.md](17-cross-platform.md) | Linux/macOS paths, `dirs`-only platform surface, CI matrix |
| 18 | [scenarios.md](18-scenarios.md) | multi-command scenario examples (gated, howto+skill-inlined) |
| 19 | [dev-build.md](19-dev-build.md) | dev build stage, `--capture-example`, + the `dev` command group (check/setup/clean/register/upgrade) (adr/016) |

Numbering gaps are merged docs (one concern per file otherwise): **02 stack** →
[03 → Stack](03-architecture.md#stack); **13 out-of-scope** →
[01 → Non-goals](01-overview.md#non-goals-yagni).

See also: [data-model/](../data-model/), [specs/](../specs/), [adr/](../adr/).
