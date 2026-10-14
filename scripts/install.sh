#!/bin/sh
# carpenter installer — fetches the `nightly` release binary for this platform.
#
# Devs/canary users:
#   curl -LsSf https://github.com/tensily/Carpenter/releases/download/nightly/install.sh | sh
#
# Channels (adr/021): this stock script follows `nightly`; the stable release
# attaches a tag-patched copy (TAG="vX.Y.Z"), and /releases/latest/download/
# serves the newest stable one. End users get stable via the README one-liner;
# nightly is the dev/canary path documented in docs/README.md.
#
# UX (adr/024): prints a branded banner and an install plan (download source,
# checksum, binary destination, skill registration, PATH status), then asks
# for confirmation when a TTY is attached. Non-interactive runs (CI, `curl |
# sh` automation) print the plan and proceed so unattended lanes never hang.
# Decoration colors are TTY-gated; piped output stays plain.
#
# Test overrides (used by local verification, harmless to ignore):
#   CARPENTER_DOWNLOAD_BASE  base URL replacing the GitHub release dir
#   CARPENTER_INSTALL_DIR    destination dir (default ~/.local/bin)
#   CARPENTER_INSTALL_YES=1  skip the interactive confirmation prompt
set -eu

REPO="tensily/Carpenter"
TAG="nightly"
BASE="${CARPENTER_DOWNLOAD_BASE:-https://github.com/${REPO}/releases/download/${TAG}}"
INSTALL_DIR="${CARPENTER_INSTALL_DIR:-${HOME}/.local/bin}"

# Command name by channel (adr/027): only the stable release (a tag-patched
# copy, TAG="vX.Y.Z") keeps `carpenter`; nightly installs as `carpenter_nightly`
# so the two coexist. The skill/data identity stays kebab (`carpenter-nightly`)
# — agent-app skill names forbid `_`.
BIN_NAME="carpenter"
SKILL_TAG="carpenter"
if [ "$TAG" = nightly ]; then
    BIN_NAME="carpenter_nightly"
    SKILL_TAG="carpenter-nightly"
fi

say() { printf '%scarpenter:%s %s\n' "$AMBER" "$RESET" "$1"; }
die() { printf '%scarpenter: error:%s %s\n' "$BAD" "$RESET" "$1" >&2; exit 1; }
abort_install() {
    printf '%scarpenter: aborted%s — nothing was downloaded or installed\n' "$BAD" "$RESET" >&2
    exit 1
}

# TTY-gated decoration (adr/024): empty off-TTY, so piped/CI output is plain.
# Palette = the carpenter deck (`Goals/PPT/presentation.html` :root): amber
# #f0a63f, honey #e9c47c, cream #f3e9d6, muted #b5a184, good #9dc76f,
# bad #e07856. Truecolor where advertised, xterm-256 approximations otherwise.
BOLD='' AMBER='' HONEY='' CREAM='' MUTED='' GOOD='' BAD='' RESET=''
if [ -t 1 ]; then
    BOLD=$(printf '\033[1m')
    RESET=$(printf '\033[0m')
    if [ "${COLORTERM:-}" = truecolor ] || [ "${COLORTERM:-}" = 24bit ]; then
        AMBER=$(printf '\033[38;2;240;166;63m')
        HONEY=$(printf '\033[38;2;233;196;124m')
        CREAM=$(printf '\033[38;2;243;233;214m')
        MUTED=$(printf '\033[38;2;181;161;132m')
        GOOD=$(printf '\033[38;2;157;199;111m')
        BAD=$(printf '\033[38;2;224;120;86m')
    else
        AMBER=$(printf '\033[38;5;215m')
        HONEY=$(printf '\033[38;5;222m')
        CREAM=$(printf '\033[38;5;230m')
        MUTED=$(printf '\033[38;5;144m')
        GOOD=$(printf '\033[38;5;150m')
        BAD=$(printf '\033[38;5;209m')
    fi
fi

os=$(uname -s)
arch=$(uname -m)

