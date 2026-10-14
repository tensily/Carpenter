#!/bin/sh
# Scenario: install identity derives from the binary stem, installs coexist
# in one list, and self install reports already_present on re-install.
# Simulates the PR #55 QA probes for install identity and the
# "install re-run is indistinguishable" observability gap.

set -eu
. "$(dirname "$0")/lib.sh"

v=$(tagged_bin validate) # tag carpenter-validate

env=$(must_ok install-fresh "$v" self install --bin-dir "$SHOME/bin")
assert_has fresh-target '"already_present":false' "$env"

env=$(must_ok reinstall "$v" self install --bin-dir "$SHOME/bin")
assert_has existing-target '"already_present":true' "$env"

must_ok register "$v" self register

list=$(must_ok list "$v" self list --bin-dir "$SHOME/bin")
assert_has installed-binary '"tag":"carpenter"' "$list"
assert_has validate-tag '"tag":"carpenter-validate"' "$list"

echo "install-identity: ok"
