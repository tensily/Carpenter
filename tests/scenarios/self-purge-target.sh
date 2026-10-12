#!/bin/sh
# Scenario: --purge-config purges the removed tag's config — never the
# calling install's — reports config_purged faithfully, and keeps-by-default
# on the absent-config path. Simulates the PR #55 QA probe for the
# "--purge-config silent no-op / wrong target" bug.

set -eu
. "$(dirname "$0")/lib.sh"

a=$(tagged_bin purge_a) # caller tag carpenter-purge-a
caller_cfg="$(config_root)/carpenter-purge-a/config.json"
target_cfg="$(config_root)/carpenter-purge-b/config.json"
mkdir -p "$(dirname "$caller_cfg")" "$(dirname "$target_cfg")"
printf '{"bin_dir":"/nonexistent"}\n' > "$caller_cfg"
printf '{"bin_dir":"/nonexistent"}\n' > "$target_cfg"

env=$(must_ok purge "$a" self remove carpenter-purge-b --purge-config --yes)
assert_has purge-flag '"config_purged":true' "$env"
[ ! -f "$target_cfg" ] || fail "target tag's config not purged"
[ -f "$caller_cfg" ] || fail "caller's config clobbered by the target's purge"

env=$(must_ok keep "$a" self remove carpenter-purge-b --yes)
assert_has kept-default '"config_purged":false' "$env"
[ -d "$(config_root)/carpenter-purge-b" ] || fail "data dir must be kept (adr/019)"

echo "self-purge-target: ok"
