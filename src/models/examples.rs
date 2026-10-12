//! Registry of representative examples for spec generation (adr/008).
//!
//! Each `*Spec`/`Data` type co-locates an example; this module exposes them to
//! `xtask gen-specs` through one pub fn so `#![deny(missing_docs)]` lands on this
//! item rather than on each example value. Typed example values are serialized
//! into the tables (so the JSON can't drift); authored field rules / notes are
//! co-located prose.

use crate::models::{
    build, config, course, goal, installs, issue, lesson, link, note, plan, progress, quiz,
    register, skip, venv, Data,
};

/// A generated spec-table entry keyed to a `docs/specs/` file.
#[derive(Debug, Clone)]
pub struct SpecEntry {
    /// Target spec file name, e.g. `"19-howto.md"`.
    pub file: &'static str,
    /// The generated markdown (table) to place between the markers.
    pub markdown: String,
}

/// All registered spec entries (the single source for `xtask gen-specs`).
pub fn all() -> Vec<SpecEntry> {
    vec![
        howto_entry(),
        course_spec_entry(),
        course_output_entry(),
        plan_spec_entry(),
        plan_output_entry(),
        goal_spec_entry(),
        goal_output_entry(),
        lesson_spec_entry(),
        lesson_output_entry(),
        quiz_output_entry(),
        venv_output_entry(),
        skip_output_entry(),
        note_spec_entry(),
        notes_output_entry(),
        progress_output_entry(),
        bug_feature_spec_entry(),
        bug_feature_output_entry(),
        config_output_entry(),
        register_output_entry(),
        build_output_entry(),
        link_output_entry(),
        installs_output_entry(),
    ]
}

