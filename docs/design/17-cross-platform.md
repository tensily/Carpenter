# Cross-platform (Linux / macOS)

carpenter targets Linux and macOS only (superseding the former 3-OS scope —
adr/012). The DB-relative layout (`course.db`, `config.json`,
`<course>/lessons/<NN>-<slug>/`) is portable by construction; the OS-specific
surface is **where the app lives on disk**. Decision rationale:
[adr/012](../adr/012-cross-platform-paths.md).

## Per-OS path resolution

| concern | Linux | macOS | owner |
|---|---|---|---|
| config dir | `~/.config/carpenter` | `~/Library/Application Support/carpenter` | `store::config_dir` via `dirs` |
| `bin_dir` default | `~/.local/bin` | `~/.local/bin` | `Config::default` (`dirs::home_dir().join(".local/bin")`) |
| installed binary | `carpenter` | `carpenter` | same name everywhere |
| `$PATH` separator | `:` | `:` | `store::is_on_path` |

`dirs` resolves `config_dir` per-OS; `Config::default` derives `bin_dir` from
`dirs::home_dir()` — no `#[cfg(target_os)]` anywhere in the codebase. The
xdg-root anchor (opencode integration) is `config_dir.parent()`, so it tracks
the OS automatically: `~/.config` Linux, `~/Library/Application Support` macOS.

## Platform assumptions that hold on both targets

- Subprocesses spawn real binaries (`uv`, `curl`, `tar`, checksum tools) — no
  shell/shebang portability concerns.
- Unlinking (or rename-replacing) a running binary is safe — the kernel keeps
  the inode alive (`upgrade`'s copy-replace and `uninstall`'s self-delete
  rely on this; see `strip_deleted` in `core/store.rs`).
- `python3` exists on PATH (both CI lanes install it); the two probe tests
  (compare parity, helper validity) still `return` on spawn failure —
  no-op, not fail.

## Unsupported platforms

Windows and linux-arm64 are out of scope: `install.sh`, `upgrade` release
mode, and `core/release.rs::platform_target` reject them with a clear error
pointing at `cargo xtask build --release`. No `.exe` handling, no `cfg!`
branches, no `.gitattributes` line-ending management exist.

## CI

`.github/workflows/ci.yml` — matrix `os: [ubuntu-latest, macos-14]`, runs the
**full** gate suite (the `AGENTS.md` → Build & test commands): fmt → clippy
(`-D warnings`) → `cargo xtask build` → `cargo test --workspace` → doc.
`rust-toolchain.toml` pins stable; `uv` is installed via `setup-uv`.
`release.yml` publishes branch-governed channels (adr/021): pushes to
`nightly` roll the `nightly` prerelease, pushes to `main` publish stable
`vX.Y.Z` (linux-musl + apple-silicon tarballs), and smoke-tests each published
artifact via the real `curl | sh` one-liner on both lanes.
