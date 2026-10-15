# Conventions

- **Primary keys:** stable string ids (`s1`, `p1`, `q1`, `c1`, `n1`, `pl1`, `g1`),
  assigned by carpenter, globally unique **within their table** (the `id` column is
  the PK). Assignment is `max+1` per table over existing numeric suffixes; deleted
  ids are **never reused**. SQLite `rowid` is ignored as a key. (The `a1` attempts
  prefix was removed with that table — adr/010.) **Exception — `lesson update
  --force`:** children whose spec content is unchanged keep their ids **and** their
  learner-state columns (`skip`, `pass_or_fail`, `last_check`); new or
  content-edited children take fresh `max+1` allocation; ids of removed children
  are still never reused.
- **Foreign keys:** plain `<parent>_id` columns; `PRAGMA foreign_keys = ON`.
  **Exceptions (no FK, polymorphic/free refs):** `test_cases.owner_id` (→ practice
  or quiz, disambiguated by `owner_type`), `notes.related`, `plans.scope_id`,
  `goals.covered_by` (JSON array). Orphan cleanup on parent delete is the app's job.
- **Booleans:** `INTEGER`, `0`/`1` (e.g. `pass_or_fail`, `skip`, `goals.override`).
- **JSON payloads** (`args`, `kwargs`, `expected`, `last_check`, `tags`,
  `covered_by`, `sections.snippets`) stored as `TEXT` (JSON); queried via SQLite
  JSON1.
- **Section snippets:** ordered `[{id, kind, content}]` where `kind ∈
  {markdown, code}`; `snippets[0].kind` must be `markdown` (app-enforced — can't
  CHECK inside a JSON array). Each snippet renders as one notebook cell.
- **Timestamps:** ISO-8601 UTC strings, suffix `Z`, no fractional seconds
  (`2026-08-09T14:30:00Z`) — lexicographically sortable for `ORDER BY ts`.
- **Ordering:** `ord INTEGER`; rendered/queried `ORDER BY ord`. From authored specs,
  `ord` is the array index (0-based).
- **Checkables:** `practice` and `quizzes` share columns `name, signature, prompt`
  (+ `pass_or_fail`, `last_check`, `skip`) but live in separate tables with
  different parents.

## Slug derivation
`slug = kebab(title)`:
1. lowercase, NFC-normalize;
2. replace every run of `[^a-z0-9]+` with a single `-`;
3. trim leading/trailing `-`;
4. truncate to 60 chars (trim again if it ends mid-run with `-`);
5. on collision with an existing slug within the scope, append `-2`, `-3`, … until
   unique.

A slug with no alphanumerics after step 2 ⇒ `ValidationError` (cannot derive).

A **provided** slug (`spec.slug`) is never derived — it is *validated* against
the same shape (`^[a-z0-9]+(-[a-z0-9]+)*$`, ≤ 60 chars) and rejected otherwise
(adr/017), so the directory name and the DB row can never diverge on a
Unicode-normalizing filesystem.
