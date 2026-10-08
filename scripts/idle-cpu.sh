#!/usr/bin/env bash
# Idle CPU of `agents-graph <file> --follow` drawing into a 200x60 pty: the CPU
# time it uses between 5 s and 35 s after start, as a percentage of those 30 s.
#
#   scripts/idle-cpu.sh <session.jsonl>      prints idle_cpu_percent=<value>
#
# AG_BIN overrides the binary (default target/release/agents-graph).
set -euo pipefail

file=${1:?usage: scripts/idle-cpu.sh <session.jsonl>}
bin=${AG_BIN:-target/release/agents-graph}
[ -x "$bin" ] || { echo "no binary at $bin (cargo build --release)" >&2; exit 1; }

# `script` gives the TUI a real terminal; stdin from /dev/null keeps a job
# control shell from stopping it in the background.
script -q /dev/null bash -c "stty rows 60 cols 200; exec '$bin' '$file' --follow" \
  < /dev/null > /dev/null 2>&1 &
launcher=$!
pid=
trap 'kill ${pid:-} $launcher 2>/dev/null || true' EXIT

for _ in $(seq 50); do
  pid=$(pgrep -n -x -f "$bin $file --follow" || true)
  [ -n "$pid" ] && break
  sleep 0.1
done
[ -n "$pid" ] || { echo "agents-graph did not start" >&2; exit 1; }

# `ps -o time=` prints [[hh:]mm:]ss.ss of CPU time.
cpu_seconds() {
  /bin/ps -o time= -p "$1" | awk -F: '{ s = 0; for (i = 1; i <= NF; i++) s = s * 60 + $i; print s }'
}

sleep 5
t0=$(cpu_seconds "$pid")
sleep 30
t1=$(cpu_seconds "$pid")

echo "pid=$pid cpu_at_5s=$t0 cpu_at_35s=$t1"
awk -v a="$t0" -v b="$t1" 'BEGIN { printf "idle_cpu_percent=%.2f\n", (b - a) / 30 * 100 }'
