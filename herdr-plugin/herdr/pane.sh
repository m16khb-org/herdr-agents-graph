#!/usr/bin/env bash
# Action command: open the graph pane, or close the one this plugin opened.
# The decision (and the pane id it remembers) lives in `agents-graph herdr
# toggle`; an action's output goes to `herdr plugin log`.
set -euo pipefail

bin="${HERDR_PLUGIN_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}/bin/agents-graph"
[ -x "$bin" ] || { echo "agents-graph is not installed at $bin; reinstall the plugin so its build step can download it" >&2; exit 1; }

exec "$bin" herdr toggle "${1:-overlay}"
