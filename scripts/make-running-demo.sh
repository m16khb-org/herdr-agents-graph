#!/usr/bin/env bash
# Write a copy of assets/claude/demo.jsonl whose main agent is Running: one
# extra assistant line dated 40 s ago. An interactive main stays Running for
# 120 s after its last activity, so the copy has to be made right before it is
# measured (scripts/idle-cpu.sh), not reused.
#
#   scripts/make-running-demo.sh <out.jsonl>                 a text line
#   scripts/make-running-demo.sh --pending-tool <out.jsonl>  a tool call with
#       no result yet, so a timer ticks on screen every second
set -euo pipefail

pending=false
if [ "${1:-}" = "--pending-tool" ]; then
  pending=true
  shift
fi
out=${1:?usage: scripts/make-running-demo.sh [--pending-tool] <out.jsonl>}
here=$(cd "$(dirname "$0")/.." && pwd)

if ts=$(date -u -v-40S +%Y-%m-%dT%H:%M:%S.000Z 2>/dev/null); then :; else
  ts=$(date -u -d '40 seconds ago' +%Y-%m-%dT%H:%M:%S.000Z)
fi

if $pending; then
  content='[{"type":"tool_use","id":"toolu_pending","name":"Bash","input":{"command":"cargo test --locked","description":"Run the test suite"}}]'
else
  content='[{"type":"text","text":"Still checking the build."}]'
fi

cp "$here/assets/claude/demo.jsonl" "$out"
printf '%s\n' "{\"type\":\"assistant\",\"uuid\":\"a8\",\"parentUuid\":\"a7\",\"sessionId\":\"demo\",\"timestamp\":\"$ts\",\"message\":{\"role\":\"assistant\",\"model\":\"claude-opus-4-8\",\"content\":$content,\"usage\":{\"input_tokens\":3400,\"output_tokens\":12}}}" >> "$out"
