//! Pure old↔new child matching behind `lesson update --force` id preservation.
//!
//! A new child keeps its old id iff the spec-derived content is structurally
//! identical (position-independent, so reorders keep ids): sections compare
//! `title` + snippets, checkables compare `name`/`signature`/`prompt`/
//! `solution` + cases, cases compare `compare`/`args`/`kwargs`/`expected`.
//! Practice items match only within their matched section (the `section_id`
//! FK must stay coherent) and cases within their matched owner. Duplicate
//! identical old children match lowest `ord` first (snapshot order); the
//! leftover is treated as removed. Fresh ids always come from `next_id`, so
//! the never-reuse invariant is untouched.

use crate::models::lesson::{CaseSpec, CheckableSpec, LessonSpec, SectionSpec};

/// The learner-state columns of a practice/quiz row. `lesson update --force`
/// carries them over for content-unchanged children; fresh rows use
/// [`CheckableState::fresh`].
#[derive(Debug, Clone)]
pub struct CheckableState {
    /// skip flag.
    pub skip: bool,
    /// last-check pass flag.
    pub pass_or_fail: bool,
    /// last-check JSON (`{passed,total,cases:[…]}`).
    pub last_check: String,
}

impl CheckableState {
    /// The state of a freshly inserted row (column defaults).
    pub fn fresh() -> Self {
        Self {
            skip: false,
            pass_or_fail: false,
            last_check: String::from("{}"),
        }
    }
}

/// A stored section available for matching.
#[derive(Debug, Clone)]
pub struct OldSection {
    /// Row id (`s1`…).
    pub id: String,
    /// Section title.
    pub title: String,
    /// `(kind, content)` per snippet — the unstable `sn{n}` ids are dropped.
    pub snippets: Vec<(String, String)>,
    /// Practice items (lowest `ord` first).
    pub practice: Vec<OldCheckable>,
}

/// A stored practice/quiz available for matching.
#[derive(Debug, Clone)]
pub struct OldCheckable {
    /// Row id (`p1`/`q1`…).
    pub id: String,
    /// Function name.
    pub name: String,
    /// Signature.
    pub signature: String,
    /// Prompt.
    pub prompt: String,
    /// Author reference solution (empty = none).
    pub solution: String,
    /// Learner-state columns.
    pub state: CheckableState,
    /// Test cases (lowest `ord` first).
    pub cases: Vec<OldCase>,
}

/// A stored test case available for matching.
#[derive(Debug, Clone)]
pub struct OldCase {
    /// Row id (`c1`…).
    pub id: String,
    /// Compare mode string.
    pub compare: String,
    /// args JSON.
    pub args: String,
    /// kwargs JSON.
    pub kwargs: String,
    /// expected JSON.
    pub expected: String,
}

/// Everything stored under a lesson that id preservation can match against
/// (empty for a fresh lesson — every position then allocates a fresh id).
#[derive(Debug, Clone, Default)]
pub struct OldContent {
    /// Sections (lowest `ord` first).
    pub sections: Vec<OldSection>,
    /// Quizzes (lowest `ord` first).
    pub quizzes: Vec<OldCheckable>,
}

/// Keep an old checkable's id and learner-state columns.
#[derive(Debug, Clone)]
pub struct Reuse {
    /// The old row id to re-insert under.
    pub id: String,
    /// Learner-state columns carried over.
    pub state: CheckableState,
}

/// Per-new-section plan.
#[derive(Debug, Clone)]
pub struct SectionPlan {
    /// Old section id to keep; `None` ⇒ allocate a fresh id.
    pub id: Option<String>,
    /// Per practice item in spec order.
    pub practice: Vec<CheckablePlan>,
}

/// Per-new-checkable plan (practice or quiz).
#[derive(Debug, Clone)]
pub struct CheckablePlan {
    /// Reuse when content-unchanged; `None` ⇒ fresh id + default state.
    pub reuse: Option<Reuse>,
    /// Per case old id in spec order; `None` ⇒ allocate a fresh id.
    pub cases: Vec<Option<String>>,
}

/// The full id-preservation plan for one insert pass (default/empty ⇒ all
/// fresh, which is what `lesson create` wants).
#[derive(Debug, Clone, Default)]
pub struct Plan {
    /// Per section in spec order.
    pub sections: Vec<SectionPlan>,
    /// Per quiz in spec order.
    pub quizzes: Vec<CheckablePlan>,
}

