**example:**

```sh
carpenter self install --bin-dir ~/.local/bin
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"installed: /home/u/.local/bin/carpenter","data":{"installed":true,"bin":"/home/u/.local/bin/carpenter","on_path":true}}
```

Default bin dir from config (`bin_dir` → ~/.local/bin).
