# Specs — Command I/O Contracts

Every command prints exactly **one** JSON envelope on stdout and exits 0 (ok) or 1
(error). The `data` shapes are what an agent parses.

**Generated tables (target state):** every spec **table** is regenerated in-place
between `<!-- BEGIN GENERATED -->` / `<!-- END GENERATED -->` markers by
`xtask gen-specs`, one entry per `*Spec`/`Data` type with a co-located
`models::examples` registry entry ([adr/008](../adr/008-specs-generated-from-types.md)).
The surrounding narrative is always hand-maintained. A file whose types have not
landed yet keeps a hand table (no markers) until its phase ships. Pure-prose files
(no generated region ever): `01-envelope`, `20-helper-contract`.

## Envelope & conventions
- [01-envelope.md](01-envelope.md) — envelope format, error codes, reused shapes, `--force` policy, global flags

## Spec input shapes (authored YAML for `--spec`)
- [02-course-spec.md](02-course-spec.md)
- [03-lesson-spec.md](03-lesson-spec.md)
- [04-plan-spec.md](04-plan-spec.md)
- [05-goal-spec.md](05-goal-spec.md)
- [06-note-spec.md](06-note-spec.md)
- [07-bug-feature-spec.md](07-bug-feature-spec.md)

## Command output contracts
- [08-course.md](08-course.md) · [09-lesson.md](09-lesson.md) · [10-plan.md](10-plan.md) · [11-goal.md](11-goal.md)
- [12-quiz.md](12-quiz.md) · [13-progress.md](13-progress.md) · [14-notes.md](14-notes.md) · [15-bug-feature.md](15-bug-feature.md)
- [16-config.md](16-config.md) · [17-link.md](17-link.md) · [18-build-install-upgrade.md](18-build-install-upgrade.md) · [19-howto.md](19-howto.md)

## Other
- [20-helper-contract.md](20-helper-contract.md) — in-notebook `helper.py` verification contract
- [21-register-deregister.md](21-register-deregister.md) — agent-app skill integration
- [22-venv.md](22-venv.md) — uv-managed course venv
- [23-skip.md](23-skip.md) — top-level `skip` command (sets `skip` columns)
- [24-installs.md](24-installs.md) — discover and remove carpenter installs by tag (adr/027)
