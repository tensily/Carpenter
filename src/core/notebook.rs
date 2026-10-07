//! Render a lesson notebook (DB → `.ipynb`) and define `scaffold_hash`.
//!
//! Render order: skip-config → title → section cells (snippets, then question
//! markdown + practice stubs + checks) → question markdown + quiz stubs +
//! checks. Every managed cell is tagged with
//! `metadata.managed` (+ id siblings); untagged learner cells are preserved by
//! sync (P5). `scaffold_hash` is the FNV-1a of the canonical scaffold string and
//! lives only in cell metadata (never the DB).

use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::core::db;
use crate::core::error::CarpenterError;
use crate::models::lesson::SnippetOut;
use crate::models::LessonConflict;

/// The canonical scaffold string for a stub (`signature` + prompt comment +
/// `raise NotImplementedError`).
pub fn canonical_scaffold(signature: &str, prompt: &str) -> String {
    if prompt.trim().is_empty() {
        format!("{signature}\n    raise NotImplementedError\n")
    } else {
        format!("{signature}\n    # {prompt}\n    raise NotImplementedError\n")
    }
}

/// `scaffold_hash` for a stub: FNV-1a (64-bit, hex) of its canonical scaffold.
pub fn scaffold_hash(signature: &str, prompt: &str) -> String {
    stable_hash(&canonical_scaffold(signature, prompt))
}

/// Stable FNV-1a 64-bit hash → 16-char hex (stable across versions/platforms).
pub fn stable_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// The skip-config cell source (delegates to `helper.is_skipped`).
pub fn skip_config_source() -> String {
    String::from(
        "import helper\n\
         def is_skipped(target):\n\
         \x20\x20\x20\x20\"\"\"Read skip flags from course.db (managed cell — do not edit).\"\"\"\n\
         \x20\x20\x20\x20return helper.is_skipped(target)\n",
    )
}

/// The check-cell source: `helper.check("<type>", "<id>", <name>)`.
pub fn check_source(owner_type: &str, owner_id: &str, fn_name: &str) -> String {
    format!("import helper\nhelper.check(\"{owner_type}\", \"{owner_id}\", {fn_name})\n")
}

/// The question-cell source: the full prompt verbatim (markdown allowed).
pub fn question_source(prompt: &str) -> String {
    format!("{}\n", prompt.trim_end())
}

fn md_cell(source: &str, managed: Value) -> Value {
    json!({"cell_type": "markdown", "metadata": managed, "source": source})
}

fn code_cell(source: &str, managed: Value) -> Value {
    json!({
        "cell_type": "code",
        "metadata": managed,
        "source": source,
        "execution_count": null,
        "outputs": []
    })
}

/// Parse a stored snippets JSON (`[{id,kind,content}]`) into output snippets.
pub fn parse_snippets(snippets_json: &str) -> Vec<SnippetOut> {
    #[derive(Deserialize)]
    struct S {
        id: String,
        kind: String,
        content: String,
    }
    let parsed: Vec<S> = serde_json::from_str(snippets_json).unwrap_or_default();
    parsed
        .into_iter()
        .map(|s| SnippetOut {
            id: s.id,
            kind: s.kind,
            content: s.content,
        })
        .collect()
}

/// Render the full notebook for a lesson as pretty-printed JSON.
pub fn render_to_string(conn: &Connection, lesson_id: &str) -> Result<String, CarpenterError> {
    let nb = render_notebook(conn, lesson_id)?;
    serde_json::to_string_pretty(&nb)
        .map_err(|e| CarpenterError::StoreError(format!("notebook encode failed: {e}")))
}

