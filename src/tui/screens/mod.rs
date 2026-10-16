//! Screens — the [`Screen`] trait every screen implements, the registry, and
//! the placeholder for screens not yet built.
//!
//! The trait is the batch contract (design/20): the skeleton owns routing,
//! chrome, and the key-global map; each screen owns its own state, drawing,
//! keys, and command calls. One screen per file under `screens/`.

pub mod home;

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::core::error::CarpenterError;
use crate::core::store::Paths;

use crate::tui::app::Ctx;
use crate::tui::router::{self, ScreenId};
use crate::tui::theme;
use crate::tui::widgets;

/// One screen of the TUI.
pub trait Screen {
    /// Registry id (routing + header tab).
    fn id(&self) -> ScreenId;

    /// Refresh from the store. Called on entry to the screen and after
    /// navigation; the DB is the single source of truth, screens never cache
    /// across actions.
    ///
    /// Errors here become the status line (the screen still draws).
    fn reload(&mut self, paths: &Paths) -> Result<(), CarpenterError>;

    /// Draw into the screen area.
    fn draw(&mut self, frame: &mut Frame, area: Rect);

    /// Handle a key. Return `true` when consumed; unconsumed keys fall through
    /// to the global map (`←`/`→` cycle, `q`/Ctrl-C quit).
    fn handle_key(&mut self, ctx: &mut Ctx<'_>, key: KeyEvent) -> bool;
}

/// The placeholder note shared by unbuilt screens.
const PLANNED: &str = "planned — screen batch (design/20)";

/// A screen planned but not yet built — renders its title + note. Each batch
/// unit replaces one arm of [`screen_for`].
pub struct Placeholder {
    id: ScreenId,
    title: &'static str,
}

impl Placeholder {
    /// A placeholder for `id`, shown as `title` with the shared planned note.
    pub fn new(id: ScreenId, title: &'static str) -> Self {
        Self { id, title }
    }
}

impl Screen for Placeholder {
    fn id(&self) -> ScreenId {
        self.id
    }

    fn reload(&mut self, _paths: &Paths) -> Result<(), CarpenterError> {
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let line = Line::from(Span::styled(PLANNED, theme::muted_text()));
        frame.render_widget(
            Paragraph::new(line).block(widgets::titled(self.title)),
            area,
        );
    }

    fn handle_key(&mut self, _ctx: &mut Ctx<'_>, _key: KeyEvent) -> bool {
        false
    }
}

/// Build the screen for `id`. Total over [`router::ALL`] — a new `ScreenId`
/// without an arm here is a compile error, not a silently unroutable tab.
fn screen_for(id: ScreenId) -> Box<dyn Screen> {
    match id {
        ScreenId::Home => Box::new(home::Home::default()),
        ScreenId::Intake => Box::new(Placeholder::new(id, "Intake")),
        ScreenId::Review => Box::new(Placeholder::new(id, "Review")),
        ScreenId::Board => Box::new(Placeholder::new(id, "Board")),
        ScreenId::Lesson => Box::new(Placeholder::new(id, "Lesson")),
        ScreenId::Pivot => Box::new(Placeholder::new(id, "Pivot")),
    }
}

/// The screen registry, in cycle order — derived from [`router::ALL`] so the
/// cycle order and the registry cannot drift.
pub fn all() -> Vec<Box<dyn Screen>> {
    router::ALL.map(screen_for).into_iter().collect()
}

#[cfg(all(test, feature = "tui"))]
mod tests {
    use super::{Placeholder, Screen};
    use crate::core::store::Paths;
    use crate::tui::app::Ctx;
    use crate::tui::testsupport::{buffer_text, key};
    use crossterm::event::KeyCode;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn placeholder_draws_title_and_planned_note() {
        let mut screen = Placeholder::new(crate::tui::router::ScreenId::Intake, "Intake");
        let mut terminal = Terminal::new(TestBackend::new(40, 5)).expect("terminal");
        terminal
            .draw(|frame| screen.draw(frame, frame.area()))
            .expect("draw");
        let text = buffer_text(terminal.backend().buffer(), 40, 5);
        assert!(text.contains("Intake"), "panel title missing: {text:?}");
        assert!(text.contains("planned — screen batch"), "note missing");
    }

    #[test]
    fn placeholder_consumes_no_keys() {
        let (paths, _slug) = crate::commands::testutil::setup();
        let mut actions = 0;
        let mut ctx = Ctx::new(&paths, &mut actions);
        assert!(
            !Placeholder::new(crate::tui::router::ScreenId::Intake, "Intake")
                .handle_key(&mut ctx, key(KeyCode::Char('x')))
        );
    }

    #[test]
    fn placeholder_reload_is_infallible() {
        let paths = Paths {
            root: std::env::temp_dir().join("carpenter-tui-ph-reload"),
            config_dir: None,
        };
        assert!(
            Placeholder::new(crate::tui::router::ScreenId::Intake, "Intake")
                .reload(&paths)
                .is_ok()
        );
    }
}
