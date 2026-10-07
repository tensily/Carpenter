# `register` / `deregister` — output contracts

Integrate carpenter with an agent app by writing/removing a global `SKILL.md`
(see [design/15-opencode-integration.md](../design/15-opencode-integration.md)).
Global scope only.

<!-- BEGIN GENERATED -->
| cmd | input | `data` (ok) |
|-----|-------|-------------|
| `register [--app opencode]` | --app (default `opencode`) | `{"app":"opencode","path":"~/.config/opencode/skills/carpenter/SKILL.md","version":"0.12.0","installed":true}` — writes `SKILL.md` + merges `"skill":{"carpenter":"allow"}`; `agents` ⇒ `ValidationError` |
| `register --app claude-code` | --app | `{"app":"claude-code","path":"~/.claude/skills/carpenter/SKILL.md","version":"0.12.0","installed":true}` — writes `SKILL.md` into `~/.claude/skills/` (auto-discovered — no permission merge) |
| `register --print-skill` | --app | `{"skill":"…"}` — prints the rendered `SKILL.md` bytes; no filesystem change |
| `deregister [--app opencode]` | --app (default `opencode`) | `{"app":"opencode","path":"~/.config/opencode/skills/carpenter/SKILL.md","removed":true}` — removes `SKILL.md` (+ dir if empty) + the `carpenter` allow key (apps that have one); `NotFound` if absent |
<!-- END GENERATED -->

`--app` values: `opencode` and `claude-code` (both write the same rendered
`SKILL.md` — opencode into `~/.config/opencode/skills/` + an allow entry,
claude-code into `~/.claude/skills/` with no permission merge), `agents` ⇒
`ValidationError` ("not yet supported"). The `--app` selector defaults to
`opencode` — it is never an interactive TTY prompt (an agent cannot answer one;
see [01-envelope.md](01-envelope.md) `--force` policy).

The `SKILL.md` body is assembled from typed fields in `core/skill.rs` — no
template ([adr/009](../adr/009-skill-assembled-from-fields.md)). `register
--print-skill` emits the rendered bytes; a determinism + frontmatter-validation
test gates it.
