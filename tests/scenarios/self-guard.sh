#!/bin/sh
# Scenario: unconfirmed destructive self-ops are refused with zero mutation
# (adr/019 revision). Bare self remove/uninstall/deregister -> Conflict
# envelope; the skill and binary are untouched. Simulates the PR #55 QA probe
# for the "unguarded self remove/uninstall" bug class.

set -eu
. "$(dirname "$0")/lib.sh"

a=$(tagged_bin guard) # tag carpenter-guard

must_ok register "$a" self register
skill="$(config_root)/opencode/skills/carpenter-guard/SKILL.md"
[ -f "$skill" ] || fail "skill not registered at $skill"

must_err remove-no-confirm Conflict \
    "$a" self remove carpenter-guard --bin-dir "$SHOME/bin"
must_err uninstall-no-confirm Conflict \
    "$a" self uninstall --bin-dir "$SHOME/bin"
must_err deregister-no-confirm Conflict \
    "$a" self deregister

[ -f "$skill" ] || fail "skill mutated without confirmation"
[ -f "$SHOME/bin/carpenter_guard" ] || fail "binary mutated without confirmation"

echo "self-guard: ok"
