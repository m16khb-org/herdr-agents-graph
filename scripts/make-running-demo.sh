#!/usr/bin/env bash
# Write a copy of assets/claude/demo.jsonl whose main agent is Running: one
# extra assistant line dated 40 s ago. An interactive main stays Running for
# 120 s after its last activity, so the copy has to be made right before it is
# measured (scripts/idle-cpu.sh), not reused.
set -euo pipefail

out=${1:?usage: scripts/make-running-demo.sh <out.jsonl>}
here=$(cd "$(dirname "$0")/.." && pwd)

if ts=$(date -u -v-40S +%Y-%m-%dT%H:%M:%S.000Z 2>/dev/null); then :; else
  ts=$(date -u -d '40 seconds ago' +%Y-%m-%dT%H:%M:%S.000Z)
fi

cp "$here/assets/claude/demo.jsonl" "$out"
printf '%s\n' "{\"type\":\"assistant\",\"uuid\":\"a8\",\"parentUuid\":\"a7\",\"sessionId\":\"demo\",\"timestamp\":\"$ts\",\"message\":{\"role\":\"assistant\",\"model\":\"claude-opus-4-8\",\"content\":[{\"type\":\"text\",\"text\":\"Still checking the build.\"}],\"usage\":{\"input_tokens\":3400,\"output_tokens\":12}}}" >> "$out"
