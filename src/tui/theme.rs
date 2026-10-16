//! Theme — the light-minimal wireframe palette mapped to terminal colors.
//!
//! Colors stay close to terminal defaults: a plain text TUI reads best and
//! every theme survives. Two accents (brand + select) and one color per
//! status/note-kind, shared by badges, glyphs, and the progress fill.

use ratatui::style::{Color, Modifier, Style};

/// Brand accent: header brand, tabs, active-course marker.
pub const ACCENT: Color = Color::Blue;

/// Selection accent: the highlighted row's border/symbol color.
pub const SELECT: Color = Color::Cyan;

/// Muted text: hints, keybar, counts.
pub const MUTED: Color = Color::DarkGray;

/// Success / done.
pub const OK: Color = Color::Green;

/// In-progress / attention.
#[allow(dead_code)] // screen-batch kit (design/20)
pub const WARN: Color = Color::Yellow;

/// Error / destructive.
pub const ERR: Color = Color::Red;

/// Style for the selected list row.
pub fn selected() -> Style {
    Style::new()
        .fg(Color::Black)
        .bg(SELECT)
        .add_modifier(Modifier::BOLD)
}

/// Brand text (the header's `carpenter`).
pub fn brand() -> Style {
    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Keycap hint text (the keybar's key names).
pub fn hint() -> Style {
    Style::new().fg(ACCENT)
}

/// Active-course marker (`★`).
pub fn marker() -> Style {
    Style::new().fg(OK)
}

/// Error text (corrupt rows, `✗` status line).
pub fn err_text() -> Style {
    Style::new().fg(ERR)
}

/// Muted text (counts, empty states).
pub fn muted_text() -> Style {
    Style::new().fg(MUTED)
}

/// Style for a screen tab: current vs idle.
pub fn tab(current: bool) -> Style {
    if current {
        Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(MUTED)
    }
}

/// Style for a `● label` badge span in the given color.
pub fn badge(color: Color) -> Style {
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}

/// Color for a lesson/plan status (`planned created verified skipped draft confirmed`,
/// plus the derived progress statuses `complete in_progress achieved`).
#[allow(dead_code)] // screen-batch kit (design/20)
pub fn status(status: &str) -> Color {
    match status {
        "verified" | "confirmed" | "complete" | "achieved" => OK,
        "created" | "in_progress" => WARN,
        _ => MUTED,
    }
}

/// Color for a feedback note kind (`gap mistake strength pattern progress`).
#[allow(dead_code)] // screen-batch kit (design/20)
pub fn note_kind(kind: &str) -> Color {
    match kind {
        "strength" | "progress" => OK,
        "mistake" | "gap" => ERR,
        "pattern" => WARN,
        _ => MUTED,
    }
}
