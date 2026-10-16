//! Small render helpers shared by screens — the TUI analog of the wireframe's
//! `c-*` component library (one place per visual pattern, never inline styles
//! at call sites).
//!
//! The kit is landed with the skeleton as the screen-batch contract
//! (design/20); entries not yet consumed by a shipped screen carry
//! `allow(dead_code)` until their unit lands.
#![allow(dead_code)]

use ratatui::text::{Line, Span};
use ratatui::widgets::Block;

use crate::tui::theme;

/// A titled bordered block — the core TUI panel unit.
pub fn titled(title: &str) -> Block<'_> {
    Block::bordered().title(format!(" {title} "))
}

/// A `[████░░] 22%` progress line `width` cells wide.
pub fn progress(pct: u32, width: usize) -> String {
    let pct = pct.clamp(0, 100);
    let filled = (width * pct as usize) / 100;
    let mut bar = String::with_capacity(width + 6);
    bar.push('[');
    for i in 0..width {
        bar.push(if i < filled { '█' } else { '░' });
    }
    bar.push(']');
    bar.push_str(&format!(" {pct}%"));
    bar
}

/// A `● label` status/kind badge span in the status's theme color.
pub fn badge(color: ratatui::style::Color, label: &str) -> Span<'_> {
    Span::styled(format!("● {label}"), theme::badge(color))
}

/// The global key-hint footer.
pub fn keybar() -> Line<'static> {
    Line::from(vec![
        Span::styled("←/→", hint()),
        Span::raw(" screens  "),
        Span::styled("↑/↓", hint()),
        Span::raw(" move  "),
        Span::styled("Enter", hint()),
        Span::raw(" apply  "),
        Span::styled("n", hint()),
        Span::raw(" new course  "),
        Span::styled("p", hint()),
        Span::raw(" pivot  "),
        Span::styled("q", hint()),
        Span::raw(" quit"),
    ])
}

fn hint() -> ratatui::style::Style {
    ratatui::style::Style::new().fg(theme::ACCENT)
}

#[cfg(all(test, feature = "tui"))]
mod tests {
    use super::progress;

    #[test]
    fn progress_renders_bar_and_pct() {
        let bar = progress(50, 10);
        assert_eq!(bar, "[█████░░░░░] 50%");
        assert_eq!(progress(0, 4), "[░░░░] 0%");
        assert_eq!(progress(100, 4), "[████] 100%");
        assert_eq!(progress(150, 4), "[████] 100%", "clamped");
    }
}
