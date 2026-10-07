//! `lesson` commands — create/get (P4) + list/show/update/delete/sync (P5).

use std::fs;

use serde_json::{json, Map, Value};

use crate::core::compare::CompareMode;
use crate::core::error::CarpenterError;
use crate::core::exec;
use crate::core::store;
use crate::core::{db, notebook, status, store::Paths, time};
use crate::models::execute::{ExecError, ExecuteCells};
use crate::models::lesson::{
    CaseTree, CheckableTree, LessonCounts, LessonListItem, LessonProgress, LessonRow, LessonSpec,
    SectionTree, VerifyCheckable,
};
use crate::models::Data;

fn validate_lesson_spec(spec: &LessonSpec) -> Result<(), CarpenterError> {
    if spec.title.trim().is_empty() {
        return Err(CarpenterError::ValidationError(
            "title must be non-empty".into(),
        ));
    }
    for (i, sec) in spec.sections.iter().enumerate() {
        if sec.snippets.is_empty() {
            return Err(CarpenterError::ValidationError(format!(
                "section {i} must have at least one snippet"
            )));
        }
        if sec.snippets[0].kind != "markdown" {
            return Err(CarpenterError::ValidationError(format!(
                "section {i}: snippets[0] must be markdown"
            )));
        }
    }
    Ok(())
}

fn unique_lesson_slug(conn: &rusqlite::Connection, base: &str) -> Result<String, CarpenterError> {
    if !db::lesson_slug_taken(conn, base)? {
        return Ok(base.to_string());
    }
    let mut n = 2;
    loop {
        let candidate = format!("{base}-{n}");
        if !db::lesson_slug_taken(conn, &candidate)? {
            return Ok(candidate);
        }
        n += 1;
    }
}

fn serialize_snippets(
    snippets: &[crate::models::lesson::SnippetSpec],
    counter: &mut usize,
) -> String {
    let mut out = Vec::new();
    for s in snippets {
        *counter += 1;
        out.push(json!({
            "id": format!("sn{}", *counter),
            "kind": s.kind,
            "content": s.content,
        }));
    }
    serde_json::to_string(&out).unwrap_or_else(|_| String::from("[]"))
}

fn insert_cases(
    conn: &rusqlite::Connection,
    owner_type: &str,
    owner_id: &str,
    cases: &[crate::models::lesson::CaseSpec],
    total: &mut i64,
) -> Result<(), CarpenterError> {
    for (c_ord, case) in cases.iter().enumerate() {
        let cid = db::next_id(conn, "test_cases", "c")?;
        let args = serde_json::to_string(&case.args).unwrap_or_else(|_| String::from("[]"));
        let kwargs = serde_json::to_string(&case.kwargs).unwrap_or_else(|_| String::from("{}"));
        let expected =
            serde_json::to_string(&case.expected).unwrap_or_else(|_| String::from("null"));
        let compare = match case.compare {
            CompareMode::Exact => "exact",
            CompareMode::Sorted => "sorted",
            CompareMode::Set => "set",
        };
        db::insert_test_case(
            conn,
            &cid,
            owner_type,
            owner_id,
            &args,
            &kwargs,
            &expected,
            compare,
            c_ord as i64,
        )?;
        *total += 1;
    }
    Ok(())
}

/// Insert a lesson's sections/practice/quizzes/cases from a spec; return counts.
fn insert_lesson_content(
    conn: &rusqlite::Connection,
    lesson_id: &str,
    spec: &LessonSpec,
) -> Result<LessonCounts, CarpenterError> {
    let mut counts = LessonCounts {
        sections: 0,
        practice: 0,
        quizzes: 0,
        cases: 0,
    };
    let mut sn = 0usize;
    for (s_ord, section) in spec.sections.iter().enumerate() {
        let sid = db::next_id(conn, "sections", "s")?;
        let snippets_json = serialize_snippets(&section.snippets, &mut sn);
        db::insert_section(
            conn,
            &sid,
            lesson_id,
            &section.title,
            &snippets_json,
            s_ord as i64,
        )?;
        counts.sections += 1;
        for (p_ord, prac) in section.practice.iter().enumerate() {
            let pid = db::next_id(conn, "practice", "p")?;
            db::insert_practice(
                conn,
                &pid,
                &sid,
                &prac.name,
                &prac.signature,
                &prac.prompt,
                prac.solution.as_deref().unwrap_or(""),
                p_ord as i64,
            )?;
            counts.practice += 1;
            insert_cases(conn, "practice", &pid, &prac.cases, &mut counts.cases)?;
        }
    }
    for (q_ord, quiz) in spec.quizzes.iter().enumerate() {
        let qid = db::next_id(conn, "quizzes", "q")?;
        db::insert_quiz(
            conn,
            &qid,
            lesson_id,
            &quiz.name,
            &quiz.signature,
            &quiz.prompt,
            quiz.solution.as_deref().unwrap_or(""),
            q_ord as i64,
        )?;
        counts.quizzes += 1;
        insert_cases(conn, "quiz", &qid, &quiz.cases, &mut counts.cases)?;
    }
    Ok(counts)
}

pub(crate) fn lesson_dir(paths: &Paths, course: &str, slug: &str, ord: i64) -> std::path::PathBuf {
    paths
        .course(course)
        .join("lessons")
        .join(format!("{ord:02}-{slug}"))
}

