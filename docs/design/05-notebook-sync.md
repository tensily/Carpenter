# Notebook render & sync contract

DB → notebook. Layout: skip-config → title → sections → practice → quiz. Every
managed cell tagged `metadata.managed`:

| cell | metadata | sync |
|------|----------|------|
| Lesson title | `managed=title, lesson_id` | regenerated from `lessons.title` |
| Skip config | `managed=skip-config, lesson_id` | regenerated from `lessons.skip` + child `skip` columns; read-only `_skip_config()` the learner/agent may call |
| Section markdown snippet | `managed=section-md, section_id, snippet_id` | regenerated from snippet `content` |
| Section code snippet | `managed=section-code, section_id, snippet_id` | regenerated from snippet `content` |
| Question markdown | `managed=question-md, practice_id` *or* `quiz_id`, `lesson_id` | regenerated from the stub's `prompt` (omitted when the prompt is empty) |
| Practice stub | `managed=practice-stub, practice_id, scaffold_hash` | preserve if learner edited |
| Check cell | `managed=check, target=<owner_type:owner_id>` | regenerated wholesale (outputs stripped) |
| Quiz stub | `managed=quiz-stub, quiz_id, scaffold_hash` | preserve if learner edited |
| Learner cell | (no tag) | always preserved |

Render order: skip-config cell → lesson title → section cells in array order
(md→markdown, code→code; `snippets[0]` is always markdown) → practice question
markdown + stubs + checks → quiz question markdown + stubs + checks. The
question markdown cell carries the full prompt verbatim (markdown allowed) and
is the learner-facing copy; the stub keeps its short `# {prompt}` comment.
`canonical_scaffold`/`scaffold_hash` cover only the stub source, so existing
notebooks never conflict because of the question cell — it is regenerated
wholesale on sync, like any other non-stub managed cell.

## `scaffold_hash`
`scaffold_hash` lives **only in cell metadata**, never in the DB. It is the hash of
the canonical scaffold string (signature + prompt-comment + `raise
NotImplementedError`) carpenter would render for that stub. On sync, carpenter
recomputes the canonical hash and compares against the cell's stored
`metadata.scaffold_hash` (which is the previous render's value). This is also the
discriminator `quiz run` uses to tell scaffolding errors from learner errors (see
[08-quiz-run.md](08-quiz-run.md) step 4).

## Stub preservation (3-way)
On sync, per managed stub cell, with `cur` = canonical scaffold hash and `stored` =
the cell's `metadata.scaffold_hash`:

| `stored` == `cur`? | DB signature changed since last render? | action |
|--------------------|------------------------------------------|--------|
| yes (learner untouched) | no | refresh scaffold from DB |
| yes (learner untouched) | yes | refresh scaffold from DB (DB is source of truth; nothing to preserve) |
| no (learner edited) | no | keep learner source verbatim |
| no (learner edited) | yes | **conflict** — skip + report; `--force` overwrites only the conflicting stub(s) |

`--force` overwrites only the conflicting managed stubs whose `stored != cur`. It
**never** touches untagged learner cells or non-conflicting stubs.

## `conflicts[]`
`lesson sync` returns `{id, synced:true, conflicts:[{id, reason}]}` where `id` is
the practice **or** quiz id (both stub types can conflict — earlier drafts only
listed practice) and `reason` is currently always `db_changed`. A stub is
reported as a conflict only when it is **both** learner-edited **and** the DB
signature changed since the last render; a learner edit with no DB change is
silently preserved (row 3 above), not reported. (`learner_edited` is reserved for
a future surfacing of the preserve case; no code path emits it today.) With no
`--force`, conflicting stubs are left untouched and reported; the agent decides
whether to force.

Agent owns the DB; learner owns filled stubs + own cells.
