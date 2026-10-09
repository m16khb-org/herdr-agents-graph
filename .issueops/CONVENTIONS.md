---
name: CONVENTIONS.md
description: Coding conventions and layer boundaries; read before writing or restructuring code.
---

# Conventions

Implementation and interface conventions. This root is the family index; focused detail lives under [conventions/](conventions/).

- [Conventions overview](conventions/overview.md): toolchain gates (rustfmt, clippy, typos, MSRV, features), module doc style, the provider extension rule, fact naming, error handling, test and fixture rules.
- [COMMIT_POLICY.md](COMMIT_POLICY.md): commit format (enforced by `committed.toml`) and the Lore body practice.

Layering and the provider/feeder/core boundaries are owned by [ARCHITECTURE.md](ARCHITECTURE.md) and the repo docs it links ([docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md), [docs/DISCOVERY.md](../docs/DISCOVERY.md)); this family does not restate them.

## Appending knowledge

- Append new dated records with MCP `project_docs_append` or `issueops project append`.
- Records are written as one file per record inside the module directory, so this index stays small.
- Revise this index only to add links to new curated modules; keep it within the manifest line budget.
