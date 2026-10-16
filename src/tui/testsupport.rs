//! Shared test helpers for `tui` tests (test-only, mirrors
//! `commands::testutil`). One copy per helper — never per test module.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;

/// A bare key event (no modifiers) for `handle_key` tests.
pub(crate) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Flatten a rendered buffer region to text (one line per row) for
/// `TestBackend` assertions.
pub(crate) fn buffer_text(buffer: &Buffer, width: u16, height: u16) -> String {
    let mut text = String::new();
    for y in 0..height {
        for x in 0..width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}
