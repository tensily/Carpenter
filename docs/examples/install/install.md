**example:**

```sh
carpenter self install --bin-dir ~/.local/bin
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"installed: /home/u/.local/bin/carpenter","data":{"installed":true,"already_present":false,"bin":"/home/u/.local/bin/carpenter","on_path":true}}
```

Default bin dir from config (`bin_dir` → ~/.local/bin). `already_present` is
true when the target binary existed before this install (re-install).
