//! Screen registry: the six screens, their `←`/`→` cycle order, and naming.

/// Every screen in the course-builder TUI, in cycle order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    /// Course list + switch (the entry screen).
    Home,
    /// Intake wizard → draft course spec.
    Intake,
    /// Spec review → `course create`.
    Review,
    /// Plan, lesson outline, progress roll-up.
    Board,
    /// Lesson detail, execute/skip, feedback notes.
    Lesson,
    /// Plan revision (goal edits → `plan update` + confirm).
    Pivot,
}

/// `←`/`→` cycle order.
pub const ALL: [ScreenId; 6] = [
    ScreenId::Home,
    ScreenId::Intake,
    ScreenId::Review,
    ScreenId::Board,
    ScreenId::Lesson,
    ScreenId::Pivot,
];

impl ScreenId {
    /// Lowercase name (header tabs, exit payload).
    pub fn name(self) -> &'static str {
        match self {
            ScreenId::Home => "home",
            ScreenId::Intake => "intake",
            ScreenId::Review => "review",
            ScreenId::Board => "board",
            ScreenId::Lesson => "lesson",
            ScreenId::Pivot => "pivot",
        }
    }

    /// The screen one step to the right (wraps).
    pub fn next(self) -> Self {
        cycle(self, 1)
    }

    /// The screen one step to the left (wraps).
    pub fn prev(self) -> Self {
        cycle(self, -1)
    }
}

fn cycle(current: ScreenId, delta: isize) -> ScreenId {
    let i = ALL.iter().position(|s| *s == current).unwrap_or(0);
    let n = ALL.len() as isize;
    ALL[((i as isize + delta + n) as usize) % ALL.len()]
}

#[cfg(all(test, feature = "tui"))]
mod tests {
    use super::{ScreenId, ALL};

    #[test]
    fn cycles_forward_through_all_screens() {
        let mut id = ScreenId::Home;
        let mut seen = vec![id];
        for _ in 0..ALL.len() - 1 {
            id = id.next();
            seen.push(id);
        }
        assert_eq!(seen, ALL.to_vec());
        assert_eq!(id.next(), ScreenId::Home, "forward wrap");
    }

    #[test]
    fn cycles_backward_through_all_screens() {
        let mut id = ScreenId::Home;
        let mut seen = vec![id];
        for _ in 0..ALL.len() - 1 {
            id = id.prev();
            seen.push(id);
        }
        assert_eq!(
            seen,
            vec![
                ScreenId::Home,
                ScreenId::Pivot,
                ScreenId::Lesson,
                ScreenId::Board,
                ScreenId::Review,
                ScreenId::Intake,
            ]
        );
        assert_eq!(id.prev(), ScreenId::Home, "backward wrap");
    }
}
