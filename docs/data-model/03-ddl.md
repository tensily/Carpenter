# DDL

```sql
-- course meta (single row) mirrors course.json for query convenience.
-- NOTE: course_meta.goal is the single course-level mission statement
-- (prose, authored via CourseSpec). It is DISTINCT from the `goals` table,
-- which holds granular course-scope objective rows created by `plan confirm`.
CREATE TABLE course_meta (
  slug        TEXT PRIMARY KEY,
  title       TEXT NOT NULL,
  goal        TEXT NOT NULL DEFAULT '',
  description TEXT NOT NULL DEFAULT '',
  created_at  TEXT NOT NULL
);

CREATE TABLE lessons (
  id         TEXT PRIMARY KEY,            -- slug, e.g. arrays-101
  slug       TEXT NOT NULL UNIQUE,
  title      TEXT NOT NULL,
  ord        INTEGER NOT NULL,            -- unique (idx_lessons_ord, adr/017);
                                           -- auto = max(ord)+1 inside the insert (atomic);
                                           -- explicit spec.order colliding => Conflict
  status     TEXT NOT NULL DEFAULT 'not_started'
             CHECK (status IN ('not_started','in_progress','complete','skipped')),
  skip       INTEGER NOT NULL DEFAULT 0,  -- 1 = whole lesson skipped (-> status 'skipped')
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- adr/017: enforced by a guarded migration (legacy duplicate ords are
-- resequenced densely on open, stable by ord, created_at, id).
CREATE UNIQUE INDEX idx_lessons_ord ON lessons(ord);

CREATE TABLE sections (
  id        TEXT PRIMARY KEY,             -- s1, s2, ...
  lesson_id TEXT NOT NULL REFERENCES lessons(id) ON DELETE CASCADE,
  title     TEXT NOT NULL,
  snippets  TEXT NOT NULL DEFAULT '[]',   -- JSON [{id, kind(markdown|code), content}, …]
                                          -- rule (app-enforced): snippets[0].kind == 'markdown'
  ord       INTEGER NOT NULL
);

CREATE TABLE practice (
  id           TEXT PRIMARY KEY,          -- p1, p2, ...
  section_id   TEXT NOT NULL REFERENCES sections(id) ON DELETE CASCADE,
  name         TEXT NOT NULL,             -- function name
  signature    TEXT NOT NULL,             -- def sum_array(arr):
  prompt       TEXT NOT NULL DEFAULT '',
  solution     TEXT NOT NULL DEFAULT '',  -- author reference solution (adr/015); author-only, never rendered
  ord          INTEGER NOT NULL,
  pass_or_fail INTEGER NOT NULL DEFAULT 0,-- 1 = last check passed all cases (set by helper)
  last_check   TEXT NOT NULL DEFAULT '{}',-- JSON {passed,total,cases:[{case_id,passed,error?}]}
  skip         INTEGER NOT NULL DEFAULT 0 -- 1 = excluded from lesson status derivation
);

CREATE TABLE quizzes (
  id           TEXT PRIMARY KEY,          -- q1, q2, ...
  lesson_id    TEXT NOT NULL REFERENCES lessons(id) ON DELETE CASCADE,
  name         TEXT NOT NULL,
  signature    TEXT NOT NULL,
  prompt       TEXT NOT NULL DEFAULT '',
  solution     TEXT NOT NULL DEFAULT '',  -- author reference solution (adr/015); author-only, never rendered
  ord          INTEGER NOT NULL,
  pass_or_fail INTEGER NOT NULL DEFAULT 0,-- 1 = last check passed all cases (set by helper)
  last_check   TEXT NOT NULL DEFAULT '{}',-- JSON {passed,total,cases:[{case_id,passed,error?}]}
  skip         INTEGER NOT NULL DEFAULT 0 -- 1 = excluded from lesson status derivation
);

CREATE TABLE test_cases (
  id        TEXT PRIMARY KEY,             -- c1, c2, ...
  owner_type TEXT NOT NULL CHECK (owner_type IN ('practice','quiz')),
  owner_id   TEXT NOT NULL,               -- -> practice.id | quizzes.id
  args       TEXT NOT NULL DEFAULT '[]',  -- JSON array
  kwargs     TEXT NOT NULL DEFAULT '{}',  -- JSON object
  expected   TEXT NOT NULL,               -- JSON (any) — never shown to learner
  compare    TEXT NOT NULL DEFAULT 'exact'
             CHECK (compare IN ('exact','sorted','set')),
  ord        INTEGER NOT NULL
);
CREATE INDEX idx_cases_owner ON test_cases(owner_type, owner_id);

CREATE TABLE notes (
  id         TEXT PRIMARY KEY,            -- n1, n2, ...
  ts         TEXT NOT NULL,
  updated_ts TEXT NOT NULL,
  kind       TEXT NOT NULL
             CHECK (kind IN ('gap','mistake','strength','pattern','progress')),
  tags       TEXT NOT NULL DEFAULT '[]',  -- JSON array[str]
  status     TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','resolved')),
  recurrence TEXT NOT NULL DEFAULT 'new'  CHECK (recurrence IN ('new','recurring')),
  related    TEXT NOT NULL DEFAULT '',    -- free ref (lesson/quiz id)
  text       TEXT NOT NULL
);

CREATE TABLE plans (
  id          TEXT PRIMARY KEY,           -- pl1, pl2, ...
  scope       TEXT NOT NULL CHECK (scope IN ('course','lesson')),
  scope_id    TEXT NOT NULL,              -- course slug | lesson id
  title       TEXT NOT NULL,
  content     TEXT NOT NULL,              -- markdown / JSON document
  created_at  TEXT NOT NULL,
  confirmed_at TEXT                       -- NULL until plan confirm
);

CREATE TABLE goals (
  id         TEXT PRIMARY KEY,            -- g1, g2, ...
  scope      TEXT NOT NULL DEFAULT 'course' CHECK (scope IN ('course')),
  scope_id   TEXT NOT NULL,               -- course slug
  text       TEXT NOT NULL,               -- the bullet goal
  status     TEXT NOT NULL DEFAULT 'pending'
             CHECK (status IN ('pending','achieved','skipped')),
  covered_by TEXT NOT NULL DEFAULT '[]',  -- JSON array[lesson id]
  override   INTEGER NOT NULL DEFAULT 0,  -- 1 = manual status, skip derivation
  created_at TEXT NOT NULL
);
CREATE INDEX idx_goals_scope ON goals(scope, scope_id);

-- id allocation: one row per prefixed table. `next` only grows across committed
-- transactions, so deleted ids are never reused (the `max+1 over existing rows`
-- alternative would reuse them). A rolled-back transaction restores `next` — the
-- rolled-back id was never attached to a row, so the never-reuse guarantee holds.
CREATE TABLE id_seq (
  tbl  TEXT PRIMARY KEY,            -- prefixed table name (sections, practice, ...)
  next INTEGER NOT NULL             -- next numeric suffix to hand out
);
```