/// Create a lesson from a spec: DB inserts + render notebook + helper.
pub fn create(paths: &Paths, course_slug: &str, spec_text: &str) -> Result<Data, CarpenterError> {
    let spec: LessonSpec = store::parse_spec(spec_text)?;
    validate_lesson_spec(&spec)?;
    let conn = db::open_course(paths, course_slug)?;
    let now = time::now_iso();
    let base = match &spec.slug {
        Some(s) => {
            store::validate_slug(s)?;
            s.clone()
        }
        None => store::slugify(&spec.title)?,
    };
    let slug = unique_lesson_slug(&conn, &base)?;
    let lesson_id = slug.clone();
    let ord = match spec.order {
        Some(o) => {
            db::insert_lesson(&conn, &lesson_id, &slug, &spec.title, o, &now, &now)?;
            o
        }
        None => db::insert_lesson_auto_ord(&conn, &lesson_id, &slug, &spec.title, &now, &now)?,
    };
    let counts = insert_lesson_content(&conn, &lesson_id, &spec)?;

    let dir = lesson_dir(paths, course_slug, &slug, ord);
    fs::create_dir_all(&dir).map_err(store::io_to_store)?;
    let nb = notebook::render_to_string(&conn, &lesson_id)?;
    store::atomic_write(&dir.join("lesson.ipynb"), nb.as_bytes())?;
    store::atomic_write(
        &dir.join("helper.py"),
        crate::core::helper::HELPER_PY.as_bytes(),
    )?;

    Ok(Data::LessonCreate {
        id: lesson_id,
        slug,
        path: dir.display().to_string(),
        counts,
    })
}

/// Show the full lesson tree.
pub fn get(paths: &Paths, course_slug: &str, id: &str) -> Result<Data, CarpenterError> {
    let conn = db::open_course(paths, course_slug)?;
    let lesson = db::get_lesson(&conn, id)?;
    let status_str = status::lesson_status(&conn, id)?
        .map(|s| s.as_str().to_string())
        .unwrap_or(lesson.status);
    let mut sections = Vec::new();
    for s in db::list_sections(&conn, id)? {
        let mut practice = Vec::new();
        for p in db::list_practice(&conn, &s.id)? {
            practice.push(checkable_tree(&conn, "practice", &p)?);
        }
        sections.push(SectionTree {
            id: s.id,
            title: s.title,
            snippets: notebook::parse_snippets(&s.snippets),
            ord: s.ord,
            practice,
        });
    }
    let mut quizzes = Vec::new();
    for q in db::list_quizzes(&conn, id)? {
        quizzes.push(checkable_tree(&conn, "quiz", &q)?);
    }
    Ok(Data::LessonGet {
        id: lesson.id,
        slug: lesson.slug,
        title: lesson.title,
        ord: lesson.ord,
        status: status_str,
        skip: lesson.skip,
        sections,
        quizzes,
    })
}

/// List lessons.
pub fn list(paths: &Paths, course_slug: &str) -> Result<Data, CarpenterError> {
    let conn = db::open_course(paths, course_slug)?;
    let mut lessons = Vec::new();
    for l in db::list_lessons(&conn)? {
        let st = status::lesson_status(&conn, &l.id)?.map(|s| s.as_str().to_string());
        lessons.push(LessonListItem {
            id: l.id,
            title: l.title,
            ord: l.ord,
            status: st.unwrap_or(l.status),
            skip: l.skip,
        });
    }
    Ok(Data::LessonList {
        lessons,
        errors: Vec::new(),
    })
}

/// Show a lesson's status + progress.
pub fn show(paths: &Paths, course_slug: &str, id: &str) -> Result<Data, CarpenterError> {
    let conn = db::open_course(paths, course_slug)?;
    let lesson = db::get_lesson(&conn, id)?;
    let status_str = status::lesson_status(&conn, id)?
        .map(|s| s.as_str().to_string())
        .unwrap_or(lesson.status);
    let c = db::lesson_show_counts(&conn, id)?;
    Ok(Data::LessonShow {
        id: lesson.id,
        title: lesson.title,
        status: status_str,
        skip: lesson.skip,
        progress: LessonProgress {
            sections: c.sections,
            practice: c.practice,
            quizzes: c.quizzes,
            passing: c.passing,
            total: c.total,
        },
    })
}

/// Update a lesson from a spec (requires `--force`); re-renders the notebook.
pub fn update(
    paths: &Paths,
    course_slug: &str,
    id: &str,
    spec_text: &str,
    force: bool,
) -> Result<Data, CarpenterError> {
    if !force {
        return Err(CarpenterError::Conflict(format!(
            "update requires --force: lesson {id}"
        )));
    }
    let spec: LessonSpec = store::parse_spec(spec_text)?;
    validate_lesson_spec(&spec)?;
    let conn = db::open_course(paths, course_slug)?;
    let lesson = db::get_lesson(&conn, id)?;
    let now = time::now_iso();
    db::delete_lesson_content(&conn, id)?;
    insert_lesson_content(&conn, id, &spec)?;
    db::update_lesson_meta(&conn, id, &spec.title, &now)?;

    let dir = lesson_dir(paths, course_slug, &lesson.slug, lesson.ord);
    if dir.exists() {
        let nb = notebook::render_to_string(&conn, id)?;
        store::atomic_write(&dir.join("lesson.ipynb"), nb.as_bytes())?;
    }
    Ok(Data::LessonUpdate {
        id: id.into(),
        updated: LessonRow {
            id: id.into(),
            slug: lesson.slug,
            title: spec.title,
            ord: lesson.ord,
            status: lesson.status,
            skip: lesson.skip,
            created_at: lesson.created_at,
            updated_at: now,
        },
    })
}

