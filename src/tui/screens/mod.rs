//! Screens — the [`Screen`] trait every screen implements, the registry, and
//! the placeholder for screens not yet built.
//!
//! The trait is the batch contract (design/20): the skeleton owns routing,
//! chrome, and the key-global map; each screen owns its own state, drawing,
//! keys, and command calls. One screen per file under `screens/`.

pub mod home;

use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::core::error::CarpenterError;
use crate::core::store::Paths;

use crate::tui::app::Ctx;
use crate::tui::router::ScreenId;

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
    fn draw(&mut self, frame: &mut Frame, area: ratatui::layout::Rect);

    /// Handle a key. Return `true` when consumed; unconsumed keys fall through
    /// to the global map (`←`/`→` cycle, `q`/Ctrl-C quit).
    fn handle_key(&mut self, ctx: &mut Ctx<'_>, key: KeyEvent) -> bool;
}

/// A screen planned but not yet built — renders its title + note. Each batch
/// unit replaces one instance in [`all`].
pub struct Placeholder {
    id: ScreenId,
    title: &'static str,
    note: &'static str,
}

impl Placeholder {
    /// A placeholder for `id`, shown as `title` with the `note` line.
    pub fn new(id: ScreenId, title: &'static str, note: &'static str) -> Self {
        Self { id, title, note }
    }
}

impl Screen for Placeholder {
    fn id(&self) -> ScreenId {
        self.id
    }

    fn reload(&mut self, _paths: &Paths) -> Result<(), CarpenterError> {
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;
        let line = Line::from(vec![
            Span::raw(self.title),
            Span::styled(" — ", ratatui::style::Style::new()),
            Span::styled(self.note, ratatui::style::Style::new().fg(theme_muted())),
        ]);
        frame.render_widget(
            Paragraph::new(line).block(crate::tui::widgets::titled(self.title)),
            area,
        );
    }

    fn handle_key(&mut self, _ctx: &mut Ctx<'_>, _key: KeyEvent) -> bool {
        false
    }
}

fn theme_muted() -> ratatui::style::Color {
    crate::tui::theme::MUTED
}

/// The screen registry, in cycle order. Placeholder entries are replaced by
/// their real screens as the batch lands.
pub fn all() -> Vec<Box<dyn Screen>> {
    vec![
        Box::new(home::Home::default()),
        Box::new(Placeholder::new(
            ScreenId::Intake,
            "Intake",
            "planned — screen batch (design/20)",
        )),
        Box::new(Placeholder::new(
            ScreenId::Review,
            "Review",
            "planned — screen batch (design/20)",
        )),
        Box::new(Placeholder::new(
            ScreenId::Board,
            "Board",
            "planned — screen batch (design/20)",
        )),
        Box::new(Placeholder::new(
            ScreenId::Lesson,
            "Lesson",
            "planned — screen batch (design/20)",
        )),
        Box::new(Placeholder::new(
            ScreenId::Pivot,
            "Pivot",
            "planned — screen batch (design/20)",
        )),
    ]
}

#[cfg(all(test, feature = "tui"))]
mod tests {
    use super::{Placeholder, Screen};
    use crate::core::store::Paths;
    use crate::tui::app::Ctx;
    use crate::tui::router::ScreenId;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};

    fn placeholder() -> Placeholder {
        Placeholder::new(ScreenId::Intake, "Intake", "planned — screen batch")
    }

    #[test]
    fn placeholder_draws_title_and_note() {
        let mut screen = placeholder();
        let mut terminal = Terminal::new(TestBackend::new(40, 5)).expect("terminal");
        terminal
            .draw(|frame| screen.draw(frame, frame.area()))
            .expect("draw");
        let text = buffer_text(terminal.backend().buffer(), 40, 5);
        assert!(text.contains("Intake"), "title missing: {text:?}");
        assert!(text.contains("planned — screen batch"), "note missing");
    }

    #[test]
    fn placeholder_consumes_no_keys() {
        let (paths, _slug) = crate::commands::testutil::setup();
        let mut actions = 0;
        let mut ctx = Ctx::new(&paths, &mut actions);
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert!(!placeholder().handle_key(&mut ctx, key));
    }

    #[test]
    fn placeholder_reload_is_infallible() {
        let paths = Paths {
            root: std::env::temp_dir().join("carpenter-tui-ph-reload"),
            config_dir: None,
        };
        assert!(placeholder().reload(&paths).is_ok());
    }

    fn buffer_text(buffer: &ratatui::buffer::Buffer, width: u16, height: u16) -> String {
        let mut text = String::new();
        for y in 0..height {
            for x in 0..width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }
}
