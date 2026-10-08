#!/usr/bin/env bash
# Pane command: resolve the session of the pane this was opened from and draw
# it. Every failure is printed here and held until you press enter, since this
# terminal is what the user is looking at.
set -euo pipefail

bin="${HERDR_PLUGIN_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}/bin/agents-graph"

die() {
  printf '\n  %s\n\n' "$*" >&2
  read -r -p '  press enter to close ' _ || true
  exit 1
}

[ -x "$bin" ] || die "agents-graph is not installed at $bin; reinstall the plugin so its build step can download it"

target=$("$bin" herdr resolve 2>&1) || die "$target"

# "<provider> <id-or-path>": the provider never contains a space, so splitting
# on the first one keeps a path with spaces intact. The agent is running, so
# ride the live edge; space and the scrubber go back.
"$bin" --provider "${target%% *}" --follow "${target#* }" \
  || die "agents-graph exited with status $? (--provider ${target%% *} --follow ${target#* })"
