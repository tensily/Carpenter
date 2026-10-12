**example:**

```sh
carpenter self deregister --app opencode --yes
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"skill deregistered: opencode","data":{"app":"opencode","path":"/…/opencode/skills/carpenter/SKILL.md","removed":true}}
```

Removes `SKILL.md` (+ dir if empty) and the allow key. NotFound if absent.
Without `--yes` a plan (the machine-global skill path the app reads) is
printed to stderr and confirmed on stdin — decline or EOF ⇒ `Conflict`,
nothing removed.