/// Delete a lesson (requires `--force`).
pub fn delete(
    paths: &Paths,
    course_slug: &str,
    id: &str,
    force: bool,
) -> Result<Data, CarpenterError> {
    if !force {
        return Err(CarpenterError::Conflict(format!(
            "delete requires --force: lesson {id}"
        )));
    }
    let conn = db::open_course(paths, course_slug)?;
    let lesson = db::get_lesson(&conn, id)?;
    db::delete_lesson(&conn, id)?;
    let dir = lesson_dir(paths, course_slug, &lesson.slug, lesson.ord);
    if dir.exists() {
        let _ = fs::remove_dir_all(&dir);
    }
    Ok(Data::LessonDelete {
        id: id.into(),
        deleted: true,
    })
}

/// Sync a lesson's notebook against the DB (3-way stub preservation).
pub fn sync(
    paths: &Paths,
    course_slug: &str,
    id: &str,
    force: bool,
) -> Result<Data, CarpenterError> {
    let conn = db::open_course(paths, course_slug)?;
    let lesson = db::get_lesson(&conn, id)?;
    let dir = lesson_dir(paths, course_slug, &lesson.slug, lesson.ord);
    let nb_path = dir.join("lesson.ipynb");
    let old: Value = if nb_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&nb_path).map_err(store::io_to_store)?)
            .unwrap_or_else(|_| json!({"cells": []}))
    } else {
        json!({"cells": []})
    };
    let (new_nb, conflicts) = notebook::sync_notebook(&old, &conn, id, force)?;
    fs::create_dir_all(&dir).map_err(store::io_to_store)?;
    let pretty = serde_json::to_string_pretty(&new_nb)
        .map_err(|e| CarpenterError::StoreError(format!("notebook encode failed: {e}")))?;
    store::atomic_write(&nb_path, pretty.as_bytes())?;
    Ok(Data::LessonSync {
        id: id.into(),
        synced: true,
        conflicts,
    })
}

/// Execute a lesson's notebook end-to-end in the course venv.
///
/// Strict (default): the first errored cell ⇒ `ExecuteError`. `--allow-errors`:
/// runs every cell and returns the full error list + counts. Always executed
/// internally with `allow_errors=True` so all cell outputs are captured.
pub fn execute(
    paths: &Paths,
    course_slug: &str,
    id: &str,
    timeout: u64,
    allow_errors: bool,
) -> Result<Data, CarpenterError> {
    let conn = db::open_course(paths, course_slug)?;
    let lesson = db::get_lesson(&conn, id)?;
    let course_dir = paths.course(course_slug);
    if !course_dir.join(".venv").exists() {
        return Err(CarpenterError::StoreError(format!(
            "no course venv for {course_slug} — run `carpenter venv create` first"
        )));
    }
    let dir = lesson_dir(paths, course_slug, &lesson.slug, lesson.ord);
    exec::run_nbconvert(&course_dir, &dir, timeout)?;

    let nb_path = dir.join("lesson.ipynb");
    let nb_text = std::fs::read_to_string(&nb_path).map_err(store::io_to_store)?;
    let nb: Value = serde_json::from_str(&nb_text)
        .map_err(|e| CarpenterError::StoreError(format!("notebook unreadable: {e}")))?;
    let errors: Vec<ExecError> = notebook::scan_errors(&nb)
        .into_iter()
        .map(|e| ExecError {
            index: e.index,
            ename: e.ename,
            evalue: e.evalue,
        })
        .collect();
    if !allow_errors {
        if let Some(first) = errors.first() {
            return Err(CarpenterError::ExecuteError {
                message: format!("cell {} errored: {}", first.index, first.ename),
                details: json!({"index": first.index, "ename": first.ename, "evalue": first.evalue}),
            });
        }
    }
    let total = nb["cells"]
        .as_array()
        .map(|c| {
            c.iter()
                .filter(|cell| cell.get("cell_type").and_then(|v| v.as_str()) == Some("code"))
                .count() as i64
        })
        .unwrap_or(0);
    Ok(Data::LessonExecute {
        id: id.into(),
        executed: true,
        cells: ExecuteCells {
            total,
            ran: total,
            errored: errors.len() as i64,
        },
        errors,
    })
}

/// One checkable in the verify payload handed to `verify.py` (author-only; the
/// `expected` values cross the subprocess boundary but are never echoed back).
#[derive(serde::Serialize)]
struct VerifyPayloadItem {
    owner_type: String,
    owner_id: String,
    name: String,
    solution: String,
    cases: Vec<VerifyPayloadCase>,
}

/// One case in the verify payload.
#[derive(serde::Serialize)]
struct VerifyPayloadCase {
    case_id: String,
    args: Value,
    kwargs: Map<String, Value>,
    expected: Value,
    compare: String,
}

