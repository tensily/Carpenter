# 028 — Terminal UI: ratatui behind the `tui` feature, dev-first rollout

Date: 2026-10-16

## Context

carpenter's only interface is the JSON-envelope CLI driven by an agent
(adr/006, adr/009). Course building through that surface works but is
agent-mediated: a learner who wants to build a course themselves has no direct
UI. An HTML wireframe of a course-builder TUI was prototyped and reviewed
(screens: home, intake, review, board, lesson+feedback, pivot; light-minimal
theme; `c-*` component inventory), then superseded by building the real thing.

Constraints that shape the decision:

- **No embedded LLM** (adr/001): the UI collects specs and runs deterministic
  commands; it never generates content.
- Notebooks are the rendered artifact and stay the learner's viewing surface
  (VS Code) — the TUI is a build cockpit, not a study surface.
- The repo is a single release binary with channel-governed install identity
  (adr/027); a second binary would touch release tarballs, `install.sh`, and
  the smoke lanes.
- `build.rs` gates every `pub fn -> Result` in `src/commands/` on a worked
  example + paired test (adr/007), and the scan is not cfg-aware — the `dev`
  group lives in `core/dev.rs` precisely to stay out of that scan (adr/016).
- Unfinished work lands dark behind a feature flag, never on a long-lived
  branch (adr/016).

## Decision

Build the TUI as a **`carpenter tui` subcommand** using **ratatui +
crossterm**, compiled only under a new **`tui` feature** (optional deps;
`dev = ["tui"]`):

1. **Subcommand, not a second binary.** One binary keeps release tarballs,
   `install.sh`, the smoke lanes, and the install-identity model untouched.
   The ship PR later flips the gate; nothing else in the release flow moves.
2. **Feature-gated dark landing.** `tui = ["dep:ratatui", "dep:crossterm"]`,
   `dev = ["tui"]`. Release builds never compile the TUI; the clap group is
   cfg-gated out of the `--help` scrape, so howto/`SKILL.md` are untouched
   until the ship PR. This is the adr/016 path applied to a feature instead of
   the authoring loop.
3. **`src/tui/` module, not `src/commands/`.** The build.rs scan is cfg-blind;
   a cfg-gated command fn in `commands/` would still owe the adr/007 gates in
   release builds. The dev group's precedent: keep dev-only surface out of the
   scanned directory. When the TUI ships, its entry moves into `commands/` and
   acquires the gates.
4. **In-process command reuse.** Screens call `commands::*` directly and
   render `Data`; no subprocess, no envelope parsing. The DB stays the source
   of truth (reload on entry; never cache across actions). Validation and
   error codes are shared by construction.
5. **stdout stays envelope-only.** The TUI renders to stderr; raw mode and the
   alternate screen restore via a Drop guard on every path; `app::run` prints
   the one JSON envelope after exit. Agents driving `carpenter` can never see
   TUI frames on stdout. Terminal IO failures map to `StoreError` (the
   environment-failure class); in-app command errors are the status line.
6. **Tests compile under a CI lane.** ratatui `TestBackend` buffer assertions
   + store-backed fixtures; CI gains `cargo test/clippy --features tui` lanes
   so the gated code is checked without shipping it.

## Consequences

- ratatui/crossterm join the lockfile as optional deps; default release builds
  do not compile them.
- The howto/specs generated surfaces are unchanged while the gate is on; the
  ship PR must add the adr/007 atom + test and regen.
- Screen work parallelizes against the `Screen` trait contract
  ([design/20](../design/20-tui.md)) — one unit per screen, skeleton first.
- The HTML wireframe is retired; its distilled contract lives in design/20.
