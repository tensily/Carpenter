//! Interactive course-builder terminal UI (experimental, adr/028).
//!
//! Compiled only under the `tui` feature (enabled by `dev`), so it never
//! reaches a release binary, the `--help` scrape, the generated howto, or the
//! inlined `SKILL.md` — the same dark-launch path as the `dev` group
//! (adr/016). Screens drive the deterministic command fns in `commands/`
//! in-process; the DB stays the single source of truth (reload on entry and
//! after navigation).
//!
//! Terminal contract: the TUI renders to **stderr**; stdout stays reserved for
//! the one JSON envelope `app::run` prints after the session ends. Raw mode +
//! the alternate screen are restored by a Drop guard on every path.

mod app;
mod router;
mod screens;
mod theme;
mod widgets;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::core::error::CarpenterError;
use crate::core::store::Paths;
use crate::models::Data;

use app::Session;

/// How long the event loop waits for input before the next draw.
const POLL: Duration = Duration::from_millis(150);

/// Run one TUI session: set up the terminal, loop until quit, restore, and
/// return the exit payload. Terminal IO failures map to
/// [`CarpenterError::StoreError`] (the environment-failure class); screen-level
/// command errors surface in-app as the status line — never a crash.
pub fn run(paths: &Paths) -> Result<Data, CarpenterError> {
    let _guard = Guard::new().map_err(io_err)?;
    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).map_err(io_err)?;
    let mut session = Session::new();
    session.enter(paths);
    while !session.done {
        terminal.draw(|frame| session.draw(frame)).map_err(io_err)?;
        if !event::poll(POLL).map_err(io_err)? {
            continue;
        }
        if let Event::Key(key) = event::read().map_err(io_err)? {
            if key.kind == KeyEventKind::Press {
                session.key(key, paths);
            }
        }
    }
    Ok(session.exit())
}

fn io_err(e: io::Error) -> CarpenterError {
    CarpenterError::StoreError(format!("terminal: {e}"))
}

/// Restores raw mode + the main screen even when a session errors out.
struct Guard;

impl Guard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        crossterm::execute!(io::stderr(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stderr(), LeaveAlternateScreen);
    }
}
