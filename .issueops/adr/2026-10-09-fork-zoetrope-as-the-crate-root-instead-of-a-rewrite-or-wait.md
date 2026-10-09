---
name: 2026-10-09-fork-zoetrope-as-the-crate-root-instead-of-a-rewrite-or-wait
description: Accepted decision record with rationale, alternatives, and consequences.
---

# Fork zoetrope as the crate root instead of a rewrite or waiting for upstream

- Date: 2026-10-09
- Kind: `adr`
- Source: project-bootstrap enrichment 2026-10-09
- Summary: herdr-agents-graph imports furkankly/zoetrope@b1f31dd with history as its crate root, renames it, and adds the omp provider from upstream PR #26.
- Context: zoetrope (MIT) read only Claude Code and Codex; its herdr bridge rejected omp panes. Upstream PR #26 (omp/pi provider) had been open without maintainer review since 2026-09-13 and the maintainer had no commits after 2026-09-11.
- Decision: Import zoetrope history with git merge --allow-unrelated-histories, rename the package to herdr-agents-graph / lib agents_graph / bin agents-graph, drop zoetrope-only assets and the web app, keep LICENSE and attribution in NOTICE, cherry-pick PR #26 and adapt it to omp 18.8.4.
- Consequences: Upstream history is part of main; commit lint starts from the fork point b1f31dd; merges use merge commits so upstream authorship survives. The UI layer inherited from zoetrope was replaced in issue #6 (PR #8, v0.2.0); the graph layout and timeline remain zoetrope's.
- Evidence:
  - docs/research/report.md §4.5
  - NOTICE
  - git log 195dbb8 ef49e61 4ddfee4
  - PR #2 (merged as 1c744d2)
- Alternatives / rejected options:
  - Wait for upstream to merge PR #26: no maintainer activity, no schedule.
  - Rewrite from scratch (Rust or Bun): reimplements roughly 4,500 lines of provider/fact/state plus the rataflow layout and discards verified Claude/Codex parsers.
  - Contribute omp to upstream only: still blocked on the same review.