/// Build the nbformat v4 JSON for a lesson from the DB.
pub fn render_notebook(conn: &Connection, lesson_id: &str) -> Result<Value, CarpenterError> {
    let lesson = db::get_lesson(conn, lesson_id)?;
    let sections = db::list_sections(conn, lesson_id)?;
    let mut cells: Vec<Value> = Vec::new();

    cells.push(code_cell(
        &skip_config_source(),
        json!({"managed": "skip-config", "lesson_id": lesson_id}),
    ));
    cells.push(md_cell(
        &format!("# {}\n", lesson.title),
        json!({"managed": "title", "lesson_id": lesson_id}),
    ));

    for sec in &sections {
        for sn in parse_snippets(&sec.snippets) {
            let managed = json!({
                "managed": if sn.kind == "code" { "section-code" } else { "section-md" },
                "section_id": sec.id,
                "snippet_id": sn.id,
            });
            if sn.kind == "code" {
                cells.push(code_cell(&sn.content, managed));
            } else {
                cells.push(md_cell(&sn.content, managed));
            }
        }
        for p in db::list_practice(conn, &sec.id)? {
            if !p.prompt.trim().is_empty() {
                cells.push(md_cell(
                    &question_source(&p.prompt),
                    json!({"managed": "question-md", "practice_id": p.id, "lesson_id": lesson_id}),
                ));
            }
            let scaf = canonical_scaffold(&p.signature, &p.prompt);
            let hash = stable_hash(&scaf);
            cells.push(code_cell(
                &scaf,
                json!({"managed": "practice-stub", "practice_id": p.id, "scaffold_hash": hash}),
            ));
            cells.push(code_cell(
                &check_source("practice", &p.id, &p.name),
                json!({"managed": "check", "target": format!("practice:{}", p.id)}),
            ));
        }
    }

    for q in db::list_quizzes(conn, lesson_id)? {
        if !q.prompt.trim().is_empty() {
            cells.push(md_cell(
                &question_source(&q.prompt),
                json!({"managed": "question-md", "quiz_id": q.id, "lesson_id": lesson_id}),
            ));
        }
        let scaf = canonical_scaffold(&q.signature, &q.prompt);
        let hash = stable_hash(&scaf);
        cells.push(code_cell(
            &scaf,
            json!({"managed": "quiz-stub", "quiz_id": q.id, "scaffold_hash": hash}),
        ));
        cells.push(code_cell(
            &check_source("quiz", &q.id, &q.name),
            json!({"managed": "check", "target": format!("quiz:{}", q.id)}),
        ));
    }

    Ok(json!({
        "cells": cells,
        "metadata": {
            "kernelspec": {"display_name": "Python 3", "language": "python", "name": "python3"},
            "language_info": {"name": "python"}
        },
        "nbformat": 4,
        "nbformat_minor": 5
    }))
}

// ---- sync (3-way stub preservation) ----

fn managed_type(cell: &Value) -> Option<String> {
    cell.get("metadata")?
        .get("managed")?
        .as_str()
        .map(String::from)
}

fn meta_str(cell: &Value, key: &str) -> Option<String> {
    cell.get("metadata")?.get(key)?.as_str().map(String::from)
}

/// A stable identity key for a managed cell (anchors learner cells on sync).
pub fn managed_key(cell: &Value, mtype: &str) -> String {
    match mtype {
        "section-md" | "section-code" => format!(
            "{mtype}:{}:{}",
            meta_str(cell, "section_id").unwrap_or_default(),
            meta_str(cell, "snippet_id").unwrap_or_default()
        ),
        "practice-stub" => format!(
            "practice-stub:{}",
            meta_str(cell, "practice_id").unwrap_or_default()
        ),
        "quiz-stub" => format!(
            "quiz-stub:{}",
            meta_str(cell, "quiz_id").unwrap_or_default()
        ),
        "check" => format!("check:{}", meta_str(cell, "target").unwrap_or_default()),
        "question-md" => format!(
            "question-md:{}:{}",
            meta_str(cell, "practice_id").unwrap_or_default(),
            meta_str(cell, "quiz_id").unwrap_or_default()
        ),
        other => other.to_string(),
    }
}

fn cell_source(cell: &Value) -> String {
    match cell.get("source") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str()).collect(),
        _ => String::new(),
    }
}

fn cell_scaffold_hash(cell: &Value) -> String {
    meta_str(cell, "scaffold_hash").unwrap_or_default()
}

/// Sync an existing notebook against the DB (3-way stub preservation).
///
/// Returns the new notebook JSON + per-stub conflicts. Managed non-stub cells are
/// regenerated wholesale; untagged learner cells are preserved (re-anchored after
/// their preceding managed cell). `force` overwrites conflicting stubs.
pub fn sync_notebook(
    old: &Value,
    conn: &Connection,
    lesson_id: &str,
    force: bool,
) -> Result<(Value, Vec<LessonConflict>), CarpenterError> {
    let fresh = render_notebook(conn, lesson_id)?;
    let fresh_cells = fresh["cells"].as_array().cloned().unwrap_or_default();
    let old_cells = old["cells"].as_array().cloned().unwrap_or_default();

    let mut learner_after: HashMap<Option<String>, Vec<Value>> = HashMap::new();
    let mut old_managed: HashMap<String, Value> = HashMap::new();
    let mut last_key: Option<String> = None;
    for cell in &old_cells {
        match managed_type(cell) {
            Some(mtype) => {
                let key = managed_key(cell, &mtype);
                old_managed.insert(key.clone(), cell.clone());
                last_key = Some(key);
            }
            None => {
                learner_after
                    .entry(last_key.clone())
                    .or_default()
                    .push(cell.clone());
            }
        }
    }

    let mut conflicts = Vec::new();
    let mut out_cells: Vec<Value> = Vec::new();
    if let Some(pre) = learner_after.remove(&None) {
        out_cells.extend(pre);
    }
    for fcell in &fresh_cells {
        let ftype = managed_type(fcell).unwrap_or_default();
        let fkey = managed_key(fcell, &ftype);
        let final_cell = if ftype == "practice-stub" || ftype == "quiz-stub" {
            let stub_id = if ftype == "practice-stub" {
                meta_str(fcell, "practice_id").unwrap_or_default()
            } else {
                meta_str(fcell, "quiz_id").unwrap_or_default()
            };
            match old_managed.get(&fkey) {
                Some(old_cell) => {
                    let old_source = cell_source(old_cell);
                    let old_hash = cell_scaffold_hash(old_cell);
                    let fresh_hash = cell_scaffold_hash(fcell); // canonical_now
                    let learner_touched = stable_hash(&old_source) != old_hash;
                    let db_changed = fresh_hash != old_hash;
                    if !learner_touched {
                        fcell.clone() // untouched → refresh from DB
                    } else if !db_changed {
                        old_cell.clone() // learner edited, DB same → keep verbatim
                    } else {
                        // learner edited AND DB changed → conflict
                        conflicts.push(LessonConflict {
                            id: stub_id,
                            reason: String::from("db_changed"),
                        });
                        if force {
                            fcell.clone()
                        } else {
                            old_cell.clone()
                        }
                    }
                }
                None => fcell.clone(), // new stub
            }
        } else {
            fcell.clone() // title/skip-config/section/check → regenerated wholesale
        };
        out_cells.push(final_cell);
        if let Some(learners) = learner_after.remove(&Some(fkey)) {
            out_cells.extend(learners);
        }
    }
    // orphan learner cells (anchored to a now-removed managed cell) → preserve at end
    for v in learner_after.into_values() {
        out_cells.extend(v);
    }

    let mut new_nb = fresh;
    new_nb["cells"] = Value::Array(out_cells);
    Ok((new_nb, conflicts))
}

