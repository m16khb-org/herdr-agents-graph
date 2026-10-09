---
name: ADR.md
description: Accepted structural decisions and their rationale; read before reversing or extending a design choice.
---

# Architecture Decision Records

Accepted architecture decisions and roadmap. This root is the family index; focused detail lives under [adr/](adr/).

- [Architecture Decision Records overview](adr/overview.md) — when to read, when to append, entry template.
- Dated records in [adr/](adr/) (`YYYY-MM-DD-<slug>.md`). Read before reversing: the zoetrope fork, the release and install path, the read and redraw strategy, and the SEED design-token layer.

## Appending knowledge

- Append new dated records with MCP `project_docs_append` or `issueops project append`.
- Records are written as one file per record inside the module directory, so this index stays small.
- Revise this index only to add links to new curated modules; keep it within the manifest line budget.
