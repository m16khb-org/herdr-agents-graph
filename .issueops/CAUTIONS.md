---
name: CAUTIONS.md
description: Recurring mistakes and operational pitfalls; read before a risky change or when a failure repeats.
---

# Cautions

Known risks and incident lessons. This root is the family index; focused detail lives under [cautions/](cautions/).

- [Cautions overview](cautions/overview.md) — standing risks that apply to most changes.
- Dated records in [cautions/](cautions/) (`YYYY-MM-DD-<slug>.md`), one incident each. Read the ones whose title matches the area you touch: herdr plugin panes, Codex hook trust, omp record shapes, CI commit lint, release tags.

## Appending knowledge

- Append new dated records with MCP `project_docs_append` or `issueops project append`.
- Records are written as one file per record inside the module directory, so this index stays small.
- Revise this index only to add links to new curated modules; keep it within the manifest line budget.