fn compare_str(m: CompareMode) -> &'static str {
    match m {
        CompareMode::Exact => "exact",
        CompareMode::Sorted => "sorted",
        CompareMode::Set => "set",
    }
}

/// Build verify payload items from a spec (`--spec` mode; owner_id = fn name).
fn payload_items_from_spec(spec: &LessonSpec) -> Vec<VerifyPayloadItem> {
    let mut out = Vec::new();
    for sec in &spec.sections {
        for prac in &sec.practice {
            out.push(VerifyPayloadItem {
                owner_type: String::from("practice"),
                owner_id: prac.name.clone(),
                name: prac.name.clone(),
                solution: prac.solution.clone().unwrap_or_default(),
                cases: prac
                    .cases
                    .iter()
                    .enumerate()
                    .map(|(i, c)| VerifyPayloadCase {
                        case_id: format!("{}-{}", prac.name, i),
                        args: Value::Array(c.args.clone()),
                        kwargs: c.kwargs.clone(),
                        expected: c.expected.clone(),
                        compare: compare_str(c.compare).into(),
                    })
                    .collect(),
            });
        }
    }
    for quiz in &spec.quizzes {
        out.push(VerifyPayloadItem {
            owner_type: String::from("quiz"),
            owner_id: quiz.name.clone(),
            name: quiz.name.clone(),
            solution: quiz.solution.clone().unwrap_or_default(),
            cases: quiz
                .cases
                .iter()
                .enumerate()
                .map(|(i, c)| VerifyPayloadCase {
                    case_id: format!("{}-{}", quiz.name, i),
                    args: Value::Array(c.args.clone()),
                    kwargs: c.kwargs.clone(),
                    expected: c.expected.clone(),
                    compare: compare_str(c.compare).into(),
                })
                .collect(),
        });
    }
    out
}

/// Build verify payload items from stored rows (`<id>` mode).
fn payload_items_from_db(
    conn: &rusqlite::Connection,
    lesson_id: &str,
) -> Result<Vec<VerifyPayloadItem>, CarpenterError> {
    fn case_payload(cc: &db::CaseDb) -> VerifyPayloadCase {
        VerifyPayloadCase {
            case_id: cc.id.clone(),
            args: serde_json::from_str(&cc.args).unwrap_or_else(|_| Value::Array(vec![])),
            kwargs: serde_json::from_str(&cc.kwargs).unwrap_or_else(|_| Map::new()),
            expected: serde_json::from_str(&cc.expected).unwrap_or(Value::Null),
            compare: cc.compare.clone(),
        }
    }
    let mut out = Vec::new();
    for sec in db::list_sections(conn, lesson_id)? {
        for p in db::list_practice(conn, &sec.id)? {
            let cases = db::list_cases(conn, "practice", &p.id)?;
            out.push(VerifyPayloadItem {
                owner_type: String::from("practice"),
                owner_id: p.id.clone(),
                name: p.name.clone(),
                solution: p.solution.clone(),
                cases: cases.iter().map(case_payload).collect(),
            });
        }
    }
    for q in db::list_quizzes(conn, lesson_id)? {
        let cases = db::list_cases(conn, "quiz", &q.id)?;
        out.push(VerifyPayloadItem {
            owner_type: String::from("quiz"),
            owner_id: q.id.clone(),
            name: q.name.clone(),
            solution: q.solution.clone(),
            cases: cases.iter().map(case_payload).collect(),
        });
    }
    Ok(out)
}

