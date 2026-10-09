---
name: OPEN_API_SPEC.md
description: Endpoint, DTO, and OpenAPI documentation gates; read before changing an API contract.
---

# OpenAPI Spec Guidance

## Applicability

This repository exposes no HTTP API: there are no endpoints, controllers, DTOs, or OpenAPI/Swagger files (confirmed by the crate layout in [ARCHITECTURE.md](ARCHITECTURE.md); the zoetrope web app was removed in the fork). The OpenAPI gate below therefore has nothing to check today.

The public contracts that play the same role here, and where their rules live:

| Contract | Consumer | Owner doc |
|---|---|---|
| `agents-graph` CLI flags and `inspect` text output | users, scripts, golden tests | [CONVENTIONS.md](CONVENTIONS.md), [TESTING.md](TESTING.md) |
| `agents-graph herdr resolve` / `herdr toggle` behaviour and exit codes | `herdr-plugin/herdr/*.sh` | [ARCHITECTURE.md](ARCHITECTURE.md), [docs/HERDR-PLUGIN.md](../docs/HERDR-PLUGIN.md) |
| `herdr-plugin.toml` actions and pane ids | herdr, user keybindings | [OPERATIONS.md](OPERATIONS.md) |
| Release archive names and `SHA256SUMS` | `herdr-plugin/herdr/install.sh` | [OPERATIONS.md](OPERATIONS.md) |

Change these with the same care an API change would get: update callers in the same change and note breaking changes in the README and the PR body. There is no `CHANGELOG.md` on `main` yet; `cliff.toml` is configured to generate one from conventional commits.

## Gate order (if an HTTP surface is ever added)

1. Static gate: `issueops api-doc static-check --json`
2. Agent gate prompt/schema: `issueops api-doc review --json`
3. Agent gate evidence: `issueops api-doc review --result FILE --json`
4. Combined gate with evidence: `issueops api-doc check --result FILE --json`

API style for such a surface: `Unknown / not confirmed` — decide it in an ADR when the surface is introduced.
