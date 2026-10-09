---
name: 2026-10-09-omp-usage-cost-is-an-object-in-real-sessions-but-a-number-in
description: Caution record for a solved false case or recurring risk.
---

# omp usage.cost is an object in real sessions but a number in the fixture

- Date: 2026-10-09
- Kind: `caution`
- Source: project-bootstrap enrichment 2026-10-09
- Summary: Real omp 18.x sessions write usage.cost as {input, output, cacheRead, cacheWrite, total}; the anonymised fixture uses a scalar, and a strict typed field would make omp's parse_line drop the whole assistant line.
- Context: Found in the issue #6 plan review: 7,145 object-shaped usage.cost values across 200 local omp sessions, zero scalars; src/provider/omp/wire.rs parses each line with serde_json::from_str(line).ok(), so one mismatched field discards tool calls, tokens and model from that line, and tests/real_sessions.rs would still pass.
- Resolution: Read cost from usage.cost.total via serde_json::Value with the scalar as a fallback, keep tests for the real shape inline (not in assets/omp, which would change goldens), and check real-session tool-call counts before and after any omp wire change.
- Evidence:
  - src/provider/omp/wire.rs:182-190,392
  - issue #6 plan review round 1
  - tests/real_sessions.rs failure criteria
