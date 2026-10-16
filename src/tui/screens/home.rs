//! Home screen — course list, active-course marker, `Enter` to switch.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};

use crate::core::error::CarpenterError;
use crate::core::store::Paths;
use crate::models::course::CourseListItem;
use crate::models::{Data, RowError};

use crate::tui::app::Ctx;
use crate::tui::router::ScreenId;
use crate::tui::screens::Screen;
use crate::tui::{theme, widgets};

/// The course list.
#[derive(Default)]
pub struct Home {
    rows: Vec<CourseListItem>,
    errors: Vec<RowError>,
    /// Slug of the active course (from config), rendered with a marker.
    active: Option<String>,
    selected: usize,
}

impl Screen for Home {
    fn id(&self) -> ScreenId {
        ScreenId::Home
    }

    fn reload(&mut self, paths: &Paths) -> Result<(), CarpenterError> {
        let Data::CourseList { courses, errors } = crate::commands::course::list(paths)? else {
            return Err(CarpenterError::StoreError(
                "unexpected course list payload".into(),
            ));
        };
        self.rows = courses;
        self.errors = errors;
        self.active = paths
            .config_file()
            .map(|p| crate::core::config::load_from(&p).active_course)
            .unwrap_or_default();
        if self.selected >= self.rows.len() {
            self.selected = 0;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let mut items: Vec<ListItem> = self
            .rows
            .iter()
            .map(|course| {
                let marker = match &self.active {
                    Some(active) if *active == course.slug => Span::styled("★ ", theme::marker()),
                    _ => Span::raw("  "),
                };
                ListItem::new(Line::from(vec![
                    marker,
                    Span::styled(
                        format!("{:<22}", course.slug),
                        ratatui::style::Style::new().fg(theme::ACCENT),
                    ),
                    Span::raw(&course.title),
                    Span::raw("  "),
                    Span::styled(
                        format!("{} lessons", course.lessons_count),
                        theme::muted_text(),
                    ),
                ]))
            })
            .collect();
        // Corrupt rows surface here — never silently dropped (error contract).
        for e in &self.errors {
            let id = e.id.as_deref().unwrap_or("?");
            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!("✗ corrupt course row ({}): {}", id, e.reason),
                theme::err_text(),
            )])));
        }
        if items.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "no courses yet — press n to scaffold one",
                theme::muted_text(),
            ))));
        }
        let list = List::new(items)
            .block(widgets::titled("Courses"))
            .highlight_style(theme::selected());
        let mut state = ListState::default().with_selected(Some(self.selected));
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn handle_key(&mut self, ctx: &mut Ctx<'_>, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.rows.is_empty() {
                    self.selected = (self.selected + 1) % self.rows.len();
                }
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.rows.is_empty() {
                    self.selected = (self.selected + self.rows.len() - 1) % self.rows.len();
                }
                true
            }
            KeyCode::Enter => {
                let Some(course) = self.rows.get(self.selected) else {
                    return true;
                };
                let slug = course.slug.clone();
                let switched = ctx.run(move |paths| crate::commands::course::switch(paths, &slug));
                // Refresh the marker now — navigation won't (same screen).
                if switched.is_some() {
                    self.active = Some(course.slug.clone());
                }
                true
            }
            KeyCode::Char('n') => {
                ctx.navigate(ScreenId::Intake);
                true
            }
            _ => false,
        }
    }
}

#[cfg(all(test, feature = "tui"))]
mod tests {
    use super::Home;
    use crate::commands;
    use crate::tui::app::Ctx;
    use crate::tui::router::ScreenId;
    use crate::tui::screens::Screen;
    use crate::tui::testsupport::{buffer_text, key};
    use crossterm::event::KeyCode;
    use ratatui::{backend::TestBackend, Terminal};

    fn two_course_paths() -> (crate::core::store::Paths, String, String) {
        let (paths, first) = commands::testutil::setup();
        let spec = "title: Linear Algebra\ngoal: la intuition\n";
        let second = match commands::course::create(&paths, spec).expect("second course") {
            crate::models::Data::CourseCreate { slug, .. } => slug,
            other => panic!("unexpected payload: {other:?}"),
        };
        (paths, first, second)
    }

    #[test]
    fn reload_lists_courses_with_active_marker() {
        let (paths, first, second) = two_course_paths();
        commands::course::switch(&paths, &second).expect("switch");
        let mut home = Home::default();
        home.reload(&paths).expect("reload");
        assert_eq!(home.rows.len(), 2);
        assert_eq!(home.active.as_deref(), Some(second.as_str()));
        assert_ne!(home.active.as_deref(), Some(first.as_str()));
    }

    #[test]
    fn enter_switches_active_course_and_refreshes_marker() {
        let (paths, _first, second) = two_course_paths();
        let mut home = Home::default();
        home.reload(&paths).expect("reload");
        let mut actions = 0;
        home.handle_key(&mut Ctx::new(&paths, &mut actions), key(KeyCode::Down));
        let mut ctx = Ctx::new(&paths, &mut actions);
        assert!(home.handle_key(&mut ctx, key(KeyCode::Enter)));
        let active = paths
            .config_file()
            .map(|p| crate::core::config::load_from(&p).active_course)
            .unwrap_or_default();
        assert_eq!(active.as_deref(), Some(second.as_str()));
        assert_eq!(
            home.active.as_deref(),
            Some(second.as_str()),
            "marker must refresh on switch, not only on reload"
        );
    }

    #[test]
    fn n_navigates_to_intake() {
        let (paths, _first, _second) = two_course_paths();
        let mut home = Home::default();
        home.reload(&paths).expect("reload");
        let mut actions = 0;
        let mut ctx = Ctx::new(&paths, &mut actions);
        assert!(home.handle_key(&mut ctx, key(KeyCode::Char('n'))));
        assert_eq!(ctx.peek_goto(), Some(ScreenId::Intake));
    }

    #[test]
    fn draw_renders_course_rows() {
        let (paths, _first, _second) = two_course_paths();
        let mut home = Home::default();
        home.reload(&paths).expect("reload");
        let mut terminal = Terminal::new(TestBackend::new(80, 10)).expect("terminal");
        terminal
            .draw(|frame| home.draw(frame, frame.area()))
            .expect("draw");
        let text = buffer_text(terminal.backend().buffer(), 80, 10);
        assert!(text.contains("Courses"), "panel title missing: {text:?}");
        assert!(text.contains("Data Structures"), "first course missing");
        assert!(text.contains("Linear Algebra"), "second course missing");
    }

    #[test]
    fn selection_wraps_at_both_ends() {
        let (paths, _first, _second) = two_course_paths();
        let mut home = Home::default();
        home.reload(&paths).expect("reload");
        let mut actions = 0;
        home.handle_key(&mut Ctx::new(&paths, &mut actions), key(KeyCode::Up));
        assert_eq!(home.selected, 1, "up from 0 wraps to last");
        home.handle_key(&mut Ctx::new(&paths, &mut actions), key(KeyCode::Down));
        assert_eq!(home.selected, 0, "down from last wraps to 0");
    }
}
