#!/usr/bin/env bash
# Install a branch's carpenter as a branch-tagged local install.
#
#   scripts/install-branch.sh              # current branch: build + install + register
#   scripts/install-branch.sh ivan/topic   # any branch: built in a temp git worktree
#   scripts/install-branch.sh --uninstall  # remove the tagged install
#
# The branch name is the install tag (adr/027): `ivan/foo` becomes the binary
# `~/.local/bin/carpenter-ivan-foo`, the data dir `~/.config/carpenter-ivan-foo`
# (macOS: `~/Library/Application Support/…`), and the agent-app skill
# `carpenter-ivan-foo` whose SKILL.md embeds the tagged binary's path. Every
# surface is derived from the binary stem by the CLI itself, so the tagged
# install is isolated in every way from the stock `carpenter` — and installing
# from `main` produces the stock, untagged install (the one exception).
#
# A branch argument never touches your checkout: the branch is materialized in
# a temporary `git worktree`, built there, and the worktree removed after.
#
# Dev tool — not shipped by the installer. `carpenter-<branch> upgrade` must
# never run on a tagged install (it would pull stable bits under a branch tag).
set -euo pipefail

say() { printf 'carpenter-branch: %s\n' "$1"; }
die() { printf 'carpenter-branch: error: %s\n' "$1" >&2; exit 1; }

REPO_ROOT=$(git rev-parse --show-toplevel)

MODE="${1:-}"
UNINSTALL=no
TARGET_BRANCH=""
case "$MODE" in
    --uninstall) UNINSTALL=yes ;;
    "") TARGET_BRANCH=$(git -C "$REPO_ROOT" branch --show-current) ;;
    *) TARGET_BRANCH=$MODE ;;
esac
[ -n "$TARGET_BRANCH" ] || die "no branch given and the checkout is detached"

# Sanitize to the CLI's kebab-case install tag (slugify semantics).
TAG=$(printf '%s' "$TARGET_BRANCH" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//')
[ -n "$TAG" ] || die "branch '$TARGET_BRANCH' sanitizes to no install tag"

# Command name (adr/027): only main keeps `carpenter`; branch installs use the
# underscored form (`carpenter_ivan_topic`). The skill/data identity stays the
# kebab tag — agent-app skill names forbid `_`.
BIN_NAME="carpenter"
[ "$TAG" = "main" ] || BIN_NAME="carpenter_${TAG//[-]/_}"
BIN_DIR="${HOME}/.local/bin"
BIN="${BIN_DIR}/${BIN_NAME}"

detect_apps() {
    command -v opencode >/dev/null 2>&1 && printf '%s\n' opencode
}

if [ "$UNINSTALL" = yes ]; then
    [ -x "$BIN" ] || die "no tagged install at $BIN"
    say "uninstalling $BIN_NAME (binary, skill, data dir)"
    # --yes: the script itself is the consent (the human passed --uninstall);
    # the CLI's plan/confirm gate is for bare invocations
    "$BIN" self uninstall --yes || say "note: CLI uninstall reported an error; continuing"
    data_dir="${XDG_CONFIG_HOME:-${HOME}/.config}/$TAG"
    [ "$(uname -s)" = Darwin ] && data_dir="${HOME}/Library/Application Support/$TAG"
    if [ -d "$data_dir" ]; then
        if [ -t 0 ]; then
            printf 'carpenter-branch: purge data dir %s? [y/N] ' "$data_dir"
            read -r answer
            case "$answer" in
                y|Y|yes) rm -rf "$data_dir" && say "purged $data_dir" ;;
                *) say "kept $data_dir" ;;
            esac
        else
            say "non-interactive — kept $data_dir (remove manually if wanted)"
        fi
    fi
    exit 0
fi

# Resolve the build tree: the checkout itself for the current branch, else a
# temporary worktree (local ref preferred, then origin/<branch>).
CURRENT=$(git -C "$REPO_ROOT" branch --show-current)
SRC_DIR="$REPO_ROOT"
WORKTREE=""
cleanup() { [ -z "$WORKTREE" ] || git -C "$REPO_ROOT" worktree remove --force "$WORKTREE"; }
trap cleanup EXIT
if [ "$TARGET_BRANCH" != "$CURRENT" ]; then
    if git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$TARGET_BRANCH"; then
        REF=$TARGET_BRANCH
    elif git -C "$REPO_ROOT" show-ref --verify --quiet "refs/remotes/origin/$TARGET_BRANCH"; then
        REF=origin/$TARGET_BRANCH
    else
        die "no branch '$TARGET_BRANCH' locally or on origin"
    fi
    WORKTREE=$(mktemp -d)/wt
    git -C "$REPO_ROOT" worktree add --detach "$WORKTREE" "$REF" >/dev/null \
        || die "cannot create worktree for $REF"
    SRC_DIR=$WORKTREE
    say "building branch $TARGET_BRANCH ($REF) in a temp worktree — your checkout is untouched"
fi

say "branch $TARGET_BRANCH -> install tag $BIN_NAME"
say "building (cargo xtask build --release — gen-howto + gen-specs + the ship build)"
(cd "$SRC_DIR" && cargo xtask build --release) || die "build failed"
[ -f "$SRC_DIR/target/release/carpenter" ] || die "build did not produce target/release/carpenter"

mkdir -p "$BIN_DIR"
install -m755 "$SRC_DIR/target/release/carpenter" "$BIN" || die "cannot write $BIN"

data_dir="${XDG_CONFIG_HOME:-${HOME}/.config}/$TAG"
[ "$(uname -s)" = Darwin ] && data_dir="${HOME}/Library/Application Support/$TAG"

apps=$(detect_apps)
if [ -n "$apps" ]; then
    if "$BIN" self register >/dev/null 2>&1; then
        say "registered skill $BIN_NAME in $apps"
    else
        say "note: register failed; run manually: $BIN self register"
    fi
else
    say "no agent app detected (later: $BIN self register)"
fi

say "installed $BIN"
say "data dir: $data_dir (isolated from the stock install)"
say "caveat: never run '$BIN_NAME self upgrade' — it would pull stable bits under this branch tag"
say "remove with: scripts/install-branch.sh --uninstall  (or: carpenter self remove $TAG --yes)"
