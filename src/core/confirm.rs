//! Interactive confirmation for destructive self-operations (`self remove`,
//! `self uninstall`, `self deregister`). The removal plan goes to stderr —
//! never stdout, which carries the one JSON envelope — and the answer is read
//! from stdin: the terminal when run interactively, EOF (decline) under
//! closed/piped streams such as `Command::output()`. `--yes` at the command
//! layer is the non-interactive bypass
//! ([adr/019](../../docs/adr/019-uninstall-semantics.md) revision — the
//! destructive inversion of [adr/024](../../docs/adr/024-install-consent-and-banner.md)'s
//! installer consent, which proceeds when unattended; a remover must not).

use std::io::{BufRead, Write};

use crate::core::error::CarpenterError;

/// The consent prompt printed after the plan. Default is decline: Enter or any
/// answer that is not an explicit `y`/`Y` aborts.
const PROMPT: &str = "proceed with the removal plan? [y/N] ";

/// Print `plan` to stderr, then ask for confirmation on stdin. `Ok(())` only
/// for an explicit `y`/`Y`; Enter, any other answer, or EOF declines — mapped
/// to `Conflict` naming `--yes`, so a non-interactive caller always gets a
/// typed error envelope instead of a silent destructive run or a hang.
pub fn confirm(plan: &str) -> Result<(), CarpenterError> {
    let mut err = std::io::stderr().lock();
    let _ = write!(err, "{plan}{PROMPT}");
    let _ = err.flush();
    if !confirm_on(&mut std::io::stdin().lock()) {
        return Err(declined());
    }
    Ok(())
}

fn declined() -> CarpenterError {
    CarpenterError::Conflict(
        "not confirmed (declined or no confirmation stream); pass --yes to proceed non-interactively"
            .into(),
    )
}

/// Injectable decision core of [`confirm`]: read one line from `r`; an
/// explicit `y`/`Y` confirms, anything else — Enter, `n`, an unreadable
/// stream, EOF — declines. Infallible by construction (an unreadable stream
/// reads as EOF). Public for tests.
pub fn confirm_on(r: &mut impl BufRead) -> bool {
    let mut line = String::new();
    match r.read_line(&mut line) {
        Ok(n) if n > 0 => matches!(line.trim(), "y" | "Y"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn explicit_yes_confirms_everything_else_declines() {
        assert!(confirm_on(&mut Cursor::new("y\n")));
        assert!(confirm_on(&mut Cursor::new("Y\n")));
        assert!(!confirm_on(&mut Cursor::new("n\n")), "explicit no");
        assert!(!confirm_on(&mut Cursor::new("\n")), "Enter = N");
        assert!(!confirm_on(&mut Cursor::new("yes\n")), "must be bare y/Y");
        assert!(!confirm_on(&mut Cursor::new("")), "EOF declines");
    }
}