case "$os:$arch" in
    Linux:x86_64)
        target=x86_64-unknown-linux-musl
        checksum="sha256sum" ;;
    Darwin:arm64)
        target=aarch64-apple-darwin
        checksum="shasum -a 256" ;;
    Darwin:x86_64)
        die "Intel Mac builds are not published; build from source: cargo xtask build --release" ;;
    *)
        die "unsupported platform: ${os} ${arch} (published: Linux x86_64, macOS Apple Silicon)" ;;
esac

tarball="carpenter-${target}.tar.gz"

# Detect opencode up front so the plan can show exactly what registration
# would touch; the register action itself runs after the binary lands.
opencode_detected=no
if command -v opencode >/dev/null 2>&1; then
    opencode_detected=yes
fi

skill_path() { # the SKILL.md the installer would write
    printf '%s/opencode/skills/%s/SKILL.md' "${XDG_CONFIG_HOME:-${HOME}/.config}" "$SKILL_TAG"
}

on_path() {
    case ":${PATH}:" in
        *":${INSTALL_DIR}:") return 0 ;;
    esac
    return 1
}

# --- banner (channel-correct: the nightly canary says so, stable is unmarked)
printf '%s' "$AMBER"
cat <<'WORDMARK'
  ____       _      ____    ____    _____   _   _   _____   _____   ____
 / ___|     / \    |  _ \  |  _ \  | ____| | \ | | |_   _| | ____| |  _ \
| |        / _ \   | |_) | | |_) | |  _|   |  \| |   | |   |  _|   | |_) |
| |___    / ___ \  |  _ <  |  __/  | |___  | |\  |   | |   | |___  |  _ <
 \____|  /_/   \_\ |_| \_\ |_|     |_____| |_| \_|   |_|   |_____| |_| \_\
WORDMARK
printf '%s' "$RESET"
if [ "$TAG" = nightly ]; then
    tagline="carpenter installer" ; chan="  ·  nightly"
else
    tagline="carpenter installer" ; chan=""