static VERIFY_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Stage `verify.py` + payload in a temp dir, run it via `uv run` in the course
/// venv, parse the per-case results. Cleans up the temp dir. The subprocess is
/// the isolation boundary for author code (same as `quiz run`).
fn run_verify(
    course_dir: &std::path::Path,
    timeout: u64,
    items: &[VerifyPayloadItem],
) -> Result<Vec<VerifyCheckable>, CarpenterError> {
    use std::io::Write;
    let n = VERIFY_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("carpenter-verify-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(store::io_to_store)?;
    let payload = serde_json::json!({"timeout": timeout, "checkables": items});
    let payload_path = tmp.join("payload.json");
    let script_path = tmp.join("verify.py");
    let res = (|| -> Result<Vec<VerifyCheckable>, CarpenterError> {
        let mut f = std::fs::File::create(&payload_path).map_err(store::io_to_store)?;
        f.write_all(payload.to_string().as_bytes())
            .map_err(store::io_to_store)?;
        std::fs::write(&script_path, crate::core::verify::VERIFY_PY).map_err(store::io_to_store)?;
        let script = script_path
            .to_str()
            .ok_or_else(|| CarpenterError::StoreError("verify.py temp path not UTF-8".into()))?;
        let payload_arg = payload_path
            .to_str()
            .ok_or_else(|| CarpenterError::StoreError("payload temp path not UTF-8".into()))?;
        let out = exec::run_uv_or_store(&["run", "python", script, payload_arg], course_dir)?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let parsed: Value = serde_json::from_str(stdout.trim()).map_err(|e| {
            CarpenterError::StoreError(format!("verify output unparseable: {e}; stdout: {stdout}"))
        })?;
        let checkables_val = parsed
            .get("checkables")
            .cloned()
            .unwrap_or_else(|| Value::Array(vec![]));
        serde_json::from_value::<Vec<VerifyCheckable>>(checkables_val)
            .map_err(|e| CarpenterError::StoreError(format!("verify results malformed: {e}")))
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    res
}

/// `lesson verify (<id> | --spec <FILE|->) [--timeout <SECS>]` ([adr/015]).
///
/// Runs each author reference `solution` against its own cases in the course
/// venv. `--spec` is the pre-create key-lock (replaces the throwaway script);
/// `<id>` re-verifies stored solutions after edits. Requires `carpenter venv
/// create` first. Results carry `actual`/`error`, never `expected`.
pub fn verify(
    paths: &Paths,
    course_slug: &str,
    id: Option<&str>,
    spec_text: Option<&str>,
    timeout: u64,
) -> Result<Data, CarpenterError> {
    let course_dir = paths.course(course_slug);
    // Resolve the payload (validates id/spec XOR, parses the spec, reads the DB)
    // before the venv check so a usage error or NotFound surfaces first.
    let (lesson_id, items) = match (id, spec_text) {
        (Some(lid), None) => {
            let conn = db::open_course(paths, course_slug)?;
            db::get_lesson(&conn, lid)?; // NotFound if absent
            (Some(lid.to_string()), payload_items_from_db(&conn, lid)?)
        }
        (None, Some(json)) => {
            let spec: LessonSpec = store::parse_spec(json)?;
            (None, payload_items_from_spec(&spec))
        }
        _ => {
            return Err(CarpenterError::ValidationError(
                "lesson verify requires exactly one of <id> or --spec".into(),
            ))
        }
    };
    if !course_dir.join(".venv").exists() {
        return Err(CarpenterError::StoreError(format!(
            "no course venv for {course_slug} — run `carpenter venv create` first"
        )));
    }
    let checkables = run_verify(&course_dir, timeout, &items)?;
    let checked = checkables.iter().filter(|c| c.has_solution).count() as i64;
    let passing = checkables
        .iter()
        .filter(|c| c.has_solution && c.total > 0 && c.passed == c.total)
        .count() as i64;
    let failing = checked - passing;
    Ok(Data::LessonVerify {
        lesson_id,
        checked,
        passing,
        failing,
        checkables,
    })
}

/// The YAML lesson-spec template emitted by `lesson new` ([adr/014](../../docs/adr/014-yaml-spec-input.md),
/// [adr/015](../../docs/adr/015-reference-solution-verify.md)). Block scalars
/// keep multi-line `content`/`solution` as-is; signatures are quoted (they end
/// in `:`).
const LESSON_TEMPLATE: &str = r#"# carpenter lesson spec (YAML; docs/specs/03-lesson-spec.md, adr/014).
# Quote any value containing ':' (signatures always do). Block scalars (|)
# keep multi-line markdown/code as-is — no \n escaping.
title: <lesson title>
slug: <lesson-slug>          # optional; derived from title if omitted
# order: 1                   # optional; appended (max+1) if omitted
sections:
  - title: <section title>
    snippets:
      - kind: markdown       # snippets[0] must be markdown
        content: |
          ## Heading

          Prose here. LaTeX works: $$a^2 + b^2 = c^2$$
      - kind: code
        content: |
          import numpy as np
          np.array([1, 2, 3])
    practice:
      - name: <fn_name>
        signature: "def <fn_name>(x):"
        prompt: One-line prompt shown in the learner stub.
        solution: |          # author reference (adr/015); `lesson verify` runs it
          def <fn_name>(x):
              return x
        cases:
          - compare: exact   # exact | sorted | set
            args: [1]
            kwargs: {}
            expected: 1
quizzes:
  - name: <quiz_fn>
    signature: "def <quiz_fn>(x):"
    solution: |
      def <quiz_fn>(x):
          return x
    cases:
      - args: [1]
        expected: 1
"#;

/// `lesson new [--out <FILE>]` — emit a YAML lesson-spec template to fill in.
///
/// No DB, no course context (the dispatch skips course resolution for `new`).
/// Prints to stdout by default; `--out` writes a file. The template uses block
/// scalars for multi-line content and includes a `solution` field (verified by
/// `lesson verify`).
pub fn new(out: Option<&str>) -> Result<Data, CarpenterError> {
    if let Some(path) = out {
        store::atomic_write(std::path::Path::new(path), LESSON_TEMPLATE.as_bytes())?;
        return Ok(Data::LessonNew {
            yaml: None,
            written_to: Some(path.to_string()),
        });
    }
    Ok(Data::LessonNew {
        yaml: Some(LESSON_TEMPLATE.to_string()),
        written_to: None,
    })
}

fn checkable_tree(
    conn: &rusqlite::Connection,
    owner_type: &str,
    c: &db::CheckableDb,
) -> Result<CheckableTree, CarpenterError> {
    let mut cases = Vec::new();
    for cc in db::list_cases(conn, owner_type, &c.id)? {
        let args: Vec<Value> = serde_json::from_str(&cc.args).unwrap_or_default();
        let kwargs: Map<String, Value> = serde_json::from_str(&cc.kwargs).unwrap_or_default();
        let expected: Value = serde_json::from_str(&cc.expected).unwrap_or(Value::Null);
        cases.push(CaseTree {
            id: cc.id,
            args,
            kwargs,
            expected,
            compare: cc.compare,
            ord: cc.ord,
        });
    }
    Ok(CheckableTree {
        id: c.id.clone(),
        name: c.name.clone(),
        signature: c.signature.clone(),
        prompt: c.prompt.clone(),
        cases,
        skip: c.skip,
        pass_or_fail: c.pass_or_fail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::testutil;

    const SPEC: &str = "title: Arrays 101\nslug: arrays-101\norder: 1\nsections:\n  - title: Intro\n    snippets:\n      - kind: markdown\n        content: \"# hi\"\n    practice:\n      - name: sum_array\n        signature: \"def sum_array(arr):\"\n        prompt: sum it\n        solution: |\n          def sum_array(arr):\n              return sum(arr)\n        cases:\n          - args:\n              - [1, 2]\n            expected: 3\n          - args:\n              - []\n            expected: 0\nquizzes:\n  - name: max_value\n    signature: \"def max_value(arr):\"\n    cases:\n      - args:\n          - [3, 1, 2]\n        expected: 3\n        compare: set\n";

    fn lesson_path(paths: &Paths, course: &str) -> std::path::PathBuf {
        paths.course(course).join("lessons").join("01-arrays-101")
    }

    #[test]
    fn create_ok_renders_notebook_and_helper() {
        let (paths, slug) = testutil::setup();
        let data = create(&paths, &slug, SPEC).expect("create");
        let Data::LessonCreate { counts, path, .. } = data else {
            panic!("LessonCreate");
        };
        assert_eq!(counts.cases, 3);
        let dir = std::path::Path::new(&path);
        assert!(dir.join("lesson.ipynb").exists());
        assert!(dir.join("helper.py").exists());
    }

    #[test]
    fn create_rejects_first_snippet_not_markdown() {
        let (paths, slug) = testutil::setup();
        let bad = "title: x\nsections:\n  - title: s\n    snippets:\n      - kind: code\n        content: x\nquizzes: []\n";
        let err = create(&paths, &slug, bad).unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
    }

    #[test]
    fn create_rejects_non_kebab_slug() {
        let (paths, slug) = testutil::setup();
        let bad = "title: x\nslug: unicode-ñ\nsections:\n  - title: s\n    snippets:\n      - kind: markdown\n        content: x\nquizzes: []\n";
        let err = create(&paths, &slug, bad).unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)), "{err}");
    }

    #[test]
    fn create_dedups_slug_on_collision() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        // Same slug, no explicit order: auto-ord allocates the next slot and
        // the slug dedups (an explicit duplicate `order` is a Conflict —
        // see create_explicit_duplicate_order_conflicts).
        let dup_spec = SPEC.replace("order: 1\n", "");
        let Data::LessonCreate { id, .. } = create(&paths, &slug, &dup_spec).expect("dup") else {
            panic!();
        };
        assert!(id.starts_with("arrays-101-"), "{id}");
    }

    fn bare_spec(i: usize) -> String {
        format!(
            "title: Lesson {i}\nslug: lesson-{i}\nsections:\n  - title: s\n    snippets:\n      - kind: markdown\n        content: x\nquizzes: []\n"
        )
    }

    #[test]
    fn create_concurrent_allocates_distinct_ords() {
        let (paths, slug) = testutil::setup();
        let n = 8;
        let mut handles = Vec::new();
        for i in 0..n {
            let paths = paths.clone();
            let slug = slug.clone();
            let spec = bare_spec(i);
            handles.push(std::thread::spawn(move || {
                create(&paths, &slug, &spec).map(|_| ())
            }));
        }
        for h in handles {
            h.join().expect("thread").expect("create");
        }
        let conn = db::open_course(&paths, &slug).unwrap();
        let ords = db::list_lessons(&conn)
            .unwrap()
            .into_iter()
            .map(|l| l.ord)
            .collect::<Vec<_>>();
        assert_eq!(ords.len(), n);
        let mut distinct = ords.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), n, "duplicate ords: {ords:?}");
    }

    #[test]
    fn create_explicit_duplicate_order_conflicts() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let dup = "title: Other\nslug: other-lesson\norder: 1\nsections:\n  - title: s\n    snippets:\n      - kind: markdown\n        content: x\nquizzes: []\n";
        let err = create(&paths, &slug, dup).unwrap_err();
        assert!(
            matches!(err, CarpenterError::Conflict(ref m) if m.contains("ord 1")),
            "{err}"
        );
    }

    #[test]
    fn open_resequences_duplicate_ords_and_enforces_index() {
        let (paths, slug) = testutil::setup();
        for i in 0..3 {
            create(&paths, &slug, &bare_spec(i)).unwrap();
        }
        {
            let conn = db::open_course(&paths, &slug).unwrap();
            // Simulate a legacy dirty DB: drop the uniqueness index a
            // pre-adr/017 database never had, then corrupt an ord.
            conn.execute("DROP INDEX idx_lessons_ord", []).unwrap();
            conn.execute("UPDATE lessons SET ord = 1 WHERE slug = 'lesson-1'", [])
                .unwrap();
        }
        let conn = db::open_course(&paths, &slug).unwrap();
        let ords = db::list_lessons(&conn)
            .unwrap()
            .into_iter()
            .map(|l| l.ord)
            .collect::<Vec<_>>();
        let mut distinct = ords.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            ords.len(),
            "duplicate ords survived: {ords:?}"
        );
        let idx: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_lessons_ord'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn get_returns_full_tree() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let data = get(&paths, &slug, "arrays-101").expect("get");
        let Data::LessonGet {
            sections, quizzes, ..
        } = data
        else {
            panic!();
        };
        assert_eq!(sections.len(), 1);
        assert_eq!(quizzes[0].cases[0].compare, "set");
    }

    #[test]
    fn list_ok() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let Data::LessonList { lessons, .. } = list(&paths, &slug).expect("list") else {
            panic!();
        };
        assert_eq!(lessons.len(), 1);
        assert_eq!(lessons[0].status, "not_started");
    }

    #[test]
    fn show_ok() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let Data::LessonShow { progress, .. } = show(&paths, &slug, "arrays-101").expect("show")
        else {
            panic!();
        };
        assert_eq!(progress.sections, 1);
        assert_eq!(progress.total, 2); // 1 practice + 1 quiz
    }

    #[test]
    fn update_ok() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let Data::LessonUpdate { updated, .. } = update(
            &paths,
            &slug,
            "arrays-101",
            "title: Arrays 202\nsections:\n  - title: s\n    snippets:\n      - kind: markdown\n        content: x\nquizzes: []\n",
            true,
        )
        .expect("update")
        else {
            panic!();
        };
        assert_eq!(updated.title, "Arrays 202");
    }

    #[test]
    fn update_without_force_conflicts() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let err = update(&paths, &slug, "arrays-101", SPEC, false).unwrap_err();
        assert!(matches!(err, CarpenterError::Conflict(_)));
    }

    #[test]
    fn delete_ok() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let _ = delete(&paths, &slug, "arrays-101", true).expect("delete");
        assert!(!lesson_path(&paths, &slug).exists());
    }

    #[test]
    fn delete_without_force_conflicts() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let err = delete(&paths, &slug, "arrays-101", false).unwrap_err();
        assert!(matches!(err, CarpenterError::Conflict(_)));
    }

    #[test]
    fn execute_requires_venv() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let err = execute(&paths, &slug, "arrays-101", 30, false).unwrap_err();
        assert!(matches!(err, CarpenterError::StoreError(_)));
    }

    #[test]
    fn verify_requires_venv_spec_mode() {
        let (paths, slug) = testutil::setup();
        // venv guard fires before the spec is even parsed.
        let err = verify(&paths, &slug, None, Some(SPEC), 30).unwrap_err();
        assert!(matches!(err, CarpenterError::StoreError(_)));
    }

    #[test]
    fn verify_requires_exactly_one_of_id_or_spec() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        // neither ⇒ ValidationError (the dispatch normally prevents this; guard
        // the invariant independently of clap).
        let err = verify(&paths, &slug, None, None, 30).unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
    }

    #[test]
    fn new_emits_yaml_template_to_stdout() {
        let Data::LessonNew { yaml, written_to } = new(None).expect("new") else {
            panic!("LessonNew");
        };
        assert!(written_to.is_none());
        let yaml = yaml.expect("yaml present in print mode");
        assert!(yaml.contains("title:"));
        assert!(yaml.contains("sections:"));
        assert!(yaml.contains("solution:"));
        assert!(yaml.contains("\"def <fn_name>(x):\"")); // quoted signature
        assert!(yaml.contains("content: |")); // block scalar
    }

    #[test]
    fn new_writes_file_with_out() {
        let path = std::env::temp_dir().join(format!(
            "carpenter-new-{}-{}.yaml",
            std::process::id(),
            std::sync::atomic::AtomicUsize::new(0)
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let Data::LessonNew { yaml, written_to } =
            new(Some(path.to_str().unwrap())).expect("new --out")
        else {
            panic!("LessonNew");
        };
        assert!(yaml.is_none());
        assert_eq!(written_to.as_deref(), Some(path.to_str().unwrap()));
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("title:"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    #[ignore = "needs a course venv + python (run manually)"]
    fn verify_locks_keys_with_solution() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        crate::commands::venv::create(&paths, &slug, None).expect("venv");
        // <id> mode: only sum_array carries a `solution` (max_value has none).
        let Data::LessonVerify {
            checked, passing, ..
        } = verify(&paths, &slug, Some("arrays-101"), None, 30).expect("verify")
        else {
            panic!("LessonVerify");
        };
        assert_eq!(checked, 1);
        assert_eq!(passing, 1);
    }

    #[test]
    #[ignore = "needs a course venv + nbconvert (run manually)"]
    fn execute_runs_strict_and_allow_errors() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        crate::commands::venv::create(&paths, &slug, None).expect("venv");
        let _ = execute(&paths, &slug, "arrays-101", 30, true).expect("execute");
    }

    #[test]
    fn sync_preserves_learner_edited_stub() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let nb_path = lesson_path(&paths, &slug).join("lesson.ipynb");
        // simulate a learner filling the practice stub
        let mut nb: Value =
            serde_json::from_str(&std::fs::read_to_string(&nb_path).unwrap()).unwrap();
        nb["cells"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .for_each(|c| {
                if c.get("metadata").and_then(|m| m.get("managed")) == Some(&json!("practice-stub"))
                {
                    c["source"] = json!("def sum_array(arr):\n    return sum(arr)\n");
                }
            });
        std::fs::write(&nb_path, nb.to_string()).unwrap();

        let Data::LessonSync { conflicts, .. } =
            sync(&paths, &slug, "arrays-101", false).expect("sync")
        else {
            panic!();
        };
        assert!(conflicts.is_empty(), "{conflicts:?}"); // db unchanged ⇒ preserved silently
        let after: Value =
            serde_json::from_str(&std::fs::read_to_string(&nb_path).unwrap()).unwrap();
        let preserved = after["cells"].as_array().unwrap().iter().any(|c| {
            c.get("metadata").and_then(|m| m.get("managed")) == Some(&json!("practice-stub"))
                && c["source"].as_str() == Some("def sum_array(arr):\n    return sum(arr)\n")
        });
        assert!(preserved, "learner fill was not preserved");
    }

    #[test]
    fn sync_conflicts_when_db_changed_under_learner_edit() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let nb_path = lesson_path(&paths, &slug).join("lesson.ipynb");
        // learner edits the stub
        let mut nb: Value =
            serde_json::from_str(&std::fs::read_to_string(&nb_path).unwrap()).unwrap();
        nb["cells"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .for_each(|c| {
                if c.get("metadata").and_then(|m| m.get("managed")) == Some(&json!("practice-stub"))
                {
                    c["source"] = json!("def sum_array(arr):\n    return sum(arr)\n");
                }
            });
        std::fs::write(&nb_path, nb.to_string()).unwrap();
        // agent changes the DB signature (scaffold changes) under the edited stub
        let conn = db::open_course(&paths, &slug).unwrap();
        conn.execute(
            "UPDATE practice SET signature='def sum_array(a, b):' WHERE id='p1'",
            [],
        )
        .unwrap();

        let Data::LessonSync { conflicts, .. } =
            sync(&paths, &slug, "arrays-101", false).expect("sync")
        else {
            panic!();
        };
        assert_eq!(conflicts.len(), 1, "{conflicts:?}");
        assert_eq!(conflicts[0].reason, "db_changed");
        assert_eq!(conflicts[0].id, "p1");
        // without --force the learner source is left intact
        let after: Value =
            serde_json::from_str(&std::fs::read_to_string(&nb_path).unwrap()).unwrap();
        let kept =
            after["cells"].as_array().unwrap().iter().any(|c| {
                c["source"].as_str() == Some("def sum_array(arr):\n    return sum(arr)\n")
            });
        assert!(kept, "learner source should be left intact without --force");
    }

    fn read_nb(nb_path: &std::path::Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(nb_path).unwrap()).unwrap()
    }

    fn managed_cells<'a>(nb: &'a Value, kind: &str) -> Vec<(usize, &'a Value)> {
        nb["cells"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .filter(|(_, c)| c.get("metadata").and_then(|m| m.get("managed")) == Some(&json!(kind)))
            .collect()
    }

    #[test]
    fn create_renders_question_md_before_prompted_stubs_only() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let nb_path = lesson_path(&paths, &slug).join("lesson.ipynb");
        let nb = read_nb(&nb_path);
        // SPEC's practice has a prompt; its quiz does not.
        let questions = managed_cells(&nb, "question-md");
        assert_eq!(questions.len(), 1, "{questions:?}");
        let (qi, q) = questions[0];
        assert_eq!(q["metadata"]["practice_id"], json!("p1"));
        assert_eq!(q["metadata"]["lesson_id"], json!("arrays-101"));
        assert_eq!(q["cell_type"], json!("markdown"));
        assert_eq!(q["source"], json!("sum it\n"));
        let stubs = managed_cells(&nb, "practice-stub");
        assert_eq!(stubs.len(), 1);
        assert_eq!(stubs[0].0, qi + 1, "question-md must precede its stub");
        assert!(managed_cells(&nb, "quiz-stub").iter().all(|(i, _)| {
            nb["cells"][i - 1]
                .get("metadata")
                .and_then(|m| m.get("managed"))
                != Some(&json!("question-md"))
        }));
    }

    #[test]
    fn sync_regenerates_question_md_and_reanchors_learner_cells() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, SPEC).unwrap();
        let nb_path = lesson_path(&paths, &slug).join("lesson.ipynb");
        // learner drops a note right after the question cell (and defaces it —
        // managed cells regenerate wholesale regardless)
        let mut nb = read_nb(&nb_path);
        let (qi, _) = managed_cells(&nb, "question-md").remove(0);
        nb["cells"].as_array_mut().unwrap().insert(
            qi + 1,
            json!({"cell_type": "markdown", "metadata": {}, "source": "my note\n"}),
        );
        nb["cells"][qi]["source"] = json!("defaced\n");
        std::fs::write(&nb_path, nb.to_string()).unwrap();

        let Data::LessonSync { conflicts, .. } =
            sync(&paths, &slug, "arrays-101", false).expect("sync")
        else {
            panic!();
        };
        assert!(conflicts.is_empty(), "{conflicts:?}");
        let after = read_nb(&nb_path);
        let mut questions = managed_cells(&after, "question-md");
        assert_eq!(questions.len(), 1);
        let (qi, q) = questions.remove(0);
        assert_eq!(
            q["source"],
            json!("sum it\n"),
            "question regenerated from DB"
        );
        // learner cell re-anchored directly after the regenerated question cell
        assert_eq!(after["cells"][qi + 1]["source"], json!("my note\n"));
    }
}
