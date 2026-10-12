**example:**

```sh
carpenter installs remove carpenter-ivan-topic
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"install removed: carpenter-ivan-topic","data":{"removed":true,"tag":"carpenter-ivan-topic","bin":"/home/u/.local/bin/carpenter-ivan-topic","skill":[{"app":"opencode","path":"/home/u/.config/opencode/skills/carpenter-ivan-topic/SKILL.md","removed":true}],"config_purged":false}}
```

Generic uninstall: removes **any** install by tag — the tagged binary, its
skill from every app that registered it, and (with `--purge-config`) its
config file — regardless of which carpenter is running the command. Same
adr/019 semantics as `uninstall` (course data is never touched; `NotFound`
when the tag has neither skill nor binary); the tag must be kebab-case
(`^[a-z0-9]+(-[a-z0-9]+)*$`, max 60).
