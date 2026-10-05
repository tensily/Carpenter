**example:**

```sh
carpenter upgrade --bin-dir ~/.local/bin
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"upgraded: 0.11.0","data":{"upgraded":true,"version":"0.11.0","bin":"/home/u/.local/bin/carpenter","source":"https://github.com/tensily/Carpenter/releases/latest/download/carpenter-x86_64-unknown-linux-musl.tar.gz","skill":[{"refreshed":true,"app":"opencode","path":"/home/u/.config/opencode/skills/carpenter/SKILL.md"},{"refreshed":true,"app":"claude-code","path":"/home/u/.claude/skills/carpenter/SKILL.md"}]}}
```

Fetches the latest **stable** release (checksum-verified), replaces the binary, and
refreshes the skill of every **registered** app (`skill` = one outcome per app;
nothing registered ⇒ `{"refreshed":false,"reason":"not_registered",…}`).
`--channel nightly` follows the rolling prerelease instead; `--source <path>` (or
config `source_dir`) rebuilds from a local checkout. `--no-skill` skips the
skill write (`skill:null`).
