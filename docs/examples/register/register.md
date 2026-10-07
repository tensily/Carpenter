**example:**

```sh
carpenter register --app opencode
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"skill registered: opencode","data":{"app":"opencode","path":"/…/opencode/skills/carpenter/SKILL.md","version":"0.12.0","installed":true}}
```

Writes `SKILL.md` + merges the `permission.skill.carpenter="allow"` entry. `--print-skill` prints the rendered bytes instead (no FS change).
