# ADR-012: Cross-platform path resolution (Linux/macOS)

Date: 2026-08-11 · Status: Accepted (rescoped 2026-08-18: Windows dropped —
supported platforms are Linux + macOS only; see the Rescope section)

## Context
carpenter ran on Linux. Two facts forced an explicit cross-platform decision:

1. **`config_dir` was already portable** via the `dirs` crate (`dirs::config_dir()` →
   `~/.config` Linux, `~/Library/Application Support` macOS), but
   the rest of the path surface was Unix-shaped:
    - `Config::default` `bin_dir` fallback was the literal `/usr/local/bin`
      ([adr/004](004-build-install-split.md) assumed a Unix `~/.local/bin`).
    - `dirs::executable_dir()` returns `None` on macOS, so a pure-`dirs`
      `bin_dir` default cannot derive it — a home-dir join is required.
2. **CI did not exist.** The quality gates (`fmt`, `clippy -D warnings`, `cargo test
   --workspace`, `cargo doc`) ran only locally.

The question: where does "which OS am I on" live, and how do paths stay correct without
scattering `#[cfg]` across the codebase?

## Decision
1. **`dirs` is the only platform surface**: `store::config_dir` wraps
   `dirs::config_dir()`; `Config::default` derives `bin_dir` from
   `dirs::home_dir().join(".local/bin")` (the `/usr/local/bin` literal is
   deleted). No `core/platform.rs`, no `#[cfg(target_os)]`, no `.exe`
   handling — both supported targets name the binary `carpenter` and share
   `~/.local/bin` conventions and the `:` PATH separator.
2. **CI: a GitHub Actions matrix over `ubuntu`/`macos` runs the full gate
   suite** (the same commands in `AGENTS.md` → Build & test), with `uv`
   installed (the `uv_is_available_in_this_env` test asserts it) and a
   committed `rust-toolchain.toml` pin for reproducibility.

## Rescope (2026-08-18): Windows dropped
The originally-planned Windows support (a `core/platform.rs` with
`exe_file_name`, `%LOCALAPPDATA%` bin dir, `;` PATH separator, `.gitattributes`
`eol=lf`, windows CI lane) never shipped and is now formally out of scope —
[design/17](../design/17-cross-platform.md) documents the two-OS reality.
`install.sh` / `upgrade` / `platform_target` reject unsupported platforms with
a clear error. Unlink/rename of a running binary (upgrade, uninstall
self-delete) is relied upon — valid on Linux and macOS only.

## Consequences
+ Zero platform `#[cfg]` in `src/`; `dirs` is the single chokepoint for per-OS
  behavior.
+ The DB is the source of truth regardless of OS — `course.db`, `config.json`,
  and the lesson layout (`<course>/lessons/<NN>-<slug>/`) are relative paths;
  `pathlib`'s `/` in `helper.py` resolves per-OS for free. `helper.py` has no
  shebang, no `os.symlink`, no `chmod` — already portable.
+ Generated spec **tables** and howto **example envelopes** stay
  Linux-illustrative (e.g. `~/.local/bin/carpenter`); they are generated from
  types/`docs/examples/`, not per-OS, and the per-OS truth lives here +
  [design/17](../design/17-cross-platform.md).
− Unsupported platforms (Windows, linux-arm64, Intel macOS) must build from
  source; `upgrade` release mode refuses them explicitly.

Work sequence: [Phase 12 — cross-platform](../design/14-build-order.md).
Design detail: [design/17](../design/17-cross-platform.md).
