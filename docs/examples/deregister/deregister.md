**example:**

```sh
carpenter self deregister --app opencode
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"skill deregistered: opencode","data":{"app":"opencode","path":"/…/opencode/skills/carpenter/SKILL.md","removed":true}}
```

Removes `SKILL.md` (+ dir if empty) and the allow key. NotFound if absent.
