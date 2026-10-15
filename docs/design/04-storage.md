# Storage layout

```
~/.config/carpenter/
├── config.json               # app defaults (bin_dir, python, timeout_secs, active_course?, source_dir?)
├── bug/<id>.json             # {id,ts,title,description,repro,stack?,status,resolved_ts?}
└── feature_request/<id>.json # same shape (rationale instead of repro)

<root>/courses/<slug>/        # <root> = --root flag, else cwd
├── course.json               # course definition (single doc)
├── course.db                 # SQLite: all tables (source of truth)
├── pyproject.toml            # uv project (base deps — see specs/22) — created by `venv create`
├── uv.lock                   # uv lockfile
├── .venv/                    # course venv (uv)
└── lessons/<NN-slug>/
    ├── lesson.ipynb          # rendered
    └── helper.py             # generated, per lesson
```

Paths are per-OS ([adr/012](../adr/012-cross-platform-paths.md),
[design/17](17-cross-platform.md)): `~/.config/carpenter` is the Linux config dir
(macOS `~/Library/Application Support/carpenter`), resolved by `dirs`
(`store::config_dir`); the `bin_dir` default comes from `Config::default`
(`dirs::home_dir()`), and the installed-binary name follows the install tag
(`store::binary_file_name` — adr/027; the stock binary is `carpenter` on both
OSes). The layout below the config dir is identical on every OS.

Schema: see [data-model/](../data-model/). Contracts: see [specs/](../specs/).
