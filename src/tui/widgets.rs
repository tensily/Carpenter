//! Small render helpers shared by screens — the TUI analog of the wireframe's
//! `c-*` component library (one place per visual pattern, never inline styles
//! at call sites).
//!
//! Kit entries not yet consumed by a shipped screen carry a per-item
//! `allow(dead_code) // screen-batch kit (design/20)`, like theme.rs — the
//! blanket form would permanently exempt this module (the batch's landing
//! pad) from the dead-code gate.

use ratatui::text::{Line, Span};
use ratatui::widgets::Block;

use crate::tui::theme;

/// A titled bordered block — the core TUI panel unit.
pub fn titled(title: &str) -> Block<'_> {
    Block::bordered().title(format!(" {title} "))
}

/// A `[████░░] 22%` progress line `width` cells wide.
#[allow(dead_code)] // screen-batch kit (design/20)
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

/// A `● label` status/kind badge span in the given theme color.
#[allow(dead_code)] // screen-batch kit (design/20)
pub fn badge(color: ratatui::style::Color, label: &str) -> Span<'_> {
    Span::styled(format!("● {label}"), theme::badge(color))
}

/// The global key-hint footer, from the design/20 key-map table.
pub fn keybar() -> Line<'static> {
    const HINTS: [(&str, &str); 6] = [
        ("←/→", "screens"),
        ("↑/↓", "move"),
        ("Enter", "apply"),
        ("n", "new course"),
        ("p", "pivot"),
        ("q", "quit"),
    ];
    let spans: Vec<Span> = HINTS
        .iter()
        .flat_map(|(key, label)| {
            [
                Span::styled((*key).to_string(), theme::hint()),
                Span::raw(format!(" {label}  ")),
            ]
        })
        .collect();
    Line::from(spans)
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
