#!/usr/bin/env bash
# Re-vendor the SEED Design rootage tokens at one commit of daangn/seed-design.
#
#   scripts/sync-seed.sh <full-git-sha>
#
# Fetches the YAML token files plus LICENSE and NOTICE byte-for-byte into
# design/seed/. Update design/seed/SOURCE.md (commit, date, checksums) by hand
# afterwards, then regenerate src/ui/seed/tokens.rs.
set -euo pipefail

sha=${1:?usage: scripts/sync-seed.sh <full-git-sha>}
root=$(cd "$(dirname "$0")/.." && pwd)
dest=$root/design/seed
base=https://raw.githubusercontent.com/daangn/seed-design/$sha

mkdir -p "$dest"
for name in color duration timing-function dimension radius font-weight; do
  curl -fsSL "$base/packages/rootage/$name.yaml" -o "$dest/$name.yaml"
done
curl -fsSL "$base/LICENSE" -o "$dest/LICENSE"
curl -fsSL "$base/NOTICE" -o "$dest/NOTICE"

echo "vendored daangn/seed-design@$sha into design/seed/"
echo "next: UPDATE_SEED=1 cargo test --locked seed_tokens"
