# 20 — Terminal UI (`carpenter tui`)

The interactive course-builder TUI: a ratatui/crossterm screen set over the
deterministic command fns. Experimental — compiled only under the `tui`
feature (enabled by `dev`), so a release binary, the `--help` scrape, the
generated howto, and the inlined `SKILL.md` never see it. Decision record:
[adr/028](../adr/028-terminal-ui.md).

## Stance

Same as the CLI itself: **no embedded LLM**. The TUI collects intake/spec via
forms and runs deterministic commands (`course create`, `plan create/confirm`,
`lesson create`, `notes add`, `plan update`); viewing lessons stays in VS Code
(notebooks are the rendered artifact); the pivot loop is human-driven.

## Architecture

- `src/tui/` — the UI (never `src/commands/`, whose files build.rs scans by
  command-fn signature). Entry: `tui::run(paths) -> Result<Data, _>`.
- Screens call `commands::*` in-process and render their `Data`. The DB is the
  single source of truth: reload on screen entry; never cache across actions.
- In-app errors are the status line (`✗ …`), never a crash and never an error
  envelope — the envelope contract stays with `app::run`, which prints exactly
  one JSON envelope on **stdout** after the session ends.
- The TUI renders to **stderr**; raw mode + the alternate screen are restored
  by a Drop guard on every path. Terminal IO failures map to
  `CarpenterError::StoreError` (the environment-failure class).

## Screens (cycle order)

| id | purpose | commands driven |
|----|---------|-----------------|
| `home` | course list, active-course marker, switch | `course list`, `course switch` |
| `intake` | 3-step wizard: outcome → background → constraints → draft spec | (forms; feeds review) |
| `review` | spec preview → scaffold | `course create` |
| `board` | plan status, goals + lesson links, outline, progress | `plan show/list`, `lesson list`, `progress summary`, `plan confirm` |
| `lesson` | lesson detail, execute/skip, feedback | `lesson show/execute/skip`, `notes add/list` |
| `pivot` | goal edits → revised plan | `goal add/update/remove`, `plan update` + `confirm` |

Status taxonomy (badges/glyphs): `planned created verified skipped` for
lessons, `draft confirmed` for plans; note kinds: `gap mistake strength
pattern progress`.

## Key map

| key | action |
|-----|--------|
| `←`/`→` | cycle screens (wraps) |
| `↑`/`↓` (`k`/`j`) | move selection |
| `Enter` | apply on selection |
| `n` | new course (home → intake) |
| `p` | pivot plan (board → pivot) |
| `q` / Ctrl-C | quit |

Screens see keys **first** and may consume them (form screens will own `q` for
text entry); unconsumed keys fall through to the global map.

## Screen contract (the batch interface)

```rust
pub trait Screen {
    fn id(&self) -> ScreenId;
    fn reload(&mut self, paths: &Paths) -> Result<(), CarpenterError>;
    fn draw(&mut self, frame: &mut Frame, area: Rect);
    fn handle_key(&mut self, ctx: &mut Ctx<'_>, key: KeyEvent) -> bool;
}
```

- One screen per file under `screens/`; a screen never touches another screen's
  state. Navigation goes through `Ctx::navigate` only.
- `Ctx` is the action surface: `run(f)` (command fn → `✓`/`✗` status line +
  action counter), `info`, `error`, `navigate`.
- Shared visual kit in `widgets.rs`/`theme.rs` (`titled` panel, `badge`,
  `progress`, keybar; status/kind colors). Call the kit; never inline styles at
  call sites.
- Tests: `TestBackend` buffer assertions per screen (`buffer[(x, y)].symbol()`),
  store-backed fixtures via `commands::testutil`. Run with
  `cargo test --workspace --features tui` (CI lane).

## Rollout

Landed dark: `dev = ["tui"]` (adr/016's relaxed-gate build carries it; release
never compiles ratatui/crossterm until the gate flips). Screen batch lands one
unit per screen against this contract; lesson authoring (full spec parity —
sections, snippets, practice/quiz cases, solutions + `lesson verify`) follows
as its own sequence. The ship PR removes the gate and moves the entry into
`commands/` under the adr/007 gates (docs + example atom + paired test).

## Superseded

An HTML wireframe of this UI (single-page, `docs/design/tui/`) was built and
reviewed, then superseded by this implementation; its contract (screens,
states, key map, component inventory) is distilled here.
