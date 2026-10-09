---
name: overview
description: Family module overview: known risks and incident lessons.
---

# Cautions — Overview

Canonical index: [CAUTIONS.md](../CAUTIONS.md)

Standing risks for most changes. Incident-specific lessons are the dated records in this directory.

- **Session data is private.** Real transcripts under `~/.claude`, `~/.codex`, `~/.omp` may be read to learn record shapes, never quoted into fixtures, docs, tests, logs, or issue/PR text. New fixtures go under `assets/<provider>/` with invented ids, paths, and text.
- **Agent formats drift without notice.** Claude Code adds record types between versions, Codex changed `session_meta.source` from string to object between 0.101 and 0.120, and omp writes `usage.cost` as an object while the anonymised fixture uses a number. Parsers must ignore unknown types and tolerate unexpected field shapes without dropping the rest of the record. Evidence: [docs/research/report.md](../../docs/research/report.md) §3, `src/provider/*/wire.rs`.
- **Goldens hide behaviour changes.** A refactor that makes `assets/**/*.model.txt` or `*.timeline.txt` differ is a behaviour change until reviewed; never regenerate with `UPDATE_GOLDEN=1` just to make CI pass.
- **Time-dependent UI must keep redrawing.** The TUI draws only when the frame would change; any new on-screen element that changes with wall time (elapsed counters, fades, live edges) must be part of the redraw decision, or the screen looks frozen.
- **Generated docs are drafts.** Verify weak evidence in `.issueops` against code before relying on it.
- **Do not commit secrets, credentials, local state, or generated artifacts** (`target/`, `herdr-plugin/bin/`).
- **CI covers ubuntu, macOS, and Windows.** Local macOS runs miss path-separator and terminal differences; compare with the CI matrix before calling a change done.
