**example:**

```sh
carpenter installs list
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"installs listed","data":{"installs":[{"tag":"carpenter","bin":"/home/u/.local/bin/carpenter","skill_apps":["opencode"],"data_dir":"/home/u/.config/carpenter"},{"tag":"carpenter-ivan-topic","bin":"/home/u/.local/bin/carpenter-ivan-topic","skill_apps":["opencode"],"data_dir":"/home/u/.config/carpenter-ivan-topic"}]}}
```

Discovers every carpenter install on this machine, keyed by its **tag** (the
binary's file stem — adr/027): `carpenter` is the stock install; a branch
install carries its branch name. The union of `carpenter*` binaries in
`bin_dir`, registered skill dirs under either agent app, and data dirs under
the config base; sorted by tag. A row's `data_dir` is the path the tag derives
to even when it does not exist yet, and `bin` is `null` when only a skill or
data dir remains.
