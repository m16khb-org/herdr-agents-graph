---
name: overview
description: Family module overview: installation and runtime operation.
---

# Operations — Overview

Canonical index: [OPERATIONS.md](../../OPERATIONS.md)

Source of truth for the plugin: [docs/HERDR-PLUGIN.md](../../../docs/HERDR-PLUGIN.md). Note: `src/ui` is being redesigned in [issue #6](https://github.com/m16khb-org/herdr-agents-graph/issues/6); this doc describes `main` at `1c744d2`.

## Local build and run

Commands from `src/main.rs` USAGE (the `--help` text):

| Command | Effect |
|---|---|
| `cargo run --` | follow the current project's live session |
| `cargo run -- <file.jsonl>` | replay a recording from the start |
| `cargo run -- <id>` | replay a session by id or unique prefix |
| `cargo run -- <dir>` | follow another project's live session |
| `cargo run -- <file> --follow` | follow a file's live edge instead of replaying |
| `cargo run -- <file> --speed N` | playback speed (default 8.0; must be positive) |
| `cargo run -- --provider <claude\|codex\|omp> ...` | force format instead of detecting |
| `cargo run -- inspect <file\|id>` | headless: print session tree + info |
| `cargo run -- herdr resolve` / `herdr toggle [p]` | plugin bridge subcommands (used by the plugin scripts) |
| `cargo run -- --version` / `--help` | print and exit |

Release build: `cargo build --release --locked` (what cd.yml runs). `scripts/idle-cpu.sh <session.jsonl>` measures idle CPU of `--follow` against `target/release/agents-graph` (needs a built release binary; `script` pty).

## Environment variables actually read

| Variable | Read in | Purpose |
|---|---|---|
| `HOME` | provider discovery, `src/herdr.rs`, keys.sh | locate session stores / config |
| `CODEX_HOME` | `src/provider/codex/discovery.rs` | Codex session root override |
| `PI_CODING_AGENT_SESSION_DIR`, `PI_CODING_AGENT_DIR` | `src/provider/omp/discovery.rs` | omp session dir overrides |
| `AGENTS_GRAPH_PANE` | `src/herdr.rs` (`TARGET_PANE_ENV`) | pane the graph targets |
| `HERDR_BIN_PATH` | `src/herdr.rs`, keys.sh | herdr binary (default `herdr`) |
| `HERDR_PLUGIN_CONTEXT_JSON`, `HERDR_PLUGIN_STATE_DIR`, `HERDR_PLUGIN_ID` | `src/herdr.rs`, keys.sh | injected by herdr into plugin commands |
| `HERDR_PLUGIN_ROOT` | open.sh, pane.sh | plugin checkout path |
| `HERDR_CONFIG_PATH`, `XDG_CONFIG_HOME` | keys.sh | herdr config location (then `~/.config/herdr/config.toml`) |
| `AG_RELEASE_BASE` | install.sh | release asset directory URL (any curl URL, `file://` ok) |
| `AG_BIN` | scripts/idle-cpu.sh | binary override |
| `UPDATE_GOLDEN` | `src/provider/harness.rs` | tests: rewrite golden `assets/**/*.model.txt` / `.timeline.txt` |
| `AG_REAL_SESSIONS`, `AG_REAL_SESSIONS_ROOT` | `tests/real_sessions.rs` | opt-in test over real local sessions |

Never record values of these in docs or logs; names and purposes only.

## herdr plugin install

```
herdr plugin install m16khb-org/herdr-agents-graph/herdr-plugin
```

- `[[build]]` runs `bash herdr/install.sh`: reads `version` from `herdr-plugin.toml`, maps `uname` to one of `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, downloads `agents-graph-<target>.tar.gz` and `SHA256SUMS` from `https://github.com/m16khb-org/herdr-agents-graph/releases/download/v<version>` (override with `AG_RELEASE_BASE`), verifies the checksum, and installs to `herdr-plugin/bin/agents-graph`. It skips when `bin/agents-graph --version` already equals `agents-graph <version>`. Nothing is put on PATH.
- `herdr plugin link <path>` runs no build; place a binary at `herdr-plugin/bin/agents-graph` yourself (HERDR-PLUGIN.md, "plugin link").
- Keybinding: run action `setup-keys` (`herdr/keys.sh setup`) to bind `prefix+shift+z` to the overlay (split/tab alternates written as comments). It writes one marker-fenced block into the herdr config, backs up to `<config>.agents-graph-backup`, validates with `herdr config check`, restores on failure, skips keys already bound. `remove-keys` deletes the block. Action output goes to `herdr plugin log`.
- Actions: `open` (overlay), `open-split`, `open-tab`, `setup-keys`, `remove-keys`; each open action toggles.

## Release procedure

1. Make `version` equal in `Cargo.toml` and `herdr-plugin/herdr-plugin.toml`.
2. Push a tag `v<version>`. `.github/workflows/cd.yml` (trigger: tags `v*`) first checks the tag against both manifests and fails on disagreement.
3. `build` job matrix (`cargo build --release --locked`; `cross` for musl): `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`. Unix archives are `.tar.gz`, Windows `.zip`, each with LICENSE and NOTICE.
4. `release` job writes `SHA256SUMS` over `agents-graph-*` and runs `gh release create` with generated notes.

Note: the plugin installer has no Windows target. CI (`ci.yml`) gates: fmt, clippy `-D warnings`, tests on ubuntu/macos/windows, portable-core check (`--no-default-features`), docs, `cargo publish --dry-run`, MSRV, typos, commit lint.

## Rollback

`herdr plugin install` accepts `--ref` (docs/research/report.md, line 60; herdr's CLI reference): `herdr plugin install m16khb-org/herdr-agents-graph/herdr-plugin --ref v<older>`; reinstall is also the update mechanism (no `plugin update`). Because the installer downloads the release matching the manifest version of the checked-out ref, an older tag fetches the older binary. Unknown / not confirmed: end-to-end behavior of `--ref` with an already-installed plugin; verify with `herdr plugin list` (shows pinned commit). Removing the keys: `remove-keys` action.

## Smoke checks

- `herdr-plugin/bin/agents-graph --version` (or `target/release/agents-graph --version`) prints `agents-graph <version>`.
- Headless on fixtures: `cargo run -- inspect assets/claude/demo.jsonl` (fixtures also under `assets/codex/`, `assets/omp/`; use `--provider` if detection is ambiguous). Not run while drafting; verify before relying.
- `herdr config check` passes after `setup-keys`.
- In herdr: focus an agent pane, press `prefix+shift+z`.

## Codex hooks caution

New Codex sessions can stop at "Hooks need review" after herdr updates its Codex hook; until trusted, herdr reports no `agent_session` and the plugin says no session for the Codex pane. Fix: trust hooks once in Codex or rerun `herdr integration install codex`. Trusting is a user decision. Full record: [caution](../../cautions/2026-10-09-codex-holds-a-new-session-at-hooks-need-review-until-herdr-s.md). Related: [pane-context caution](../../cautions/2026-10-09-a-herdr-plugin-pane-s-own-context-names-the-focus-at-pane-st.md).

## Environment and secrets

- Do not put raw env values, credentials, or local state in docs or logs. The only token in the repo workflows is `secrets.GITHUB_TOKEN` in cd.yml.

## Project docs upkeep

- `issueops project bootstrap --repo . --json` creates docs; `--sync` refreshes from evidence.
- Keep docs current via MCP `project_docs_route` → `project_docs_read` → `project_docs_revise`; append resolved cases to CAUTIONS/ADR with `project_docs_append`.
- `ISSUEOPS_DISABLE_HOOKS=1` turns issueops context hooks into no-ops.
