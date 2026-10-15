# Data Model

SQLite is the source of truth. One `course.db` per course directory. Schema lives
in `core/db.rs` as one initial `CREATE`-based schema plus idempotent guarded
migrations in `db::migrate` (`solution` column, `ord` resequence +
`idx_lessons_ord`); a versioned `migrate` command remains deferred —
[adr/010](../adr/010-live-check-state.md). These docs are the
human-readable mirror. On conflict, the schema in `core/db.rs` wins — update these
docs to match.

| # | file | topic |
|---|------|-------|
| 01 | [er-diagram.md](01-er-diagram.md) | entity-relationship map |
| 02 | [conventions.md](02-conventions.md) | keys, FKs, JSON payloads, ordering |
| 03 | [ddl.md](03-ddl.md) | full `CREATE TABLE` statements |
| 04 | [status-derivation.md](04-status-derivation.md) | how lesson/goal status is computed |
| 05 | [app-config.md](05-app-config.md) | app-level files (per-OS config dir) |