/// Match `spec` against `old`, position-independently, and return the
/// per-position reuse plan. Greedy first-match in snapshot (lowest-`ord`)
/// order; a matched old child is consumed, so a duplicate falls through to
/// fresh allocation/removal.
pub fn plan(old: &OldContent, spec: &LessonSpec) -> Plan {
    let mut sec_pool: Vec<&OldSection> = old.sections.iter().collect();
    let sections = spec
        .sections
        .iter()
        .map(
            |sec| match sec_pool.iter().position(|o| section_matches(o, sec)) {
                None => SectionPlan {
                    id: None,
                    practice: sec.practice.iter().map(fresh_checkable).collect(),
                },
                Some(i) => {
                    let o = sec_pool.remove(i);
                    let mut prac_pool: Vec<&OldCheckable> = o.practice.iter().collect();
                    SectionPlan {
                        id: Some(o.id.clone()),
                        practice: match_pool(&mut prac_pool, &sec.practice),
                    }
                }
            },
        )
        .collect();
    let mut quiz_pool: Vec<&OldCheckable> = old.quizzes.iter().collect();
    let quizzes = match_pool(&mut quiz_pool, &spec.quizzes);
    Plan { sections, quizzes }
}

fn fresh_checkable(c: &CheckableSpec) -> CheckablePlan {
    CheckablePlan {
        reuse: None,
        cases: vec![None; c.cases.len()],
    }
}

fn match_pool(pool: &mut Vec<&OldCheckable>, spec: &[CheckableSpec]) -> Vec<CheckablePlan> {
    spec.iter()
        .map(
            |c| match pool.iter().position(|o| checkable_matches(o, c)) {
                None => fresh_checkable(c),
                Some(i) => {
                    let o = pool.remove(i);
                    CheckablePlan {
                        reuse: Some(Reuse {
                            id: o.id.clone(),
                            state: o.state.clone(),
                        }),
                        cases: match_cases(&o.cases, &c.cases),
                    }
                }
            },
        )
        .collect()
}

fn match_cases(old: &[OldCase], spec: &[CaseSpec]) -> Vec<Option<String>> {
    let mut pool: Vec<&OldCase> = old.iter().collect();
    spec.iter()
        .map(|c| {
            pool.iter()
                .position(|o| case_matches(o, c))
                .map(|i| pool.remove(i).id.clone())
        })
        .collect()
}

fn section_matches(o: &OldSection, s: &SectionSpec) -> bool {
    o.title == s.title
        && o.snippets.len() == s.snippets.len()
        && o.snippets
            .iter()
            .zip(&s.snippets)
            .all(|(a, b)| a.0 == b.kind && a.1 == b.content)
}

fn checkable_matches(o: &OldCheckable, c: &CheckableSpec) -> bool {
    o.name == c.name
        && o.signature == c.signature
        && o.prompt == c.prompt
        && o.solution == c.solution.as_deref().unwrap_or("")
        && o.cases.len() == c.cases.len()
        && o.cases
            .iter()
            .zip(&c.cases)
            .all(|(a, b)| case_matches(a, b))
}

fn case_matches(o: &OldCase, c: &CaseSpec) -> bool {
    let (compare, args, kwargs, expected) = case_key(c);
    o.compare == compare && o.args == args && o.kwargs == kwargs && o.expected == expected
}