/// A cell-execution error captured from nbconvert outputs.
#[derive(Debug, Clone)]
pub struct NbError {
    /// cell index.
    pub index: usize,
    /// exception name.
    pub ename: String,
    /// exception value.
    pub evalue: String,
}

/// Scan a notebook for code-cell error outputs (nbconvert `--allow_errors` writes
/// them as `output_type:"error"`).
pub fn scan_errors(nb: &Value) -> Vec<NbError> {
    let mut out = Vec::new();
    let Some(cells) = nb["cells"].as_array() else {
        return out;
    };
    for (i, cell) in cells.iter().enumerate() {
        if cell.get("cell_type").and_then(|v| v.as_str()) != Some("code") {
            continue;
        }
        let Some(outputs) = cell.get("outputs").and_then(|o| o.as_array()) else {
            continue;
        };
        for o in outputs {
            if o.get("output_type").and_then(|v| v.as_str()) == Some("error") {
                out.push(NbError {
                    index: i,
                    ename: o
                        .get("ename")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    evalue: o
                        .get("evalue")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
                break;
            }
        }
    }
    out
}

/// If the cell is a managed stub, return `(owner_type, id, scaffold_hash)`.
pub fn stub_info(cell: &Value) -> Option<(&'static str, String, String)> {
    let m = managed_type(cell)?;
    match m.as_str() {
        "practice-stub" => Some((
            "practice",
            meta_str(cell, "practice_id")?,
            cell_scaffold_hash(cell),
        )),
        "quiz-stub" => Some(("quiz", meta_str(cell, "quiz_id")?, cell_scaffold_hash(cell))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_and_hash_are_deterministic() {
        let a = canonical_scaffold("def f(x):", "return x");
        let b = canonical_scaffold("def f(x):", "return x");
        assert_eq!(a, b);
        assert!(a.contains("raise NotImplementedError"));
        assert_eq!(scaffold_hash("def f(x):", "return x"), stable_hash(&a));
        // empty prompt ⇒ no comment line
        let e = canonical_scaffold("def g():", "");
        assert!(!e.contains('#'));
        assert!(e.contains("raise NotImplementedError"));
    }

    #[test]
    fn stable_hash_is_hex_and_stable() {
        let h = stable_hash("def f(x):\n    raise NotImplementedError\n");
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(h, stable_hash("def f(x):\n    raise NotImplementedError\n"));
    }

    #[test]
    fn check_source_emits_helper_call() {
        let s = check_source("practice", "p1", "sum_array");
        assert!(
            s.contains(r#"helper.check("practice", "p1", sum_array)"#),
            "{s}"
        );
    }

    #[test]
    fn question_source_is_prompt_with_single_trailing_newline() {
        assert_eq!(question_source("Why?\n"), "Why?\n");
        assert_eq!(question_source("a\nb"), "a\nb\n");
        assert_eq!(question_source(""), "\n");
    }

    #[test]
    fn managed_key_distinguishes_question_cells() {
        let p1 = json!({"metadata": {"managed": "question-md", "practice_id": "p1"}});
        let p2 = json!({"metadata": {"managed": "question-md", "practice_id": "p2"}});
        let q1 = json!({"metadata": {"managed": "question-md", "quiz_id": "p1"}});
        let kp1 = managed_key(&p1, "question-md");
        assert_eq!(kp1, "question-md:p1:");
        assert_eq!(managed_key(&q1, "question-md"), "question-md::p1");
        assert_ne!(kp1, managed_key(&p2, "question-md"));
        assert_ne!(kp1, managed_key(&q1, "question-md"));
    }
}
