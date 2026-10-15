//! SQLite repository: the sole SQL surface.
//!
//! Opens `course.db`, applies the schema (one idempotent `CREATE` batch), enables
//! FK enforcement, and exposes typed accessors. Commands never write raw SQL —
//! they call here. Schema lives in code; `docs/data-model/03-ddl.md` is the
//! human-readable mirror (code wins on conflict).

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::core::error::CarpenterError;
use crate::core::reconcile;
use crate::core::store::Paths;
use crate::models::{CourseCounts, CourseRow, GoalRow, PlanRow};

/// The full initial schema (`docs/data-model/03-ddl.md`) plus the internal
/// `id_seq` counter that backs [`next_id`]'s never-reuse guarantee. All
/// `CREATE … IF NOT EXISTS`, so opening an existing db is a no-op.
pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS course_meta (
  slug        TEXT PRIMARY KEY,
  title       TEXT NOT NULL,
  goal        TEXT NOT NULL DEFAULT '',
  description TEXT NOT NULL DEFAULT '',
  created_at  TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS lessons (
  id         TEXT PRIMARY KEY,
  slug       TEXT NOT NULL UNIQUE,
  title      TEXT NOT NULL,
  ord        INTEGER NOT NULL,
  status     TEXT NOT NULL DEFAULT 'not_started'
             CHECK (status IN ('not_started','in_progress','complete','skipped')),
  skip       INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sections (
  id        TEXT PRIMARY KEY,
  lesson_id TEXT NOT NULL REFERENCES lessons(id) ON DELETE CASCADE,
  title     TEXT NOT NULL,
  snippets  TEXT NOT NULL DEFAULT '[]',
  ord       INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS practice (
  id           TEXT PRIMARY KEY,
  section_id   TEXT NOT NULL REFERENCES sections(id) ON DELETE CASCADE,
  name         TEXT NOT NULL,
  signature    TEXT NOT NULL,
  prompt       TEXT NOT NULL DEFAULT '',
  solution     TEXT NOT NULL DEFAULT '',
  ord          INTEGER NOT NULL,
  pass_or_fail INTEGER NOT NULL DEFAULT 0,
  last_check   TEXT NOT NULL DEFAULT '{}',
  skip         INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS quizzes (
  id           TEXT PRIMARY KEY,
  lesson_id    TEXT NOT NULL REFERENCES lessons(id) ON DELETE CASCADE,
  name         TEXT NOT NULL,
  signature    TEXT NOT NULL,
  prompt       TEXT NOT NULL DEFAULT '',
  solution     TEXT NOT NULL DEFAULT '',
  ord          INTEGER NOT NULL,
  pass_or_fail INTEGER NOT NULL DEFAULT 0,
  last_check   TEXT NOT NULL DEFAULT '{}',
  skip         INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS test_cases (
  id         TEXT PRIMARY KEY,
  owner_type TEXT NOT NULL CHECK (owner_type IN ('practice','quiz')),
  owner_id   TEXT NOT NULL,
  args       TEXT NOT NULL DEFAULT '[]',
  kwargs     TEXT NOT NULL DEFAULT '{}',
  expected   TEXT NOT NULL,
  compare    TEXT NOT NULL DEFAULT 'exact'
             CHECK (compare IN ('exact','sorted','set')),
  ord        INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_cases_owner ON test_cases(owner_type, owner_id);
CREATE TABLE IF NOT EXISTS notes (
  id         TEXT PRIMARY KEY,
  ts         TEXT NOT NULL,
  updated_ts TEXT NOT NULL,
  kind       TEXT NOT NULL
             CHECK (kind IN ('gap','mistake','strength','pattern','progress')),
  tags       TEXT NOT NULL DEFAULT '[]',
  status     TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','resolved')),
  recurrence TEXT NOT NULL DEFAULT 'new'  CHECK (recurrence IN ('new','recurring')),
  related    TEXT NOT NULL DEFAULT '',
  text       TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS plans (
  id           TEXT PRIMARY KEY,
  scope        TEXT NOT NULL CHECK (scope IN ('course','lesson')),
  scope_id     TEXT NOT NULL,
  title        TEXT NOT NULL,
  content      TEXT NOT NULL,
  created_at   TEXT NOT NULL,
  confirmed_at TEXT
);
CREATE TABLE IF NOT EXISTS goals (
  id         TEXT PRIMARY KEY,
  scope      TEXT NOT NULL DEFAULT 'course' CHECK (scope IN ('course')),
  scope_id   TEXT NOT NULL,
  text       TEXT NOT NULL,
  status     TEXT NOT NULL DEFAULT 'pending'
             CHECK (status IN ('pending','achieved','skipped')),
  covered_by TEXT NOT NULL DEFAULT '[]',
  override   INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_goals_scope ON goals(scope, scope_id);
CREATE TABLE IF NOT EXISTS id_seq (
  tbl  TEXT PRIMARY KEY,
  next INTEGER NOT NULL
);
"#;

/// Open (or create) a course database, apply the schema, enable FK enforcement.
pub fn open(path: &Path) -> Result<Connection, CarpenterError> {
    let conn = Connection::open(path).map_err(store_msg)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(store_msg)?;
    conn.execute_batch(SCHEMA).map_err(store_msg)?;
    migrate(&conn)?;
    Ok(conn)
}

/// Does `table` have a column named `col`? (Used to gate idempotent migrations —
/// SQLite has no `ADD COLUMN IF NOT EXISTS`.)
fn has_column(conn: &Connection, table: &str, col: &str) -> Result<bool, CarpenterError> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(store_msg)?;
    let names: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(store_msg)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(store_msg)?;
    Ok(names.iter().any(|n| n == col))
}

/// Idempotent schema migrations for DBs created before a column existed. Each
/// step guards on [`has_column`] so it is a no-op on current-schema DBs. Add a
/// new step per schema change; never edit `SCHEMA` retroactively alone.
fn migrate(conn: &Connection) -> Result<(), CarpenterError> {
    // solution TEXT NOT NULL DEFAULT '' on practice + quizzes (adr/015).
    for table in ["practice", "quizzes"] {
        if !has_column(conn, table, "solution")? {
            conn.execute(
                &format!("ALTER TABLE {table} ADD COLUMN solution TEXT NOT NULL DEFAULT ''"),
                [],
            )
            .map_err(store_msg)?;
        }
    }
    // ord uniqueness (adr/017): legacy DBs can hold duplicate ords (racy
    // max(ord)+1 read-then-insert). Resequence densely (stable by
    // ord, created_at, id), then enforce the invariant with a unique index so
    // any future duplicate fails loudly instead of silently.
    let dupes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM (SELECT ord FROM lessons GROUP BY ord HAVING COUNT(*) > 1)",
            [],
            |r| r.get(0),
        )
        .map_err(store_msg)?;
    if dupes > 0 {
        conn.execute(
            "WITH ranked AS \
               (SELECT id, ROW_NUMBER() OVER (ORDER BY ord, created_at, id) AS rn FROM lessons) \
             UPDATE lessons SET ord = (SELECT rn FROM ranked WHERE ranked.id = lessons.id)",
            [],
        )
        .map_err(store_msg)?;
    }
    conn.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS idx_lessons_ord ON lessons(ord);")
        .map_err(store_msg)?;
    Ok(())
}

/// Insert a course_meta row.
pub fn insert_course_meta(conn: &Connection, row: &CourseRow) -> Result<(), CarpenterError> {
    conn.execute(
        "INSERT INTO course_meta (slug,title,goal,description,created_at) VALUES (?1,?2,?3,?4,?5)",
        params![
            row.slug,
            row.title,
            row.goal,
            row.description,
            row.created_at
        ],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Read a course_meta row by slug. [`CarpenterError::NotFound`] if absent.
pub fn get_course_meta(conn: &Connection, slug: &str) -> Result<CourseRow, CarpenterError> {
    conn.query_row(
        "SELECT slug,title,goal,description,created_at FROM course_meta WHERE slug=?1",
        params![slug],
        |r| {
            Ok(CourseRow {
                slug: r.get(0)?,
                title: r.get(1)?,
                goal: r.get(2)?,
                description: r.get(3)?,
                created_at: r.get(4)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => CarpenterError::NotFound(format!("course {slug}")),
        e => store_msg(e),
    })
}

/// Update the mutable course_meta fields (title, goal, description).
pub fn update_course_meta(
    conn: &Connection,
    slug: &str,
    title: &str,
    goal: &str,
    description: &str,
) -> Result<(), CarpenterError> {
    conn.execute(
        "UPDATE course_meta SET title=?2, goal=?3, description=?4 WHERE slug=?1",
        params![slug, title, goal, description],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Count rows per top-level table for a course.
pub fn course_counts(conn: &Connection) -> Result<CourseCounts, CarpenterError> {
    let count = |table: &str| -> Result<i64, CarpenterError> {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(store_msg)
    };
    Ok(CourseCounts {
        lessons: count("lessons")?,
        sections: count("sections")?,
        practice: count("practice")?,
        quizzes: count("quizzes")?,
    })
}

/// Number of lessons in a course.
pub fn lessons_count(conn: &Connection) -> Result<i64, CarpenterError> {
    conn.query_row("SELECT COUNT(*) FROM lessons", [], |r| r.get::<_, i64>(0))
        .map_err(store_msg)
}

/// Allocate the next id for a prefixed table (e.g. `("sections","s")` -> `"s1"`).
///
/// Uses the `id_seq` counter, so deleting rows never causes an id to be reused.
/// `table` and `prefix` are internal constants (never user input).
pub fn next_id(conn: &Connection, table: &str, prefix: &str) -> Result<String, CarpenterError> {
    let n: i64 = conn
        .query_row(
            "INSERT INTO id_seq (tbl, next) VALUES (?1, 1)
             ON CONFLICT(tbl) DO UPDATE SET next = next + 1
             RETURNING next",
            params![table],
            |r| r.get(0),
        )
        .map_err(store_msg)?;
    Ok(format!("{prefix}{n}"))
}

fn store_msg(e: rusqlite::Error) -> CarpenterError {
    CarpenterError::StoreError(e.to_string())
}

/// Inputs needed to derive a lesson's status.
#[derive(Debug, Clone, Copy)]
pub struct LessonStatusInputs {
    /// whole-lesson skip flag.
    pub skip: bool,
    /// count of non-skipped practice+quiz items.
    pub total_items: i64,
    /// count of those with `pass_or_fail=1`.
    pub passing_items: i64,
}

/// Open the active course's database. [`CarpenterError::NotFound`] if absent.
pub fn open_course(paths: &Paths, slug: &str) -> Result<Connection, CarpenterError> {
    let p = paths.course(slug).join("course.db");
    if !p.exists() {
        return Err(CarpenterError::NotFound(format!("course {slug}")));
    }
    open(&p)
}

/// Does a lesson with this id exist?
pub fn lesson_exists(conn: &Connection, lesson_id: &str) -> Result<bool, CarpenterError> {
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM lessons WHERE id=?1",
            params![lesson_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(store_msg)?;
    Ok(id.is_some())
}

/// Gather the inputs for lesson status derivation; `None` if the lesson is absent.
pub fn lesson_status_inputs(
    conn: &Connection,
    lesson_id: &str,
) -> Result<Option<LessonStatusInputs>, CarpenterError> {
    let skip: Option<i64> = conn
        .query_row(
            "SELECT skip FROM lessons WHERE id=?1",
            params![lesson_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(store_msg)?;
    let Some(skip) = skip else {
        return Ok(None);
    };
    let total: i64 = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM practice p JOIN sections s ON p.section_id=s.id WHERE s.lesson_id=?1 AND p.skip=0)
                  + (SELECT COUNT(*) FROM quizzes WHERE lesson_id=?1 AND skip=0)",
            params![lesson_id],
            |r| r.get(0),
        )
        .map_err(store_msg)?;
    let passing: i64 = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM practice p JOIN sections s ON p.section_id=s.id WHERE s.lesson_id=?1 AND p.skip=0 AND p.pass_or_fail=1)
                  + (SELECT COUNT(*) FROM quizzes WHERE lesson_id=?1 AND skip=0 AND pass_or_fail=1)",
            params![lesson_id],
            |r| r.get(0),
        )
        .map_err(store_msg)?;
    Ok(Some(LessonStatusInputs {
        skip: skip != 0,
        total_items: total,
        passing_items: passing,
    }))
}

// ---- plans ----

fn plan_row(r: &rusqlite::Row) -> rusqlite::Result<PlanRow> {
    Ok(PlanRow {
        id: r.get(0)?,
        scope: r.get(1)?,
        scope_id: r.get(2)?,
        title: r.get(3)?,
        content: r.get(4)?,
        created_at: r.get(5)?,
        confirmed_at: r.get(6)?,
    })
}

const PLAN_COLS: &str = "id,scope,scope_id,title,content,created_at,confirmed_at FROM plans";

/// Insert a plan row.
pub fn insert_plan(conn: &Connection, row: &PlanRow) -> Result<(), CarpenterError> {
    conn.execute(
        "INSERT INTO plans (id,scope,scope_id,title,content,created_at,confirmed_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![row.id, row.scope, row.scope_id, row.title, row.content, row.created_at, row.confirmed_at],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Read a plan by id; [`CarpenterError::NotFound`] if absent.
pub fn get_plan(conn: &Connection, id: &str) -> Result<PlanRow, CarpenterError> {
    conn.query_row(
        &format!("SELECT {PLAN_COLS} WHERE id=?1"),
        params![id],
        plan_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => CarpenterError::NotFound(format!("plan {id}")),
        e => store_msg(e),
    })
}

/// List plans, optionally filtered by scope.
pub fn list_plans(conn: &Connection, scope: Option<&str>) -> Result<Vec<PlanRow>, CarpenterError> {
    let rows = match scope {
        Some(s) => conn
            .prepare(&format!("SELECT {PLAN_COLS} WHERE scope=?1 ORDER BY id"))
            .map_err(store_msg)?
            .query_map(params![s], plan_row)
            .map_err(store_msg)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(store_msg)?,
        None => conn
            .prepare(&format!("SELECT {PLAN_COLS} ORDER BY id"))
            .map_err(store_msg)?
            .query_map([], plan_row)
            .map_err(store_msg)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(store_msg)?,
    };
    Ok(rows)
}

/// Mark a plan confirmed (sets `confirmed_at`).
pub fn set_plan_confirmed(
    conn: &Connection,
    id: &str,
    confirmed_at: &str,
) -> Result<(), CarpenterError> {
    conn.execute(
        "UPDATE plans SET confirmed_at=?2 WHERE id=?1",
        params![id, confirmed_at],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Replace a plan's title + content (for `update`).
pub fn replace_plan(
    conn: &Connection,
    id: &str,
    title: &str,
    content: &str,
) -> Result<(), CarpenterError> {
    conn.execute(
        "UPDATE plans SET title=?2, content=?3 WHERE id=?1",
        params![id, title, content],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Delete a plan.
pub fn delete_plan(conn: &Connection, id: &str) -> Result<(), CarpenterError> {
    conn.execute("DELETE FROM plans WHERE id=?1", params![id])
        .map_err(store_msg)?;
    Ok(())
}

// ---- goals ----

fn goal_row(r: &rusqlite::Row) -> rusqlite::Result<GoalRow> {
    let covered_json: String = r.get(5)?;
    let covered_by: Vec<String> = serde_json::from_str(&covered_json).unwrap_or_default();
    let ov: i64 = r.get(6)?;
    Ok(GoalRow {
        id: r.get(0)?,
        scope: r.get(1)?,
        scope_id: r.get(2)?,
        text: r.get(3)?,
        status: r.get(4)?,
        covered_by,
        override_flag: ov != 0,
        created_at: r.get(7)?,
    })
}

const GOAL_COLS: &str = "id,scope,scope_id,text,status,covered_by,override,created_at FROM goals";

/// Insert a goal row.
pub fn insert_goal(conn: &Connection, row: &GoalRow) -> Result<(), CarpenterError> {
    let covered = serde_json::to_string(&row.covered_by).unwrap_or_else(|_| String::from("[]"));
    let ov: i64 = if row.override_flag { 1 } else { 0 };
    conn.execute(
        "INSERT INTO goals (id,scope,scope_id,text,status,covered_by,override,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![row.id, row.scope, row.scope_id, row.text, row.status, covered, ov, row.created_at],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Read a goal by id; [`CarpenterError::NotFound`] if absent.
pub fn get_goal(conn: &Connection, id: &str) -> Result<GoalRow, CarpenterError> {
    conn.query_row(
        &format!("SELECT {GOAL_COLS} WHERE id=?1"),
        params![id],
        goal_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => CarpenterError::NotFound(format!("goal {id}")),
        e => store_msg(e),
    })
}

/// List all goals for a course.
pub fn list_goals(conn: &Connection) -> Result<Vec<GoalRow>, CarpenterError> {
    let rows = conn
        .prepare(&format!("SELECT {GOAL_COLS} ORDER BY id"))
        .map_err(store_msg)?
        .query_map([], goal_row)
        .map_err(store_msg)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(store_msg)?;
    Ok(rows)
}

/// Update a goal's status, override flag, and covered_by.
pub fn update_goal(
    conn: &Connection,
    id: &str,
    status: &str,
    override_flag: bool,
    covered_by: &[String],
) -> Result<(), CarpenterError> {
    let covered = serde_json::to_string(covered_by).unwrap_or_else(|_| String::from("[]"));
    let ov: i64 = if override_flag { 1 } else { 0 };
    conn.execute(
        "UPDATE goals SET status=?2, override=?3, covered_by=?4 WHERE id=?1",
        params![id, status, ov, covered],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Delete a goal.
pub fn delete_goal(conn: &Connection, id: &str) -> Result<(), CarpenterError> {
    conn.execute("DELETE FROM goals WHERE id=?1", params![id])
        .map_err(store_msg)?;
    Ok(())
}

// ---- lessons / sections / practice / quizzes / test_cases ----

/// A lessons row (read fields).
#[derive(Debug, Clone)]
pub struct LessonDb {
    /// id (slug).
    pub id: String,
    /// slug.
    pub slug: String,
    /// title.
    pub title: String,
    /// order.
    pub ord: i64,
    /// status.
    pub status: String,
    /// skip flag.
    pub skip: bool,
    /// created_at.
    pub created_at: String,
    /// updated_at.
    pub updated_at: String,
}

/// A sections row.
#[derive(Debug, Clone)]
pub struct SectionDb {
    /// id (`s1`…).
    pub id: String,
    /// title.
    pub title: String,
    /// snippets JSON (`[{id,kind,content}]`).
    pub snippets: String,
    /// order.
    pub ord: i64,
}

/// A practice/quizzes row.
#[derive(Debug, Clone)]
pub struct CheckableDb {
    /// id.
    pub id: String,
    /// function name.
    pub name: String,
    /// signature.
    pub signature: String,
    /// prompt.
    pub prompt: String,
    /// author reference solution (empty = none). Author-only; never rendered.
    pub solution: String,
    /// order.
    pub ord: i64,
    /// skip flag.
    pub skip: bool,
    /// last-check pass flag.
    pub pass_or_fail: bool,
    /// last-check JSON (`{passed,total,cases:[…]}`).
    pub last_check: String,
}

/// A test_cases row.
#[derive(Debug, Clone)]
pub struct CaseDb {
    /// id (`c1`…).
    pub id: String,
    /// args JSON.
    pub args: String,
    /// kwargs JSON.
    pub kwargs: String,
    /// expected JSON.
    pub expected: String,
    /// compare mode.
    pub compare: String,
    /// order.
    pub ord: i64,
}

/// Map a lessons-insert failure to [`CarpenterError::Conflict`] when it hits the
/// `ord` uniqueness index, else a plain store error.
fn lesson_insert_err(e: rusqlite::Error, ord: i64) -> CarpenterError {
    if let rusqlite::Error::SqliteFailure(_, Some(msg)) = &e {
        if msg.contains("lessons.ord") {
            return CarpenterError::Conflict(format!("lesson ord {ord} already taken"));
        }
    }
    store_msg(e)
}

/// Insert a lessons row with an explicit `ord` (status `not_started`, skip 0).
///
/// [`CarpenterError::Conflict`] if `ord` is already taken (adr/017).
pub fn insert_lesson(
    conn: &Connection,
    id: &str,
    slug: &str,
    title: &str,
    ord: i64,
    created_at: &str,
    updated_at: &str,
) -> Result<(), CarpenterError> {
    conn.execute(
        "INSERT INTO lessons (id,slug,title,ord,status,skip,created_at,updated_at) \
         VALUES (?1,?2,?3,?4,'not_started',0,?5,?6)",
        params![id, slug, title, ord, created_at, updated_at],
    )
    .map_err(|e| lesson_insert_err(e, ord))?;
    Ok(())
}

/// Insert a lessons row with an atomically allocated `ord` and return it.
///
/// Allocation (`max(ord)+1`) happens inside the same statement as the insert,
/// so concurrent creators cannot observe the same `max` snapshot (adr/017).
pub fn insert_lesson_auto_ord(
    conn: &Connection,
    id: &str,
    slug: &str,
    title: &str,
    created_at: &str,
    updated_at: &str,
) -> Result<i64, CarpenterError> {
    conn.query_row(
        "INSERT INTO lessons (id,slug,title,ord,status,skip,created_at,updated_at) \
         VALUES (?1,?2,?3,(SELECT COALESCE(MAX(ord),0)+1 FROM lessons),'not_started',0,?4,?5) \
         RETURNING ord",
        params![id, slug, title, created_at, updated_at],
        |r| r.get(0),
    )
    .map_err(|e| lesson_insert_err(e, 0))
}

/// Is a lesson slug already taken?
pub fn lesson_slug_taken(conn: &Connection, slug: &str) -> Result<bool, CarpenterError> {
    let id: Option<String> = conn
        .query_row("SELECT id FROM lessons WHERE slug=?1", params![slug], |r| {
            r.get(0)
        })
        .optional()
        .map_err(store_msg)?;
    Ok(id.is_some())
}

/// Read a lesson by id; [`CarpenterError::NotFound`] if absent.
pub fn get_lesson(conn: &Connection, id: &str) -> Result<LessonDb, CarpenterError> {
    conn.query_row(
        "SELECT id,slug,title,ord,status,skip,created_at,updated_at FROM lessons WHERE id=?1",
        params![id],
        lesson_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => CarpenterError::NotFound(format!("lesson {id}")),
        e => store_msg(e),
    })
}

fn lesson_row(r: &rusqlite::Row) -> rusqlite::Result<LessonDb> {
    Ok(LessonDb {
        id: r.get(0)?,
        slug: r.get(1)?,
        title: r.get(2)?,
        ord: r.get(3)?,
        status: r.get(4)?,
        skip: r.get::<_, i64>(5)? != 0,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
    })
}

/// List all lessons, ordered by `ord`.
pub fn list_lessons(conn: &Connection) -> Result<Vec<LessonDb>, CarpenterError> {
    let rows = conn
        .prepare(
            "SELECT id,slug,title,ord,status,skip,created_at,updated_at FROM lessons ORDER BY ord",
        )
        .map_err(store_msg)?
        .query_map([], lesson_row)
        .map_err(store_msg)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(store_msg)?;
    Ok(rows)
}

/// Counts for `lesson show` progress.
#[derive(Debug, Clone, Copy)]
pub struct LessonShowCounts {
    /// section count.
    pub sections: i64,
    /// practice count.
    pub practice: i64,
    /// quiz count.
    pub quizzes: i64,
    /// non-skipped practice+quiz (total).
    pub total: i64,
    /// non-skipped practice+quiz with `pass_or_fail=1`.
    pub passing: i64,
}

/// Gather `lesson show` progress counts.
pub fn lesson_show_counts(
    conn: &Connection,
    lesson_id: &str,
) -> Result<LessonShowCounts, CarpenterError> {
    let one = |sql: &str| -> Result<i64, CarpenterError> {
        conn.query_row(sql, params![lesson_id], |r| r.get::<_, i64>(0))
            .map_err(store_msg)
    };
    let sections = one("SELECT COUNT(*) FROM sections WHERE lesson_id=?1")?;
    let practice = one(
        "SELECT COUNT(*) FROM practice p JOIN sections s ON p.section_id=s.id WHERE s.lesson_id=?1",
    )?;
    let quizzes = one("SELECT COUNT(*) FROM quizzes WHERE lesson_id=?1")?;
    let (total, passing) = match lesson_status_inputs(conn, lesson_id)? {
        Some(inp) => (inp.total_items, inp.passing_items),
        None => (0, 0),
    };
    Ok(LessonShowCounts {
        sections,
        practice,
        quizzes,
        total,
        passing,
    })
}

/// Update a lesson's title + `updated_at`.
pub fn update_lesson_meta(
    conn: &Connection,
    id: &str,
    title: &str,
    updated_at: &str,
) -> Result<(), CarpenterError> {
    conn.execute(
        "UPDATE lessons SET title=?2, updated_at=?3 WHERE id=?1",
        params![id, title, updated_at],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Delete all of a lesson's content (test cases for its owners, sections, quizzes)
/// but keep the lesson row (used by `update`).
pub fn delete_lesson_content(conn: &Connection, lesson_id: &str) -> Result<(), CarpenterError> {
    let practice_ids: Vec<String> = conn
        .prepare(
            "SELECT p.id FROM practice p JOIN sections s ON p.section_id=s.id WHERE s.lesson_id=?1",
        )
        .map_err(store_msg)?
        .query_map(params![lesson_id], |r| r.get::<_, String>(0))
        .map_err(store_msg)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(store_msg)?;
    let quiz_ids: Vec<String> = conn
        .prepare("SELECT id FROM quizzes WHERE lesson_id=?1")
        .map_err(store_msg)?
        .query_map(params![lesson_id], |r| r.get::<_, String>(0))
        .map_err(store_msg)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(store_msg)?;
    for pid in &practice_ids {
        conn.execute(
            "DELETE FROM test_cases WHERE owner_type='practice' AND owner_id=?1",
            params![pid],
        )
        .map_err(store_msg)?;
    }
    for qid in &quiz_ids {
        conn.execute(
            "DELETE FROM test_cases WHERE owner_type='quiz' AND owner_id=?1",
            params![qid],
        )
        .map_err(store_msg)?;
    }
    conn.execute(
        "DELETE FROM sections WHERE lesson_id=?1",
        params![lesson_id],
    )
    .map_err(store_msg)?;
    conn.execute("DELETE FROM quizzes WHERE lesson_id=?1", params![lesson_id])
        .map_err(store_msg)?;
    Ok(())
}

/// Delete a lesson and all its content (cascade).
pub fn delete_lesson(conn: &Connection, lesson_id: &str) -> Result<(), CarpenterError> {
    delete_lesson_content(conn, lesson_id)?;
    conn.execute("DELETE FROM lessons WHERE id=?1", params![lesson_id])
        .map_err(store_msg)?;
    Ok(())
}

/// Insert a sections row.
pub fn insert_section(
    conn: &Connection,
    id: &str,
    lesson_id: &str,
    title: &str,
    snippets_json: &str,
    ord: i64,
) -> Result<(), CarpenterError> {
    conn.execute(
        "INSERT INTO sections (id,lesson_id,title,snippets,ord) VALUES (?1,?2,?3,?4,?5)",
        params![id, lesson_id, title, snippets_json, ord],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// List sections of a lesson, ordered by `ord`.
pub fn list_sections(conn: &Connection, lesson_id: &str) -> Result<Vec<SectionDb>, CarpenterError> {
    let rows = conn
        .prepare("SELECT id,title,snippets,ord FROM sections WHERE lesson_id=?1 ORDER BY ord")
        .map_err(store_msg)?
        .query_map(params![lesson_id], |r| {
            Ok(SectionDb {
                id: r.get(0)?,
                title: r.get(1)?,
                snippets: r.get(2)?,
                ord: r.get(3)?,
            })
        })
        .map_err(store_msg)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(store_msg)?;
    Ok(rows)
}

const CHECKABLE_COLS: &str = "id,name,signature,prompt,solution,ord,skip,pass_or_fail,last_check";

/// List practice items of a section, ordered by `ord`.
pub fn list_practice(
    conn: &Connection,
    section_id: &str,
) -> Result<Vec<CheckableDb>, CarpenterError> {
    let rows = conn
        .prepare(&format!(
            "SELECT {CHECKABLE_COLS} FROM practice WHERE section_id=?1 ORDER BY ord"
        ))
        .map_err(store_msg)?
        .query_map(params![section_id], checkable_mapper)
        .map_err(store_msg)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(store_msg)?;
    Ok(rows)
}

/// Insert a practice row. `state` carries the learner-state columns — fresh
/// inserts pass [`reconcile::CheckableState::fresh`]; `lesson update --force`
/// passes the carried columns for content-unchanged children.
#[allow(clippy::too_many_arguments)]
pub fn insert_practice(
    conn: &Connection,
    id: &str,
    section_id: &str,
    name: &str,
    signature: &str,
    prompt: &str,
    solution: &str,
    ord: i64,
    state: &reconcile::CheckableState,
) -> Result<(), CarpenterError> {
    conn.execute(
        "INSERT INTO practice (id,section_id,name,signature,prompt,solution,ord,pass_or_fail,last_check,skip) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            id,
            section_id,
            name,
            signature,
            prompt,
            solution,
            ord,
            i64::from(state.pass_or_fail),
            state.last_check,
            i64::from(state.skip),
        ],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// List quizzes of a lesson, ordered by `ord`.
pub fn list_quizzes(
    conn: &Connection,
    lesson_id: &str,
) -> Result<Vec<CheckableDb>, CarpenterError> {
    let rows = conn
        .prepare(&format!(
            "SELECT {CHECKABLE_COLS} FROM quizzes WHERE lesson_id=?1 ORDER BY ord"
        ))
        .map_err(store_msg)?
        .query_map(params![lesson_id], checkable_mapper)
        .map_err(store_msg)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(store_msg)?;
    Ok(rows)
}

/// Insert a quiz row. `state` carries the learner-state columns — fresh
/// inserts pass [`reconcile::CheckableState::fresh`]; `lesson update --force`
/// passes the carried columns for content-unchanged children.
#[allow(clippy::too_many_arguments)]
pub fn insert_quiz(
    conn: &Connection,
    id: &str,
    lesson_id: &str,
    name: &str,
    signature: &str,
    prompt: &str,
    solution: &str,
    ord: i64,
    state: &reconcile::CheckableState,
) -> Result<(), CarpenterError> {
    conn.execute(
        "INSERT INTO quizzes (id,lesson_id,name,signature,prompt,solution,ord,pass_or_fail,last_check,skip) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            id,
            lesson_id,
            name,
            signature,
            prompt,
            solution,
            ord,
            i64::from(state.pass_or_fail),
            state.last_check,
            i64::from(state.skip),
        ],
    )
    .map_err(store_msg)?;
    Ok(())
}

fn checkable_mapper(r: &rusqlite::Row) -> rusqlite::Result<CheckableDb> {
    Ok(CheckableDb {
        id: r.get(0)?,
        name: r.get(1)?,
        signature: r.get(2)?,
        prompt: r.get(3)?,
        solution: r.get(4)?,
        ord: r.get(5)?,
        skip: r.get::<_, i64>(6)? != 0,
        pass_or_fail: r.get::<_, i64>(7)? != 0,
        last_check: r.get(8)?,
    })
}

/// Read a quiz by id; [`CarpenterError::NotFound`] if absent.
pub fn get_quiz(conn: &Connection, id: &str) -> Result<CheckableDb, CarpenterError> {
    conn.query_row(
        &format!("SELECT {CHECKABLE_COLS} FROM quizzes WHERE id=?1"),
        params![id],
        checkable_mapper,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => CarpenterError::NotFound(format!("quiz {id}")),
        e => store_msg(e),
    })
}

/// The lesson id that owns a quiz.
pub fn quiz_lesson_id(conn: &Connection, quiz_id: &str) -> Result<String, CarpenterError> {
    conn.query_row(
        "SELECT lesson_id FROM quizzes WHERE id=?1",
        params![quiz_id],
        |r| r.get::<_, String>(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => CarpenterError::NotFound(format!("quiz {quiz_id}")),
        e => store_msg(e),
    })
}

/// Read a practice item by id; [`CarpenterError::NotFound`] if absent.
pub fn get_practice(conn: &Connection, id: &str) -> Result<CheckableDb, CarpenterError> {
    conn.query_row(
        &format!("SELECT {CHECKABLE_COLS} FROM practice WHERE id=?1"),
        params![id],
        checkable_mapper,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => CarpenterError::NotFound(format!("practice {id}")),
        e => store_msg(e),
    })
}

/// The lesson id that owns a practice item; [`CarpenterError::NotFound`] if the
/// practice id is absent.
pub fn practice_lesson_id(conn: &Connection, practice_id: &str) -> Result<String, CarpenterError> {
    conn.query_row(
        "SELECT s.lesson_id FROM practice p JOIN sections s ON p.section_id=s.id WHERE p.id=?1",
        params![practice_id],
        |r| r.get::<_, String>(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            CarpenterError::NotFound(format!("practice {practice_id}"))
        }
        e => store_msg(e),
    })
}

/// Set the `skip` column on a `lessons`/`practice`/`quizzes` row (`table` is an
/// internal constant, never user input).
pub fn set_skip(
    conn: &Connection,
    table: &str,
    id: &str,
    skip: bool,
) -> Result<(), CarpenterError> {
    let v: i64 = if skip { 1 } else { 0 };
    conn.execute(
        &format!("UPDATE {table} SET skip=?2 WHERE id=?1"),
        params![id, v],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Refresh the denormalized `lessons.status` cache (recomputed on read too —
/// `docs/data-model/04-status-derivation.md`).
pub fn set_lesson_status(conn: &Connection, id: &str, status: &str) -> Result<(), CarpenterError> {
    conn.execute(
        "UPDATE lessons SET status=?2 WHERE id=?1",
        params![id, status],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Count test cases for an owner.
pub fn owner_case_count(
    conn: &Connection,
    owner_type: &str,
    owner_id: &str,
) -> Result<i64, CarpenterError> {
    conn.query_row(
        "SELECT COUNT(*) FROM test_cases WHERE owner_type=?1 AND owner_id=?2",
        params![owner_type, owner_id],
        |r| r.get::<_, i64>(0),
    )
    .map_err(store_msg)
}

/// Insert a test_cases row.
#[allow(clippy::too_many_arguments)]
pub fn insert_test_case(
    conn: &Connection,
    id: &str,
    owner_type: &str,
    owner_id: &str,
    args_json: &str,
    kwargs_json: &str,
    expected_json: &str,
    compare: &str,
    ord: i64,
) -> Result<(), CarpenterError> {
    conn.execute(
        "INSERT INTO test_cases (id,owner_type,owner_id,args,kwargs,expected,compare,ord) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            id,
            owner_type,
            owner_id,
            args_json,
            kwargs_json,
            expected_json,
            compare,
            ord
        ],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// List test cases for an owner, ordered by `ord`.
pub fn list_cases(
    conn: &Connection,
    owner_type: &str,
    owner_id: &str,
) -> Result<Vec<CaseDb>, CarpenterError> {
    let rows = conn
        .prepare(
            "SELECT id,args,kwargs,expected,compare,ord FROM test_cases \
             WHERE owner_type=?1 AND owner_id=?2 ORDER BY ord",
        )
        .map_err(store_msg)?
        .query_map(params![owner_type, owner_id], |r| {
            Ok(CaseDb {
                id: r.get(0)?,
                args: r.get(1)?,
                kwargs: r.get(2)?,
                expected: r.get(3)?,
                compare: r.get(4)?,
                ord: r.get(5)?,
            })
        })
        .map_err(store_msg)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(store_msg)?;
    Ok(rows)
}

/// Snapshot a lesson's current content as reconcile-comparable shapes — read
/// BEFORE [`delete_lesson_content`] so `lesson update --force` can re-insert
/// content-unchanged children under their old ids with their learner-state
/// columns. Lists come back lowest-`ord` first (the matching order).
pub fn snapshot_lesson_content(
    conn: &Connection,
    lesson_id: &str,
) -> Result<reconcile::OldContent, CarpenterError> {
    let mut content = reconcile::OldContent::default();
    for s in list_sections(conn, lesson_id)? {
        let mut practice = Vec::new();
        for p in list_practice(conn, &s.id)? {
            practice.push(snapshot_checkable(conn, "practice", &p)?);
        }
        content.sections.push(reconcile::OldSection {
            id: s.id,
            title: s.title,
            snippets: snippet_pairs(&s.snippets),
            practice,
        });
    }
    for q in list_quizzes(conn, lesson_id)? {
        content.quizzes.push(snapshot_checkable(conn, "quiz", &q)?);
    }
    Ok(content)
}

fn snapshot_checkable(
    conn: &Connection,
    owner_type: &str,
    c: &CheckableDb,
) -> Result<reconcile::OldCheckable, CarpenterError> {
    let cases = list_cases(conn, owner_type, &c.id)?
        .into_iter()
        .map(|cc| reconcile::OldCase {
            id: cc.id,
            compare: cc.compare,
            args: cc.args,
            kwargs: cc.kwargs,
            expected: cc.expected,
        })
        .collect();
    Ok(reconcile::OldCheckable {
        id: c.id.clone(),
        name: c.name.clone(),
        signature: c.signature.clone(),
        prompt: c.prompt.clone(),
        solution: c.solution.clone(),
        state: reconcile::CheckableState {
            skip: c.skip,
            pass_or_fail: c.pass_or_fail,
            last_check: c.last_check.clone(),
        },
        cases,
    })
}

/// `(kind, content)` per stored snippet — the `sn{n}` ids are dropped (they
/// are renumbered on every insert, never matched). Lenient on corrupt JSON
/// (same policy as the renderer): an unparseable value matches nothing, which
/// at worst re-keys the section, never loses data.
fn snippet_pairs(snippets_json: &str) -> Vec<(String, String)> {
    #[derive(serde::Deserialize)]
    struct Snippet {
        kind: String,
        content: String,
    }
    serde_json::from_str::<Vec<Snippet>>(snippets_json)
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.kind, s.content))
        .collect()
}

// ---- notes ----

/// A notes row (`tags` is raw JSON; parsing is the caller's job so corrupt rows
/// can surface in `errors[]`).
#[derive(Debug, Clone)]
pub struct NoteDb {
    /// id (`n1`…).
    pub id: String,
    /// created timestamp.
    pub ts: String,
    /// last-update timestamp.
    pub updated_ts: String,
    /// kind.
    pub kind: String,
    /// tags JSON (`["…"]`).
    pub tags: String,
    /// `open` | `resolved`.
    pub status: String,
    /// `new` | `recurring`.
    pub recurrence: String,
    /// free lesson/quiz ref (may be empty).
    pub related: String,
    /// the note body.
    pub text: String,
}

const NOTE_COLS: &str = "id,ts,updated_ts,kind,tags,status,recurrence,related,text FROM notes";

fn note_row(r: &rusqlite::Row) -> rusqlite::Result<NoteDb> {
    Ok(NoteDb {
        id: r.get(0)?,
        ts: r.get(1)?,
        updated_ts: r.get(2)?,
        kind: r.get(3)?,
        tags: r.get(4)?,
        status: r.get(5)?,
        recurrence: r.get(6)?,
        related: r.get(7)?,
        text: r.get(8)?,
    })
}

/// Insert a notes row.
pub fn insert_note(conn: &Connection, row: &NoteDb) -> Result<(), CarpenterError> {
    conn.execute(
        "INSERT INTO notes (id,ts,updated_ts,kind,tags,status,recurrence,related,text) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            row.id,
            row.ts,
            row.updated_ts,
            row.kind,
            row.tags,
            row.status,
            row.recurrence,
            row.related,
            row.text
        ],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Read a note by id; [`CarpenterError::NotFound`] if absent.
pub fn get_note(conn: &Connection, id: &str) -> Result<NoteDb, CarpenterError> {
    conn.query_row(
        &format!("SELECT {NOTE_COLS} WHERE id=?1"),
        params![id],
        note_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => CarpenterError::NotFound(format!("note {id}")),
        e => store_msg(e),
    })
}

/// List all notes, ordered by `ts` then `id`.
pub fn list_notes(conn: &Connection) -> Result<Vec<NoteDb>, CarpenterError> {
    let rows = conn
        .prepare(&format!("SELECT {NOTE_COLS} ORDER BY ts, id"))
        .map_err(store_msg)?
        .query_map([], note_row)
        .map_err(store_msg)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(store_msg)?;
    Ok(rows)
}

/// Replace a note's authored fields (for `update`); `status` is untouched.
#[allow(clippy::too_many_arguments)]
pub fn update_note(
    conn: &Connection,
    id: &str,
    kind: &str,
    tags_json: &str,
    recurrence: &str,
    related: &str,
    text: &str,
    updated_ts: &str,
) -> Result<(), CarpenterError> {
    conn.execute(
        "UPDATE notes SET kind=?2, tags=?3, recurrence=?4, related=?5, text=?6, updated_ts=?7 \
         WHERE id=?1",
        params![id, kind, tags_json, recurrence, related, text, updated_ts],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Set a note's status (for `resolve`).
pub fn set_note_status(
    conn: &Connection,
    id: &str,
    status: &str,
    updated_ts: &str,
) -> Result<(), CarpenterError> {
    conn.execute(
        "UPDATE notes SET status=?2, updated_ts=?3 WHERE id=?1",
        params![id, status, updated_ts],
    )
    .map_err(store_msg)?;
    Ok(())
}

/// Delete a note.
pub fn delete_note(conn: &Connection, id: &str) -> Result<(), CarpenterError> {
    conn.execute("DELETE FROM notes WHERE id=?1", params![id])
        .map_err(store_msg)?;
    Ok(())
}

// ---- progress roll-ups ----

/// Non-skipped quiz roll-up: `(passing, total)`.
pub fn quiz_rollup(conn: &Connection) -> Result<(i64, i64), CarpenterError> {
    conn.query_row(
        "SELECT COALESCE(SUM(pass_or_fail), 0), COUNT(*) FROM quizzes WHERE skip=0",
        [],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
    )
    .map_err(store_msg)
}

/// Aggregate note counts in one scan (all notes, regardless of status).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoteCounts {
    /// all notes.
    pub total: i64,
    /// `status='open'`.
    pub open: i64,
    /// `recurrence='recurring'`.
    pub recurring: i64,
    /// `kind='gap'`.
    pub gap: i64,
    /// `kind='mistake'`.
    pub mistake: i64,
    /// `kind='strength'`.
    pub strength: i64,
    /// `kind='pattern'`.
    pub pattern: i64,
    /// `kind='progress'`.
    pub progress: i64,
}

/// Gather the aggregate note counts.
pub fn note_counts(conn: &Connection) -> Result<NoteCounts, CarpenterError> {
    conn.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(status = 'open'), 0),
                COALESCE(SUM(recurrence = 'recurring'), 0),
                COALESCE(SUM(kind = 'gap'), 0),
                COALESCE(SUM(kind = 'mistake'), 0),
                COALESCE(SUM(kind = 'strength'), 0),
                COALESCE(SUM(kind = 'pattern'), 0),
                COALESCE(SUM(kind = 'progress'), 0)
         FROM notes",
        [],
        |r| {
            Ok(NoteCounts {
                total: r.get(0)?,
                open: r.get(1)?,
                recurring: r.get(2)?,
                gap: r.get(3)?,
                mistake: r.get(4)?,
                strength: r.get(5)?,
                pattern: r.get(6)?,
                progress: r.get(7)?,
            })
        },
    )
    .map_err(store_msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn tmp_db() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("carpenter-db-{}-{n}.db", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn open_applies_schema_idempotently() {
        let path = tmp_db();
        {
            let conn = open(&path).expect("open");
            // tables exist
            conn.execute("INSERT INTO course_meta (slug,title,goal,description,created_at) VALUES ('c','t','g','','now')", [])
                .expect("insert");
        }
        let conn = open(&path).expect("reopen");
        let row = get_course_meta(&conn, "c").expect("get");
        assert_eq!(row.title, "t");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn course_meta_roundtrip() {
        let conn = open(&tmp_db()).expect("open");
        let row = CourseRow {
            slug: "ds".into(),
            title: "Data Structures".into(),
            goal: "learn".into(),
            description: "desc".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        insert_course_meta(&conn, &row).expect("insert");
        let got = get_course_meta(&conn, "ds").expect("get");
        assert_eq!(got.goal, "learn");
        update_course_meta(&conn, "ds", "T2", "G2", "D2").expect("update");
        assert_eq!(get_course_meta(&conn, "ds").unwrap().title, "T2");
        assert!(matches!(
            get_course_meta(&conn, "missing"),
            Err(CarpenterError::NotFound(_))
        ));
    }

    #[test]
    fn next_id_is_monotonic_and_never_reuses() {
        let conn = open(&tmp_db()).expect("open");
        assert_eq!(next_id(&conn, "sections", "s").unwrap(), "s1");
        assert_eq!(next_id(&conn, "sections", "s").unwrap(), "s2");
        // simulate a delete elsewhere; the counter is unaffected
        assert_eq!(next_id(&conn, "sections", "s").unwrap(), "s3");
        // independent table, independent counter
        assert_eq!(next_id(&conn, "practice", "p").unwrap(), "p1");
    }

    #[test]
    fn counts_start_zero() {
        let conn = open(&tmp_db()).expect("open");
        let c = course_counts(&conn).expect("counts");
        assert_eq!(c.lessons, 0);
        assert_eq!(c.practice, 0);
    }

    #[test]
    fn practice_accessors_and_skip_columns() {
        let conn = open(&tmp_db()).expect("open");
        insert_lesson(&conn, "l1", "l1", "L", 1, "t0", "t0").expect("lesson");
        insert_section(&conn, "s1", "l1", "S", "[]", 0).expect("section");
        insert_practice(
            &conn,
            "p1",
            "s1",
            "f",
            "def f():",
            "",
            "",
            0,
            &reconcile::CheckableState::fresh(),
        )
        .expect("practice");
        insert_quiz(
            &conn,
            "q1",
            "l1",
            "g",
            "def g():",
            "",
            "",
            0,
            &reconcile::CheckableState::fresh(),
        )
        .expect("quiz");
        assert_eq!(practice_lesson_id(&conn, "p1").expect("owner"), "l1");
        assert_eq!(get_practice(&conn, "p1").expect("get").name, "f");
        assert!(matches!(
            get_practice(&conn, "nope"),
            Err(CarpenterError::NotFound(_))
        ));
        assert!(matches!(
            practice_lesson_id(&conn, "nope"),
            Err(CarpenterError::NotFound(_))
        ));
        set_skip(&conn, "practice", "p1", true).expect("set");
        assert!(get_practice(&conn, "p1").unwrap().skip);
        set_skip(&conn, "quizzes", "q1", true).expect("set");
        assert!(get_quiz(&conn, "q1").unwrap().skip);
        set_skip(&conn, "practice", "p1", false).expect("clear");
        assert!(!get_practice(&conn, "p1").unwrap().skip);
        set_lesson_status(&conn, "l1", "skipped").expect("status");
        assert_eq!(get_lesson(&conn, "l1").unwrap().status, "skipped");
    }

    #[test]
    fn insert_carries_learner_state_columns() {
        let conn = open(&tmp_db()).expect("open");
        insert_lesson(&conn, "l1", "l1", "L", 1, "t0", "t0").expect("lesson");
        insert_section(&conn, "s1", "l1", "S", "[]", 0).expect("section");
        let carried = reconcile::CheckableState {
            skip: true,
            pass_or_fail: true,
            last_check: String::from(r#"{"passed":1,"total":1}"#),
        };
        insert_practice(&conn, "p1", "s1", "f", "def f():", "", "", 0, &carried).expect("practice");
        insert_quiz(&conn, "q1", "l1", "g", "def g():", "", "", 0, &carried).expect("quiz");
        for got in [
            get_practice(&conn, "p1").expect("practice"),
            get_quiz(&conn, "q1").expect("quiz"),
        ] {
            assert!(got.skip);
            assert!(got.pass_or_fail);
            assert_eq!(got.last_check, r#"{"passed":1,"total":1}"#);
        }
    }

    #[test]
    fn snapshot_lesson_content_reads_children_lowest_ord_first() {
        let conn = open(&tmp_db()).expect("open");
        insert_lesson(&conn, "l1", "l1", "L", 1, "t0", "t0").expect("lesson");
        insert_section(
            &conn,
            "s2",
            "l1",
            "S2",
            r#"[{"id":"sn1","kind":"code","content":"x"}]"#,
            1,
        )
        .expect("s2");
        insert_section(
            &conn,
            "s1",
            "l1",
            "S1",
            r#"[{"id":"sn1","kind":"markdown","content":"hi"}]"#,
            0,
        )
        .expect("s1");
        insert_practice(
            &conn,
            "p1",
            "s1",
            "f",
            "def f():",
            "p",
            "sol",
            0,
            &reconcile::CheckableState::fresh(),
        )
        .expect("p1");
        conn.execute(
            "UPDATE practice SET skip=1, pass_or_fail=1, last_check='{\"passed\":1}' WHERE id='p1'",
            [],
        )
        .unwrap();
        insert_test_case(&conn, "c1", "practice", "p1", "[1]", "{}", "1", "exact", 0)
            .expect("case");
        insert_quiz(
            &conn,
            "q1",
            "l1",
            "g",
            "def g():",
            "",
            "",
            0,
            &reconcile::CheckableState::fresh(),
        )
        .expect("q1");
        let snap = snapshot_lesson_content(&conn, "l1").expect("snapshot");
        let sec_ids: Vec<_> = snap.sections.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(sec_ids, ["s1", "s2"]);
        assert_eq!(
            snap.sections[0].snippets,
            vec![(String::from("markdown"), String::from("hi"))]
        );
        let p = &snap.sections[0].practice[0];
        assert_eq!(p.id, "p1");
        assert!(p.state.skip && p.state.pass_or_fail);
        assert_eq!(p.state.last_check, r#"{"passed":1}"#);
        assert_eq!(p.cases[0].id, "c1");
        assert_eq!(snap.quizzes[0].id, "q1");
    }

    #[test]
    fn note_accessors_roundtrip() {
        let conn = open(&tmp_db()).expect("open");
        let row = NoteDb {
            id: String::from("n1"),
            ts: String::from("2026-08-09T12:00:00Z"),
            updated_ts: String::from("2026-08-09T12:00:00Z"),
            kind: String::from("gap"),
            tags: String::from(r#"["recursion"]"#),
            status: String::from("open"),
            recurrence: String::from("new"),
            related: String::from("q2"),
            text: String::from("struggles with base cases"),
        };
        insert_note(&conn, &row).expect("insert");
        let got = get_note(&conn, "n1").expect("get");
        assert_eq!(got.kind, "gap");
        assert!(matches!(
            get_note(&conn, "nope"),
            Err(CarpenterError::NotFound(_))
        ));
        update_note(
            &conn,
            "n1",
            "mistake",
            "[]",
            "recurring",
            "",
            "edited",
            "2026-08-09T13:00:00Z",
        )
        .expect("update");
        let after = get_note(&conn, "n1").unwrap();
        assert_eq!(after.kind, "mistake");
        assert_eq!(after.status, "open"); // update preserves status
        assert_eq!(after.updated_ts, "2026-08-09T13:00:00Z");
        set_note_status(&conn, "n1", "resolved", "2026-08-09T14:00:00Z").expect("resolve");
        assert_eq!(get_note(&conn, "n1").unwrap().status, "resolved");
        assert_eq!(list_notes(&conn).unwrap().len(), 1);
        delete_note(&conn, "n1").expect("delete");
        assert!(matches!(
            get_note(&conn, "n1"),
            Err(CarpenterError::NotFound(_))
        ));
    }

    #[test]
    fn quiz_and_note_rollups() {
        let conn = open(&tmp_db()).expect("open");
        insert_lesson(&conn, "l1", "l1", "L", 1, "t0", "t0").expect("lesson");
        insert_quiz(
            &conn,
            "q1",
            "l1",
            "g",
            "def g():",
            "",
            "",
            0,
            &reconcile::CheckableState::fresh(),
        )
        .expect("quiz");
        insert_quiz(
            &conn,
            "q2",
            "l1",
            "g",
            "def g():",
            "",
            "",
            1,
            &reconcile::CheckableState::fresh(),
        )
        .expect("quiz");
        // q1 passes, q2 skipped — skipped is excluded from the roll-up.
        conn.execute("UPDATE quizzes SET pass_or_fail=1 WHERE id='q1'", [])
            .unwrap();
        set_skip(&conn, "quizzes", "q2", true).unwrap();
        let (passing, total) = quiz_rollup(&conn).unwrap();
        assert_eq!((passing, total), (1, 1));

        let mk = |id: &str, kind: &str, status: &str, rec: &str| NoteDb {
            id: id.into(),
            ts: format!("2026-08-09T12:00:0{id}Z"),
            updated_ts: format!("2026-08-09T12:00:0{id}Z"),
            kind: kind.into(),
            tags: "[]".into(),
            status: status.into(),
            recurrence: rec.into(),
            related: String::new(),
            text: "t".into(),
        };
        insert_note(&conn, &mk("n1", "gap", "open", "recurring")).unwrap();
        insert_note(&conn, &mk("n2", "gap", "resolved", "new")).unwrap();
        insert_note(&conn, &mk("n3", "pattern", "open", "new")).unwrap();
        let c = note_counts(&conn).unwrap();
        assert_eq!(c.total, 3);
        assert_eq!(c.open, 2);
        assert_eq!(c.recurring, 1);
        assert_eq!(c.gap, 2);
        assert_eq!(c.pattern, 1);
        assert_eq!(c.progress, 0);
    }
}
