**example:**

```sh
carpenter self uninstall --bin-dir ~/.local/bin --yes
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"uninstalled: /home/u/.local/bin/carpenter","data":{"uninstalled":true,"bin":"/home/u/.local/bin/carpenter","skill":[{"app":"opencode","path":"/home/u/.config/opencode/skills/carpenter/SKILL.md","removed":true}],"config_purged":false}}
```

Inverse of `install`: without `--yes`, prints a removal plan (binary, skill
path, config, kept data dir) to stderr and confirms on stdin — decline
or EOF ⇒ `Conflict`, nothing removed. Confirmed, it removes the skill if
**registered** (best-effort — `{"removed":false,"reason":"not_registered"}`
when not), then deletes `<bin_dir>/carpenter` (safe while running on
Linux/macOS). `NotFound` only when no skill, binary, **or** data dir exists.
`--purge-config` also removes the running tag's config file; course data is
never touched. The skill is machine-global per tag — `--bin-dir` scopes only the
binary.
