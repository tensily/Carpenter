# ADR-024: installer consent plan + branded banner

Date: 2026-09-01 · Status: Accepted

## Context

The `curl | sh` one-liner is the primary install path, and trust is the whole
game for a script piped straight into `sh`. Until now the installer's first
output was a single `downloading …` line — the user had no visibility into what
would be touched, and the first consent moment arrived only *after* the binary
was already installed (the per-app skill-registration prompts). The owner also
wanted the install ceremony to carry the brand, Airflow-style, using the
palette of the carpenter talk deck (`Goals/PPT/presentation.html`).

## Decision

`scripts/install.sh` shows what it will do, asks before doing it, and reports
what it did (adr-gated constraint: the `TAG="nightly"` line stays
byte-identical — `release.yml` seds it for stable releases):

- **Banner** — embedded figlet-style `CARPENTER` wordmark (static heredoc, no
  runtime figlet, ≤80 cols) plus a channel-correct tagline derived from `TAG`:
  `carpenter installer · nightly` on the canary script, plain
  `carpenter installer` on the stable release's tag-patched copy. One script,
  two channels, no drift.
- **Install plan** — before any network activity: download source, checksum,
  binary destination, per *detected* app the exact `SKILL.md` path plus the
  `opencode.json` permission merge, and PATH status. App detection moved ahead
  of consent (pure `command -v`, no side effects); register actions stay
  post-install with their per-app `[Y/n]` (Enter registers — registration is
  the encouraged default; only `n` skips).
- **Consent** — interactive (TTY + readable `/dev/tty`) →
  `proceed with the install plan? [Y/n]`; Enter or any non-`n` answer
  proceeds, only an explicit `n` declines — aborting exit 1 before anything is
  downloaded. A dead tty (EOF on the read) also aborts: Enter means "a human
  chose the default", EOF means "nobody is answering". `CARPENTER_INSTALL_YES=1`
  skips the prompt.
  Non-interactive (CI smoke, `curl | sh` automation) prints the plan and
  proceeds — unattended lanes never hang.
- **Palette** — the deck's `:root`: amber `#f0a63f` (wordmark, prompts, plan
  heading), honey `#e9c47c` (channel mark, `!` warnings), cream `#f3e9d6`
  (plan values), muted `#b5a184` (keys, tagline), good `#9dc76f` (`✓`
  summary), bad `#e07856` (errors, abort). Truecolor when `COLORTERM`
  advertises it, xterm-256 approximations otherwise, empty strings when piped
  so CI logs stay plain.
- **Summary** — post-install: `✓ checksum verified`, `✓ installed vX → bin`,
  `! PATH` hint when off-PATH.
- **Tests** — `tests/install_sh.rs`: plan + non-interactive proceed;
  channel-correct banner (applies the release-time sed to a copy, asserts the
  `nightly` mark disappears — this also pins the sed anchor);
  pty decline via `script(1)` (explicit `n` aborts; per-OS syntax — macOS
  `script` does not propagate the child exit status, so assertions ride on
  output + filesystem state) and pty Enter-defaults (Enter proceeds +
  registers). `ci.yml` runs `shellcheck scripts/install.sh` (Linux lane).

## Consequences

+ A user piping the script sees exactly what will be written where, and can
  stop it with one keypress before any network activity.
+ One script serves both channels; the tagline cannot disagree with the tag.
+ Unattended lanes stay hang-free by construction (non-interactive = proceed).
− Tests substring-couple to installer copy — accepted; it is adr/007's
  drift-guard philosophy applied to the installer.
− Two color tiers to keep honest (truecolor + 256 fallback).

## Rejected

- **Preflight fail-fast checks** (writable install dir, existing-binary probe,
  disk space) — YAGNI for a single-binary install; `install -m755` already
  fails cleanly with a permissions hint. The plan is consent, not
  pre-verification.
- **Framed banner variant** (wordmark + rules + boxed plan panel) — the box
  pushed past 76 columns and wrapped on narrow terminals; the compact
  wordmark won in review.
- **Runtime figlet** — one more dependency in the trust path; the art is
  static by design.
