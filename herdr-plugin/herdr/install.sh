#!/usr/bin/env bash
# Build step: put the agents-graph release binary this checkout's manifest
# version names into herdr-plugin/bin/, after checking it against the
# release's SHA256SUMS.
#
# herdr runs build steps with every HERDR_PLUGIN_* variable removed, so the
# install location is derived from this script's own path, never from the
# plugin state directory. Nothing is compiled and nothing lands on PATH: the
# binary lives inside the plugin checkout, so it cannot shadow another tool.
#
# AG_RELEASE_BASE overrides where the release assets are fetched from (any URL
# curl reads, file:// included); it names the directory holding this
# version's archives and SHA256SUMS.
set -euo pipefail

plugin_dir=$(cd "$(dirname "$0")/.." && pwd)
bin="$plugin_dir/bin/agents-graph"

version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$plugin_dir/herdr-plugin.toml" | head -1)
[ -n "$version" ] || { echo "no version in $plugin_dir/herdr-plugin.toml" >&2; exit 1; }

if [ -x "$bin" ] && [ "$("$bin" --version 2>/dev/null)" = "agents-graph $version" ]; then
  echo "agents-graph $version already installed"
  exit 0
fi

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) target=aarch64-apple-darwin ;;
  Darwin-x86_64) target=x86_64-apple-darwin ;;
  Linux-x86_64) target=x86_64-unknown-linux-musl ;;
  Linux-aarch64 | Linux-arm64) target=aarch64-unknown-linux-musl ;;
  *) echo "no agents-graph release for $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac

base="${AG_RELEASE_BASE:-https://github.com/m16khb-org/herdr-agents-graph/releases/download/v$version}"
archive="agents-graph-$target.tar.gz"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

curl -fsSL "$base/$archive" -o "$tmp/$archive"
curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS"

grep -E "^[0-9a-f]{64}[ *]+$archive\$" "$tmp/SHA256SUMS" > "$tmp/expected" || {
  echo "SHA256SUMS has no entry for $archive" >&2
  exit 1
}
if command -v shasum > /dev/null 2>&1; then
  check=(shasum -a 256 -c --status expected)
else
  check=(sha256sum -c --status expected)
fi
(cd "$tmp" && "${check[@]}") || {
  echo "checksum mismatch for $archive; nothing installed" >&2
  exit 1
}

mkdir -p "$tmp/x" "$plugin_dir/bin"
tar -xzf "$tmp/$archive" -C "$tmp/x" agents-graph
install -m 755 "$tmp/x/agents-graph" "$bin.partial"
mv -f "$bin.partial" "$bin"

echo "installed agents-graph $("$bin" --version | cut -d' ' -f2)"