/// Flatten every output-contract group's `rows()` into `(cmd, data)` pairs — the
/// single source shared with `output_table`/gen-specs (adr/008). The envelope
/// smoke test drives these exact values through `core::output::render`, so the
/// committed spec tables and the smoke test cannot drift.
pub fn envelope_examples() -> Vec<(&'static str, Data)> {
    let mut out: Vec<(&'static str, Data)> = Vec::new();
    out.push(("howto", howto_data()));
    for rows in [
        course::examples::rows(),
        plan::examples::rows(),
        goal::examples::rows(),
        lesson::examples::rows(),
        quiz::examples::rows(),
        venv::examples::rows(),
        skip::examples::rows(),
        note::examples::rows(),
        progress::examples::rows(),
        issue::examples::rows(),
        config::examples::rows(),
        register::examples::rows(),
        build::examples::rows(),
        link::examples::rows(),
        installs::examples::rows(),
    ] {
        for (cmd, _input, _note, data) in rows {
            out.push((cmd, data));
        }
    }
    out
}

/// Build a 3-column output-contract table from `(cmd, input, note, data)` rows;
/// the `data` cell is the serialized variant + the authored note.
fn output_table(rows: Vec<(&'static str, &'static str, &'static str, Data)>) -> String {
    let mut table = String::from("| cmd | input | `data` (ok) |\n|-----|-------|-------------|\n");
    for (cmd, input, note, data) in rows {
        let json = serde_json::to_string(&data).unwrap_or_default();
        let cell = if note.is_empty() {
            format!("`{json}`")
        } else {
            format!("`{json}` — {note}")
        };
        table.push_str(&format!("| `{cmd}` | {input} | {cell} |\n"));
    }
    table.trim_end().to_string()
}

/// The `howto` success-payload example (shared by its spec entry + the envelope smoke).
fn howto_data() -> Data {
    Data::Howto {
        howto: String::from("…"),
    }
}

fn howto_entry() -> SpecEntry {
    let data = serde_json::to_string(&howto_data()).unwrap_or_default();
    SpecEntry {
        file: "19-howto.md",
        markdown: format!(
            "| cmd | input | output (`data`) |\n|-----|-------|---------------|\n| `howto` | — | `{data}` |"
        ),
    }
}

fn course_spec_entry() -> SpecEntry {
    let yaml = serde_yml::to_string(&course::examples::spec()).unwrap_or_default();
    SpecEntry {
        file: "02-course-spec.md",
        markdown: format!(
            "| field | type | rule |\n|-------|------|------|\n\
             | slug | string? | derived from title if absent ([conventions](../data-model/02-conventions.md#slug-derivation)) |\n\
             | title | string | required, non-empty |\n\
             | goal | string | required, non-empty |\n\
             | description | string | optional, default `\"\"` |\n\n\
             Example:\n\n```yaml\n{yaml}\n```"
        ),
    }
}

fn course_output_entry() -> SpecEntry {
    SpecEntry {
        file: "08-course.md",
        markdown: output_table(course::examples::rows()),
    }
}

fn plan_spec_entry() -> SpecEntry {
    let yaml = serde_yml::to_string(&plan::examples::spec()).unwrap_or_default();
    SpecEntry {
        file: "04-plan-spec.md",
        markdown: format!(
            "| field | type | rule |\n|-------|------|------|\n\
             | title | string | required |\n\
             | goals | string[] | bullet goals; become `goals` rows on `confirm` (course scope) |\n\
             | links | `{{goal_index_<i>: lesson_id[]}}` | maps each goal to covering lessons. `<i>` is the 0-based index into `goals[]` (range-checked at `create`; lesson ids resolved at `confirm`). A goal absent from `links` gets `covered_by:[]`. |\n\n\
             Example:\n\n```yaml\n{yaml}\n```"
        ),
    }
}

fn plan_output_entry() -> SpecEntry {
    SpecEntry {
        file: "10-plan.md",
        markdown: output_table(plan::examples::rows()),
    }
}

fn goal_spec_entry() -> SpecEntry {
    let yaml = serde_yml::to_string(&goal::examples::spec()).unwrap_or_default();
    SpecEntry {
        file: "05-goal-spec.md",
        markdown: format!(
            "| field | type | rule |\n|-------|------|------|\n\
             | text | string | required, non-empty — the bullet goal |\n\
             | covered_by | string[] | default `[]` — lesson ids covering this goal (resolved on use; unresolved ⇒ `ValidationError`) |\n\n\
             Example:\n\n```yaml\n{yaml}\n```"
        ),
    }
}

fn goal_output_entry() -> SpecEntry {
    SpecEntry {
        file: "11-goal.md",
        markdown: output_table(goal::examples::rows()),
    }
}

fn lesson_spec_entry() -> SpecEntry {
    let yaml = serde_yml::to_string(&lesson::examples::spec()).unwrap_or_default();
    SpecEntry {
        file: "03-lesson-spec.md",
        markdown: format!(
            "| field | type | rule |\n|-------|------|------|\n\
             | title | string | required |\n\
             | slug | string? | derived from title if absent |\n\
             | order | int? | appended (max+1) if absent |\n\
             | sections[].snippets | `{{kind, content}}`[] | required; **`snippets[0].kind == \"markdown\"`**; each renders one cell |\n\
              | sections[].practice / quizzes | Checkable[] | array index ⇒ `ord` |\n\
              | cases[].compare | enum | `exact`(default) \\| `sorted` \\| `set` |\n\
              | cases[].args | array | default `[]` |\n\
              | cases[].kwargs | object | default `{{}}` |\n\
              | cases[].expected | any | required |\n\
              | practice[]/quizzes[].solution | string? | **author reference solution** (Python defining the fn `name`); author-only — never rendered/shown to learner; verified by `lesson verify` ([adr/015](../adr/015-reference-solution-verify.md)) |\n\n\
             **Checkable** (shared): `{{name, signature, prompt?, cases[]}}`. `expected` for a `sorted`/`set` case must be sortable/hashable else the case errors (`error:\"unsortable\"`/`\"unhashable\"`).\n\n\
             Example:\n\n```yaml\n{yaml}\n```"
        ),
    }
}

fn lesson_output_entry() -> SpecEntry {
    SpecEntry {
        file: "09-lesson.md",
        markdown: output_table(lesson::examples::rows()),
    }
}

fn quiz_output_entry() -> SpecEntry {
    SpecEntry {
        file: "12-quiz.md",
        markdown: output_table(quiz::examples::rows()),
    }
}

fn venv_output_entry() -> SpecEntry {
    SpecEntry {
        file: "22-venv.md",
        markdown: output_table(venv::examples::rows()),
    }
}

fn skip_output_entry() -> SpecEntry {
    SpecEntry {
        file: "23-skip.md",
        markdown: output_table(skip::examples::rows()),
    }
}

fn note_spec_entry() -> SpecEntry {
    let yaml = serde_yml::to_string(&note::examples::spec()).unwrap_or_default();
    SpecEntry {
        file: "06-note-spec.md",
        markdown: format!(
            "| field | type | rule |\n|-------|------|------|\n\
             | kind | enum | `gap\\|mistake\\|strength\\|pattern\\|progress` — required |\n\
             | tags | string[] | default `[]` |\n\
             | recurrence | enum | `new`(default) \\| `recurring` — **authored**; the system never overwrites it (it may surface `related_open` as a hint in `add` output — see [14-notes.md](14-notes.md)) |\n\
             | related | string? | a lesson/quiz id; stored as free text (no FK) — an unresolvable id is kept as-is, not rejected |\n\
             | text | string | required, non-empty |\n\n\
             Example:\n\n```yaml\n{yaml}\n```"
        ),
    }
}

fn notes_output_entry() -> SpecEntry {
    SpecEntry {
        file: "14-notes.md",
        markdown: output_table(note::examples::rows()),
    }
}

fn progress_output_entry() -> SpecEntry {
    SpecEntry {
        file: "13-progress.md",
        markdown: output_table(progress::examples::rows()),
    }
}

fn bug_feature_spec_entry() -> SpecEntry {
    let yaml = serde_yml::to_string(&issue::examples::spec()).unwrap_or_default();
    SpecEntry {
        file: "07-bug-feature-spec.md",
        markdown: format!(
            "| field | type | rule |\n|-------|------|------|\n\
             | title | string | required, non-empty |\n\
             | description | string | required, non-empty |\n\
             | repro | string? | bug only — passing it on a feature (or with `rationale`) ⇒ `ValidationError` |\n\
             | rationale | string? | feature only — passing it on a bug (or with `repro`) ⇒ `ValidationError` |\n\n\
             Example:\n\n```yaml\n{yaml}\n```"
        ),
    }
}

fn bug_feature_output_entry() -> SpecEntry {
    SpecEntry {
        file: "15-bug-feature.md",
        markdown: output_table(issue::examples::rows()),
    }
}

fn config_output_entry() -> SpecEntry {
    SpecEntry {
        file: "16-config.md",
        markdown: output_table(config::examples::rows()),
    }
}

fn register_output_entry() -> SpecEntry {
    SpecEntry {
        file: "21-register-deregister.md",
        markdown: output_table(register::examples::rows()),
    }
}

fn build_output_entry() -> SpecEntry {
    SpecEntry {
        file: "18-build-install-upgrade.md",
        markdown: output_table(build::examples::rows()),
    }
}

fn link_output_entry() -> SpecEntry {
    SpecEntry {
        file: "17-link.md",
        markdown: output_table(link::examples::rows()),
    }
}

fn installs_output_entry() -> SpecEntry {
    SpecEntry {
        file: "24-self.md",
        markdown: output_table(installs::examples::rows()),
    }
}

#[cfg(test)]
#[test]
fn howto_entry_serializes_to_expected_shape() {
    let entry = howto_entry();
    assert!(
        entry.markdown.contains(r#"`{"howto":"…"}`"#),
        "{}",
        entry.markdown
    );
}

/// gen-specs emits each `*::examples::spec()` as YAML; the example must round-trip
/// through serde_yml (serialize → parse → serialize is stable). Guards the YAML-only
/// spec-doc examples against a serde-yml/struct mismatch (adr/014).
#[cfg(test)]
#[test]
fn spec_examples_round_trip_as_yaml() {
    let lesson_yaml = serde_yml::to_string(&lesson::examples::spec()).expect("serialize lesson");
    let back: crate::models::lesson::LessonSpec =
        serde_yml::from_str(&lesson_yaml).unwrap_or_else(|e| panic!("parse: {e}\n{lesson_yaml}"));
    let again = serde_yml::to_string(&back).expect("reserialize");
    assert_eq!(lesson_yaml, again, "YAML round-trip not stable");
}

#[cfg(test)]
#[test]
fn output_tables_have_a_header_and_rows() {
    for e in [
        course_output_entry(),
        plan_output_entry(),
        goal_output_entry(),
        lesson_output_entry(),
        quiz_output_entry(),
        venv_output_entry(),
        skip_output_entry(),
        notes_output_entry(),
        progress_output_entry(),
        bug_feature_output_entry(),
        config_output_entry(),
        register_output_entry(),
        build_output_entry(),
        link_output_entry(),
    ] {
        assert!(
            e.markdown.contains("| cmd | input |"),
            "{}: {}",
            e.file,
            e.markdown
        );
    }
}

/// Parametrized envelope smoke test (Phase 10): every registered `data` example —
/// the same source gen-specs serializes into the spec tables — round-trips through
/// `core::output::render` into a well-formed `{"status":"ok",…,"data":{…}}`
/// envelope whose `data` matches the independently-serialized example. Doubles as
/// the spec golden: a `Data` shape change shows up here and in the spec table from
/// one source (adr/008).
#[cfg(test)]
#[test]
fn envelope_smoke_round_trips_every_example() {
    use crate::core::output::render;
    for (cmd, data) in envelope_examples() {
        let expected = serde_json::to_value(&data).expect("example must serialize");
        let (json, is_error) = render(Ok(data));
        assert!(!is_error, "{cmd}: envelope reported error");
        let env: serde_json::Value =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{cmd}: invalid JSON: {e}"));
        assert_eq!(env["status"], "ok", "{cmd}: status");
        let message_ok = env
            .get("message")
            .and_then(|m| m.as_str())
            .map(|m| !m.is_empty())
            .unwrap_or(false);
        assert!(message_ok, "{cmd}: empty message");
        assert_eq!(
            env.get("data").unwrap_or(&serde_json::Value::Null),
            &expected,
            "{cmd}: data round-trip mismatch"
        );
    }
}

/// The error envelope contract holds for every `CarpenterError` variant.
#[cfg(test)]
#[test]
fn envelope_renders_every_error_variant() {
    use crate::core::error::CarpenterError;
    use crate::core::output::render;
    use serde_json::json;
    let cases = [
        CarpenterError::NotFound("x".into()),
        CarpenterError::AlreadyExists("x".into()),
        CarpenterError::ValidationError("x".into()),
        CarpenterError::StoreError("x".into()),
        CarpenterError::ExecuteError {
            message: "x".into(),
            details: json!({}),
        },
        CarpenterError::Conflict("x".into()),
    ];
    for err in cases {
        let code = err.code();
        let (json, is_error) = render(Err(err));
        assert!(is_error, "{code}: envelope reported ok");
        let env: serde_json::Value =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{code}: invalid JSON: {e}"));
        assert_eq!(env["status"], "error", "{code}: status");
        assert_eq!(env["code"], code, "{code}: code field");
        assert!(env.get("details").is_some(), "{code}: missing details");
    }
}
