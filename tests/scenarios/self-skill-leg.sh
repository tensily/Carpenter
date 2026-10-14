#!/bin/sh
# Scenario: the confirm path (--yes) removes exactly the targeted tag's legs,
# the skill is restorable via register, removing tag A leaves tag B intact,
# unknown tags are NotFound, and data-dir-only tags are listed AND removable
# with the data dir kept (list/remove parity). Simulates the PR #55 QA probes
# for the "machine-global skill leg" and "list/remove parity" bugs.

set -eu
. "$(dirname "$0")/lib.sh"

a=$(tagged_bin skill_a) # tag carpenter-skill-a
b=$(tagged_bin skill_b) # tag carpenter-skill-b

must_ok register-a "$a" self register
must_ok register-b "$b" self register
skills="$(config_root)/opencode/skills"
[ -f "$skills/carpenter-skill-a/SKILL.md" ] || fail "skill a missing"
[ -f "$skills/carpenter-skill-b/SKILL.md" ] || fail "skill b missing"

must_ok remove-a "$a" self remove carpenter-skill-a --bin-dir "$SHOME/bin" --yes
[ ! -e "$skills/carpenter-skill-a" ] || fail "skill a not removed"
[ -f "$skills/carpenter-skill-b/SKILL.md" ] || fail "tag B skill clobbered by A's removal"
[ -f "$SHOME/bin/carpenter_skill_b" ] || fail "tag B binary clobbered by A's removal"

# recovery: removal deleted the binary too — reinstall the copy, then
# register restores skill + permission key (adr/019 recovery path)
cp "$CARPENTER" "$SHOME/bin/carpenter_skill_a"
must_ok restore-a "$a" self register
[ -f "$skills/carpenter-skill-a/SKILL.md" ] || fail "skill a not restorable via register"

mkdir -p "$(config_root)/carpenter-stale"
list=$(must_ok list "$a" self list --bin-dir "$SHOME/bin")
assert_has stale-listed '"tag":"carpenter-stale"' "$list"

must_err unknown-tag NotFound \
    "$a" self remove carpenter-nonexistent --bin-dir "$SHOME/bin" --yes

env=$(must_ok stale-remove "$a" self remove carpenter-stale --bin-dir "$SHOME/bin" --yes)
assert_has stale-removed '"removed":true' "$env"
assert_has stale-no-bin '"bin":null' "$env"
[ -d "$(config_root)/carpenter-stale" ] || fail "data dir must be kept (adr/019)"

echo "self-skill-leg: ok"
