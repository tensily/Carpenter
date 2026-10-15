# Overview

Agent-driven CLI that builds Python/Jupyter learning material. **Rust binary,
SQLite store.** SQLite is the source of truth; notebooks render from it. No
embedded LLM — an external agent (opencode) is the tutor; carpenter is
deterministic storage + rendering + execution.

## Domain model

```
Course
 ├─ Plan (course)           bullet goals -> linked lessons
 └─ Lesson                  = one rendered notebook; ordered roadmap
     ├─ Plan (lesson)
     ├─ Section             teaching (markdown + code snippets -> cells)
     │   └─ Practice        fill-in function (the "practice session")
     │       └─ TestCase*
     └─ Quiz                assessment function (end of notebook)
         └─ TestCase*
```

Practice and Quiz share a `Checkable` shape (`name, signature, prompt` + cases);
separate tables, different ownership. No shared abstraction unless duplication
proves real.

## Status derivation (bottom-up)
- Lesson `complete` ← all its **non-skipped** practice + quiz have `pass_or_fail=1`
  (set by the helper on each check). `skipped` ← `lessons.skip=1`. No manual
  override; skip is the only escape hatch.
- Course-goal `achieved` ← its `covered_by` lessons are `complete`.
- `goal update --status` pins a goal (sets `override=1`); `--status derived` clears it.
- Live state only — there is no `attempts` history (adr/010). See
  [data-model/04](../data-model/04-status-derivation.md).

## IDs
Stable strings assigned by carpenter (monotonic per-table `id_seq` counter,
never reused).
- Course/lesson: slug (`data-structures`, `arrays-101`); lesson dir = `NN-slug`.
- Section `s1..`, Practice `p1..`, Quiz `q1..`, TestCase `c1..`, Note `n1..`,
  Plan `pl1..`, Goal `g1..` (globally unique within their table).

## Non-goals (YAGNI)

_(merged from the former `13-out-of-scope.md`)_

Projects / multi-course aggregation; generic event log (derive from
notes+timestamps); speculative artifact base / descriptor paths; manual
hand-prose (use scraped howto); cross-course views; capstone entity;
concurrency/locking; non-Python runtimes. Revisit only on a concrete need.

(Note: attempt *history* was explicitly removed, not merely deferred — see
[adr/010](../adr/010-live-check-state.md). Live state lives in
`pass_or_fail`/`last_check`.)
