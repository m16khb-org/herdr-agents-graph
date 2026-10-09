---
name: 2026-10-09-the-terminal-ui-draws-only-from-pinned-seed-tokens-and-state
description: Accepted decision record with rationale, alternatives, and consequences.
---

# The terminal UI draws only from pinned SEED tokens, and state never imports ui

- Date: 2026-10-09
- Kind: `adr`
- Source: issue #6 (io-4b513b64dc69)
- Summary: Colours come from SEED rootage tokens pinned in design/seed and compiled into src/ui/seed/tokens.rs; the theme is owned by the frontend loop and passed to ui::draw; src/state holds view state as plain data and does not import src/ui.
- Context: Issue #6 replaced the zoetrope-derived UI (per-file colour literals, a chip tray and panel imported by state) with Now/Lanes/Graph views on SEED Design tokens (daangn/seed-design@22b68ce0, rootage-artifacts 3.0.2, Apache-2.0).
- Decision: (1) design/seed holds the upstream YAML, LICENSE, NOTICE and SOURCE.md byte-for-byte; a test-only generator (src/ui/seed/gen.rs, yaml-rust2 dev-dependency) regenerates tokens.rs and fails when it drifts (UPDATE_SEED=1 rewrites). No YAML at runtime. (2) The brand role uses SEED's purple at the same steps SEED uses carrot; no Daangn logo, name or character. (3) Colours only through ui::seed::theme::Theme; no Color:: literal outside src/ui/seed (gate G3). Snapshot goldens in assets/ui name runs by token, not RGB. (4) Theme is decided once in tui::run and passed into ui::draw; it is not an App field, so src/state has no ui import. Graph node/edge data live in state/graph.rs; their rataflow renderers live in ui/views/graph.rs and recover the theme from the palette. (5) src/ui reads no clock: wall time is Timeline::now_reference, monotonic time is App.clock (gate G16).
- Consequences: Updating SEED is scripts/sync-seed.sh <commit> then UPDATE_SEED=1 cargo test --locked seed_tokens, and the snapshot goldens may need UPDATE_GOLDEN=1. Release archives carry LICENSE-APACHE-SEED and NOTICE-SEED. A new widget or view must take &Theme and use tokens; G3 and G16 fail otherwise.
- Evidence:
  - design/seed/SOURCE.md
  - src/ui/seed/{gen.rs,tokens.rs,theme.rs,snapshot.rs}
  - src/ui/mod.rs draw(frame, app, theme); src/tui.rs Theme::detect
  - grep -rn 'use crate::ui' src/state → 0
  - NOTICE, .github/workflows/cd.yml Package steps
- Alternatives / rejected options:
  - Recolour the existing zoetrope UI with SEED colours: keeps the structure the issue asked to replace.
  - Parse YAML at runtime: adds a runtime dependency and start-up work; the generated module is checked in instead.
  - Store Theme in App: makes state import ui, which the redesign set out to remove.
  - Use SEED's carrot brand colour: SEED's NOTICE reserves Daangn's brand resources; purple was chosen by the user.
