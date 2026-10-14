# ADR-006: integrate with agent apps via a SKILL.md (not custom tools)

Date: 2026-08-08 · Status: Accepted

## Context
carpenter must be discoverable by an external agent (opencode). Two opencode
integration paths exist: generate
custom-tool `.ts` files (`.opencode/tools/`) that shell out per command, or drop a
single `SKILL.md` into the app's skills directory. The agent already has a `bash`
tool and carpenter already self-documents via `carpenter howto`.

## Decision
Integrate via a **`SKILL.md`** written to the app's global skills dir
(`~/.config/opencode/skills/carpenter/SKILL.md`). `register` writes it (plus an
`allow` permission entry); `deregister` removes it. The skill body is lean and
defers to `carpenter howto` for the live command surface — it never duplicates the
command list.

## Consequences
+ One file to generate/track, not ~40 per-command tool files — far less codegen
  and no per-command Zod/schema drift.
+ The command surface has a single source (`howto`, scraped from `clap`); the skill
  is just the activation entry point. DRY across docs and integration.
+ One anchor to resolve (the XDG root) and one permission store to merge — the
  integration stays a single `register`/`deregister` pair.
− The agent calls carpenter through `bash` (running `carpenter <cmd>`), so it does
  not get typed per-command tool args; it relies on `howto` + `docs/specs/` for
  argument shapes. Acceptable: validation stays in carpenter either way.

## Update (2026-10-14)

opencode is the only supported agent app. Registration is flag-free
(`carpenter self register`): the skill path and the `opencode.json` permission
entry both derive from the XDG root. `upgrade`/`uninstall` touch the skill only
when it is registered (skill file present). See
[design/15](../design/15-opencode-integration.md).
