//! App state and input routing.
//!
//! [`Session`] owns the screens and the cross-screen chrome (header, status
//! line, keybar). [`Ctx`] is what a screen gets to act on: run a command
//! (errors become the status line — never a crash), navigate, or quit. Screens
//! see keys first and mark unconsumed ones for the global map (`←`/`→` cycle,
//! `q`/Ctrl-C quit), so form screens can own `q` for their own ends later.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::core::error::CarpenterError;
use crate::core::store::Paths;
use crate::models::Data;

use crate::tui::router::ScreenId;
use crate::tui::screens::{self, Screen};
use crate::tui::{theme, widgets};
/// One session of the TUI: current screen, status line, exit payload inputs.
pub struct Session {
    screens: Vec<Box<dyn Screen>>,
    current: usize,
    /// Set when the loop should end.
    pub done: bool,
    notice: Option<Notice>,
    actions: u32,
    visited: Vec<ScreenId>,
}

/// A status-line message (`✓`/`✗` prefixed by severity).
struct Notice {
    text: String,
    is_error: bool,
}

impl Notice {
    fn new(text: impl Into<String>, is_error: bool) -> Self {
        Self {
            text: text.into(),
            is_error,
        }
    }
}

/// What a screen may do to the session while handling a key.
pub struct Ctx<'a> {
    paths: &'a Paths,
    notice: Option<Notice>,
    goto: Option<ScreenId>,
    actions: &'a mut u32,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(paths: &'a Paths, actions: &'a mut u32) -> Self {
        Self {
            paths,
            notice: None,
            goto: None,
            actions,
        }
    }

    /// Run one command fn: success → `✓ message` status line + the payload
    /// (so the caller can refresh derived state), failure → `✗ error`. The
    /// envelope contract stays with `app::run`; in-app errors are status
    /// lines, matching "never raise to the caller".
    pub fn run(
        &mut self,
        action: impl FnOnce(&Paths) -> Result<Data, CarpenterError>,
    ) -> Option<Data> {
        match action(self.paths) {
            Ok(data) => {
                *self.actions += 1;
                self.info(data.message());
                Some(data)
            }
            Err(e) => {
                self.error(e.to_string());
                None
            }
        }
    }

    /// Set a neutral status-line message.
    pub fn info(&mut self, text: impl Into<String>) {
        self.notice = Some(Notice::new(text, false));
    }

    /// Set an error status-line message.
    pub fn error(&mut self, text: impl Into<String>) {
        self.notice = Some(Notice::new(text, true));
    }

    /// Navigate to another screen on this key.
    pub fn navigate(&mut self, id: ScreenId) {
        self.goto = Some(id);
    }

    /// (test only) the pending navigation target, if any.
    #[cfg(test)]
    pub fn peek_goto(&self) -> Option<ScreenId> {
        self.goto
    }
}

impl Session {
    /// A session starting on [`ScreenId::Home`].
    pub fn new() -> Self {
        Self {
            screens: screens::all(),
            current: 0,
            done: false,
            notice: None,
            actions: 0,
            visited: vec![ScreenId::Home],
        }
    }

    /// First load: refresh the entry screen.
    pub fn enter(&mut self, paths: &Paths) {
        self.reload_current(paths);
    }

    /// The id of the on-screen screen.
    pub fn current_id(&self) -> ScreenId {
        self.screens[self.current].id()
    }

    /// Feed one key press: screen first, then the global map.
    pub fn key(&mut self, key: KeyEvent, paths: &Paths) {
        let mut ctx = Ctx::new(paths, &mut self.actions);
        let consumed = self.screens[self.current].handle_key(&mut ctx, key);
        self.notice = ctx.notice;
        if let Some(id) = ctx.goto {
            self.goto(id, paths);
        }
        if consumed {
            return;
        }
        match key.code {
            KeyCode::Left => self.goto(self.current_id().prev(), paths),
            KeyCode::Right => self.goto(self.current_id().next(), paths),
            KeyCode::Char('q') => self.done = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.done = true;
            }
            _ => {}
        }
    }

    fn goto(&mut self, id: ScreenId, paths: &Paths) {
        if id == self.current_id() {
            return;
        }
        let Some(i) = self.screens.iter().position(|s| s.id() == id) else {
            return;
        };
        if self.visited.last() != Some(&id) {
            self.visited.push(id);
        }
        self.current = i;
        self.reload_current(paths);
    }

    fn reload_current(&mut self, paths: &Paths) {
        if let Err(e) = self.screens[self.current].reload(paths) {
            self.notice = Some(Notice::new(e.to_string(), true));
        }
    }

    /// The exit payload: screens visited (in order) + applied action count.
    pub fn exit(&self) -> Data {
        Data::TuiExit {
            screens: self
                .visited
                .iter()
                .map(|id| id.name().to_string())
                .collect(),
            actions: self.actions,
        }
    }

    /// Draw the session: header tabs, current screen, status line, keybar.
    pub fn draw(&mut self, frame: &mut ratatui::Frame) {
        use ratatui::layout::{Constraint, Layout};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

        let mut header = vec![Span::styled("carpenter", theme::brand())];
        for id in crate::tui::router::ALL {
            header.push(Span::styled(
                format!(" {} ", id.name()),
                theme::tab(id == self.current_id()),
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(header)), chunks[0]);

        self.screens[self.current].draw(frame, chunks[1]);

        let status = match &self.notice {
            Some(n) if n.is_error => {
                Line::from(Span::styled(format!("✗ {}", n.text), theme::err_text()))
            }
            Some(n) => Line::from(Span::styled(format!("✓ {}", n.text), theme::marker())),
            None => Line::from(Span::raw("")),
        };
        frame.render_widget(Paragraph::new(status), chunks[2]);
        frame.render_widget(Paragraph::new(widgets::keybar()), chunks[3]);
    }
}

#[cfg(all(test, feature = "tui"))]
mod tests {
    use super::Session;
    use crate::commands;
    use crate::models::Data;
    use crate::tui::router::ScreenId;
    use crate::tui::testsupport::key;
    use crossterm::event::KeyCode;

    #[test]
    fn right_cycles_screens_and_records_visits() {
        let (paths, _slug) = commands::testutil::setup();
        let mut session = Session::new();
        assert_eq!(session.current_id(), ScreenId::Home);
        session.key(key(KeyCode::Right), &paths);
        assert_eq!(session.current_id(), ScreenId::Intake);
        session.key(key(KeyCode::Right), &paths);
        assert_eq!(session.current_id(), ScreenId::Review);
        session.key(key(KeyCode::Left), &paths);
        assert_eq!(session.current_id(), ScreenId::Intake);
        assert_eq!(
            session.visited,
            vec![
                ScreenId::Home,
                ScreenId::Intake,
                ScreenId::Review,
                ScreenId::Intake,
            ]
        );
    }

    #[test]
    fn q_quits_from_home() {
        let (paths, _slug) = commands::testutil::setup();
        let mut session = Session::new();
        session.key(key(KeyCode::Char('q')), &paths);
        assert!(session.done);
        match session.exit() {
            Data::TuiExit { screens, actions } => {
                assert_eq!(screens, vec!["home"]);
                assert_eq!(actions, 0);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }
}
