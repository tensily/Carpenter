# lib.sh — shared helpers for tests/scenarios/*.sh (sourced, never executed).
#
# Contract (enforced by the runner, tests/scenarios.rs): each script gets a
# hermetic scratch env — CARPENTER_BIN (the built binary under test),
# SCENARIO_HOME (per-test scratch dir), HOME/XDG_CONFIG_HOME pinned inside it —
# and a per-script timeout (a hang fails the test). A script honors that env:
# every byte it writes goes under SCENARIO_HOME, the binary under test is
# always $CARPENTER_BIN, and nothing outside the scratch is touched — no real
# HOME, no real skills, no real config.

set -eu

CARPENTER="${CARPENTER_BIN:?runner must set CARPENTER_BIN}"
SHOME="${SCENARIO_HOME:?runner must set SCENARIO_HOME}"

fail() {
    echo "FAIL: $*"
    exit 1
}

# A validate-tag copy: install identity (config dir, skill name, permission
# key, data dir) derives from the binary stem, so a renamed copy is a fully
# isolated install. Prints the copy's path.
tagged_bin() { # $1 = stem suffix, e.g. guard -> carpenter_guard (tag carpenter-guard)
    [ -n "${1:-}" ] || fail "tagged_bin: missing stem suffix"
    mkdir -p "$SHOME/bin"
    tb_dest="$SHOME/bin/carpenter_$1"
    cp "$CARPENTER" "$tb_dest"
    printf '%s\n' "$tb_dest"
}

# The config base a carpenter process resolves under the runner's sandboxed
# env: dirs::config_dir() honors XDG_CONFIG_HOME on Linux; on macOS it is
# $HOME/Library/Application Support regardless.
config_root() {
    case "$(uname -s)" in
        Darwin) printf '%s/Library/Application Support\n' "$HOME" ;;
        *) printf '%s\n' "${XDG_CONFIG_HOME:-$HOME/.config}" ;;
    esac
}

# Run a command; print its envelope to stdout (capture with $(…)); assert the
# envelope is ok. stderr (e.g. confirmation plans) is dropped.
must_ok() { # $1 = label, rest = command
    mo_label=$1
    shift
    mo_out=$("$@") || fail "$mo_label: non-ok exit: ${mo_out:-<no envelope>}"
    case "$mo_out" in
        *'"status":"ok"'*) printf '%s\n' "$mo_out" ;;
        *) fail "$mo_label: expected ok envelope, got: $mo_out" ;;
    esac
}

# Run a command expected to fail; assert the exact envelope code.
must_err() { # $1 = label, $2 = expected code, rest = command
    me_label=$1
    me_want=$2
    shift 2
    me_out=$("$@") && fail "$me_label: expected $me_want, got ok: $me_out"
    case "$me_out" in
        *"\"code\":\"$me_want\""*) printf '%s\n' "$me_out" ;;
        *) fail "$me_label: expected $me_want, got: $me_out" ;;
    esac
}

# Assert a substring is present in captured output.
assert_has() { # $1 = label, $2 = needle, $3 = haystack
    case "$3" in
        *"$2"*) ;;
        *) fail "$1: missing '$2' in: $3" ;;
    esac
}
