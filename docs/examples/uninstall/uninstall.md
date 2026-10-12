**example:**

```sh
carpenter self uninstall --bin-dir ~/.local/bin
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"uninstalled: /home/u/.local/bin/carpenter","data":{"uninstalled":true,"bin":"/home/u/.local/bin/carpenter","skill":[{"app":"opencode","path":"/home/u/.config/opencode/skills/carpenter/SKILL.md","removed":true},{"app":"claude-code","path":"/home/u/.claude/skills/carpenter/SKILL.md","removed":true}],"config_purged":false}}
```

Inverse of `install`: removes the skill of every **registered** app (best-effort — `{"removed":false,"reason":"not_registered"}` when none), then deletes `<bin_dir>/carpenter` (safe while running on Linux/macOS). `NotFound` when neither skill nor binary exists. `--purge-config` also removes the config file; course data is never touched.
