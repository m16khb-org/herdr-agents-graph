---
name: OPERATIONS.md
description: Install, runtime, and operating procedures; read before running, deploying, or troubleshooting.
---

# Operations

Installation and runtime operation of `agents-graph` (the binary) and its herdr plugin. This root is the family index; focused detail lives under [operations/guides/](operations/guides/).

- [Operations overview](operations/guides/overview.md): local run, plugin install, env vars, release, rollback, smoke checks.

Repo-authored sources that win over this summary: [docs/HERDR-PLUGIN.md](../docs/HERDR-PLUGIN.md), [herdr-plugin/README.md](../herdr-plugin/README.md), [docs/DISCOVERY.md](../docs/DISCOVERY.md), [README.md](../README.md), [.github/workflows/cd.yml](../.github/workflows/cd.yml).

## Quick facts

- Binary: `agents-graph`; release `v0.1.0` tag exists on origin at `1c744d2`.
- Plugin id `m16khb.herdr-agents-graph`, needs herdr >= 0.9.3, platforms macos/linux.
- Release = push a `v*` tag; version in tag, `Cargo.toml`, and `herdr-plugin/herdr-plugin.toml` must agree.
- Known operational risk: [Codex hook trust](cautions/2026-10-09-codex-holds-a-new-session-at-hooks-need-review-until-herdr-s.md) (see overview).

## Appending knowledge

- Append new dated records with MCP `project_docs_append` or `issueops project append`.
- Records are written as one file per record inside the module directory, so this index stays small.
- Revise this index only to add links to new curated modules; keep it within the manifest line budget.