fi
pad=$(( (74 - ${#tagline} - ${#chan}) / 2 ))
printf '%*s%s%s%s%s%s\n\n' "$pad" '' "$MUTED" "$tagline" "$HONEY" "$chan" "$RESET"

# --- the install plan: everything this script is about to touch -------------
printf '%sinstall plan%s\n' "$BOLD$AMBER" "$RESET"
printf '  %sdownload%s  %s%s%s\n' "$MUTED" "$RESET" "$CREAM" "$tarball" "$RESET"
printf '      %sfrom%s  %s%s%s\n' "$MUTED" "$RESET" "$CREAM" "$BASE" "$RESET"
printf '  %sverify%s    %sSHA256SUMS checksum (%s)%s\n' \
    "$MUTED" "$RESET" "$CREAM" "$checksum" "$RESET"
printf '  %sinstall%s   %s%s/%s%s\n' \
    "$MUTED" "$RESET" "$CREAM" "$INSTALL_DIR" "$BIN_NAME" "$RESET"
if [ "$opencode_detected" = yes ]; then
    printf '  %sregister%s  %s%s%s -> %s%s%s\n' \
        "$MUTED" "$RESET" "$CREAM" opencode "$RESET" "$CREAM" "$(skill_path)" "$RESET"
    printf '             %s+ permission "skill.%s": allow in opencode.json%s\n' "$CREAM" "$SKILL_TAG" "$RESET"
else
    printf '  %sregister%s  %s(no agent app detected; later: carpenter self register)%s\n' \
        "$MUTED" "$RESET" "$CREAM" "$RESET"
fi
if on_path; then
    printf '  %spath%s      %s%s is on your PATH%s\n' \
        "$MUTED" "$RESET" "$CREAM" "$INSTALL_DIR" "$RESET"
else
    # shellcheck disable=SC2016  # $PATH prints literally — it's the hint's text
    printf '  %spath%s      %s%s is not on your PATH — after install: export PATH="%s:$PATH"%s\n' \
        "$MUTED" "$RESET" "$CREAM" "$INSTALL_DIR" "$INSTALL_DIR" "$RESET"
fi
printf '\n'

# Interactive (a TTY is attached): prompts read /dev/tty — stdin carries the
# piped script under `curl | sh`. Non-interactive (CI, `curl | sh` under
# automation): proceed with the printed plan so unattended lanes never hang.
interactive=no
if { [ -t 1 ] || [ -t 2 ]; } && [ -r /dev/tty ]; then
    interactive=yes
fi

if [ "$interactive" = yes ] && [ "${CARPENTER_INSTALL_YES:-}" != "1" ]; then
    printf '%scarpenter:%s proceed with the install plan? [Y/n] ' "$AMBER" "$RESET" >&2
    read -r answer </dev/tty || abort_install
    case "$answer" in
        n|N|no|NO|No) abort_install ;;
    esac
elif [ "$interactive" = no ]; then
    say "non-interactive — proceeding with the plan above"
fi

case "$BASE" in
    file://*) ;;
    *) say "downloading ${tarball} from ${BASE}" ;;
esac

tmp=$(mktemp -d) || die "mktemp failed"
trap 'rm -rf "$tmp"' EXIT

curl -fsSL -o "${tmp}/${tarball}" "${BASE}/${tarball}" \
    || die "download failed: ${BASE}/${tarball}"
curl -fsSL -o "${tmp}/SHA256SUMS" "${BASE}/SHA256SUMS" \
    || die "download failed: ${BASE}/SHA256SUMS"

# shellcheck disable=SC2086
(cd "$tmp" && grep " ${tarball}\$" SHA256SUMS | $checksum -c - >/dev/null) \
    || die "checksum mismatch for ${tarball}"

tar -xzf "${tmp}/${tarball}" -C "$tmp" || die "extract failed: ${tarball}"
[ -f "${tmp}/carpenter" ] || die "tarball does not contain a 'carpenter' binary"

mkdir -p "$INSTALL_DIR" || die "cannot create ${INSTALL_DIR}"
install -m755 "${tmp}/carpenter" "${INSTALL_DIR}/${BIN_NAME}" \
    || die "cannot write ${INSTALL_DIR}/${BIN_NAME} (permissions?)"

# Register the skill with opencode. Interactive (a TTY is attached): ask y/n.
# Non-interactive (CI, `curl | sh` under automation): auto-register so
# unattended lanes never hang. Prompts read /dev/tty — stdin carries the piped
# script under `curl | sh`.
ask_register() { # Y/n from the user (Enter registers)
    printf '%scarpenter:%s register the skill for opencode? [Y/n] ' "$AMBER" "$RESET" >&2
    read -r answer </dev/tty || return 1
    case "$answer" in
        n|N|no|NO|No) return 1 ;;
        *) return 0 ;;
    esac
}

do_register() {
    if "${INSTALL_DIR}/${BIN_NAME}" self register >/dev/null 2>&1; then
        say "registered in opencode (skill)"
    else
        printf 'carpenter: note: register failed; run manually: %s self register\n' \
            "${INSTALL_DIR}/${BIN_NAME}" >&2
    fi
}

if [ "$opencode_detected" = yes ]; then
    if [ "$interactive" = "yes" ]; then
        if ask_register; then
            do_register
        else
            say "skipped opencode (register later: carpenter self register)"
        fi
    else
        say "opencode detected — registering carpenter skill (non-interactive)"
        do_register
    fi
else
    printf 'carpenter: note: no agent app detected; register manually with: carpenter self register\n' >&2
fi

# --- summary: what just landed on this machine -------------------------------
printf '\n'
printf '%s✓%s %schecksum verified (%s)%s\n' "$GOOD" "$RESET" "$CREAM" "$checksum" "$RESET"
ver=$("${INSTALL_DIR}/${BIN_NAME}" --version 2>/dev/null || echo carpenter)
printf '%s✓%s %sinstalled %s -> %s/%s%s\n' \
    "$GOOD" "$RESET" "$CREAM" "$ver" "$INSTALL_DIR" "$BIN_NAME" "$RESET"
if ! on_path; then
    # shellcheck disable=SC2016  # $PATH prints literally — it's the hint's text
    printf '%s!%s %s%s is not on PATH; add it: export PATH="%s:$PATH"%s\n' \
        "$HONEY" "$RESET" "$CREAM" "$INSTALL_DIR" "$INSTALL_DIR" "$RESET"
fi