/// The spec case rendered exactly as `insert_test_case` stores it (canonical
/// `serde_json` strings), so equality with the stored columns is byte-exact.
fn case_key(c: &CaseSpec) -> (String, String, String, String) {
    (
        c.compare.as_str().to_string(),
        serde_json::to_string(&c.args).unwrap_or_else(|_| String::from("[]")),
        serde_json::to_string(&c.kwargs).unwrap_or_else(|_| String::from("{}")),
        serde_json::to_string(&c.expected).unwrap_or_else(|_| String::from("null")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::compare::CompareMode;
    use crate::models::lesson::{SectionSpec, SnippetSpec};
    use serde_json::{json, Map, Value};

    fn case(expected: Value) -> CaseSpec {
        CaseSpec {
            compare: CompareMode::Exact,
            args: vec![Value::from(1)],
            kwargs: Map::new(),
            expected,
        }
    }

    fn checkable(name: &str) -> CheckableSpec {
        CheckableSpec {
            name: name.into(),
            signature: format!("def {name}(x):"),
            prompt: format!("{name} prompt"),
            cases: vec![case(json!(1))],
            solution: None,
        }
    }

    fn old_checkable(id: &str, name: &str) -> OldCheckable {
        let c = checkable(name);
        OldCheckable {
            id: id.into(),
            name: c.name,
            signature: c.signature,
            prompt: c.prompt,
            solution: String::new(),
            state: CheckableState {
                skip: true,
                pass_or_fail: false,
                last_check: "{}".into(),
            },
            cases: vec![OldCase {
                id: format!("{id}-case"),
                compare: "exact".into(),
                args: "[1]".into(),
                kwargs: "{}".into(),
                expected: "1".into(),
            }],
        }
    }

    fn old_section(id: &str, title: &str, practice: Vec<OldCheckable>) -> OldSection {
        OldSection {
            id: id.into(),
            title: title.into(),
            snippets: vec![("markdown".into(), "hi".into())],
            practice,
        }
    }

    fn spec_section(title: &str, practice: Vec<CheckableSpec>) -> SectionSpec {
        SectionSpec {
            title: title.into(),
            snippets: vec![SnippetSpec {
                kind: "markdown".into(),
                content: "hi".into(),
            }],
            practice,
        }
    }

    fn spec(sections: Vec<SectionSpec>, quizzes: Vec<CheckableSpec>) -> LessonSpec {
        LessonSpec {
            title: "T".into(),
            slug: None,
            order: None,
            sections,
            quizzes,
        }
    }

    #[test]
    fn unchanged_children_reuse_ids_and_state() {
        let old = OldContent {
            sections: vec![old_section("s1", "Intro", vec![old_checkable("p1", "f")])],
            quizzes: vec![old_checkable("q1", "g")],
        };
        let s = spec(
            vec![spec_section("Intro", vec![checkable("f")])],
            vec![checkable("g")],
        );
        let p = plan(&old, &s);
        assert_eq!(p.sections[0].id.as_deref(), Some("s1"));
        let r = p.sections[0].practice[0].reuse.as_ref().expect("reuse");
        assert_eq!(r.id, "p1");
        assert!(r.state.skip, "learner state must be carried");
        assert_eq!(
            p.sections[0].practice[0].cases,
            [Some(String::from("p1-case"))]
        );
        assert_eq!(p.quizzes[0].reuse.as_ref().expect("quiz reuse").id, "q1");
    }

    #[test]
    fn edited_children_get_fresh_positions() {
        let old = OldContent {
            sections: vec![old_section("s1", "Intro", vec![old_checkable("p1", "f")])],
            quizzes: vec![],
        };
        let mut edited = checkable("f");
        edited.prompt = "changed".into();
        let s = spec(vec![spec_section("Intro", vec![edited])], vec![]);
        let p = plan(&old, &s);
        assert_eq!(p.sections[0].id.as_deref(), Some("s1"));
        assert!(p.sections[0].practice[0].reuse.is_none());
        assert_eq!(p.sections[0].practice[0].cases, [None]);
    }

    #[test]
    fn reordered_children_still_match() {
        let old = OldContent {
            sections: vec![old_section(
                "s1",
                "Intro",
                vec![old_checkable("p1", "f"), old_checkable("p2", "g")],
            )],
            quizzes: vec![],
        };
        let s = spec(
            vec![spec_section("Intro", vec![checkable("g"), checkable("f")])],
            vec![],
        );
        let p = plan(&old, &s);
        assert_eq!(p.sections[0].practice[0].reuse.as_ref().unwrap().id, "p2");
        assert_eq!(p.sections[0].practice[1].reuse.as_ref().unwrap().id, "p1");
    }

    #[test]
    fn duplicate_old_children_match_lowest_ord_first() {
        let old = OldContent {
            sections: vec![old_section(
                "s1",
                "Intro",
                vec![old_checkable("p1", "f"), old_checkable("p2", "f")],
            )],
            quizzes: vec![],
        };
        let s = spec(vec![spec_section("Intro", vec![checkable("f")])], vec![]);
        let p = plan(&old, &s);
        assert_eq!(p.sections[0].practice[0].reuse.as_ref().unwrap().id, "p1");
    }

    #[test]
    fn changed_section_rekeys_whole_subtree() {
        let old = OldContent {
            sections: vec![old_section("s1", "Intro", vec![old_checkable("p1", "f")])],
            quizzes: vec![],
        };
        let s = spec(vec![spec_section("Renamed", vec![checkable("f")])], vec![]);
        let p = plan(&old, &s);
        assert_eq!(p.sections[0].id, None);
        assert!(
            p.sections[0].practice[0].reuse.is_none(),
            "practices match only within their matched section"
        );
    }
}
