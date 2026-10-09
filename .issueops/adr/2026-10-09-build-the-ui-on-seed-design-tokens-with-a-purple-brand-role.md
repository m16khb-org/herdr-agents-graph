---
name: 2026-10-09-build-the-ui-on-seed-design-tokens-with-a-purple-brand-role
description: Accepted decision record with rationale, alternatives, and consequences.
---

# Build the UI on SEED Design tokens with a purple brand role

- Date: 2026-10-09
- Kind: `adr`
- Source: project-bootstrap enrichment 2026-10-09
- Summary: The redesigned UI (issue #6) uses SEED Design rootage tokens pinned at rootage-artifacts 3.0.2 and maps SEED's brand role to the purple palette instead of carrot.
- Context: The UI layer copied from zoetrope hard-codes colors per file. The user asked to adopt 당근 SEED Design. SEED's NOTICE treats brand resources (logo, name, characters, and what identifies Daangn) as trademarks separate from the Apache-2.0 code license.
- Decision: Vendor the rootage YAML with LICENSE and NOTICE under design/seed, generate Rust tokens in a test (UPDATE_SEED=1), map brand to the same steps of purple (blue is SEED's informative tone), ship the Apache-2.0 copy and SEED NOTICE in release archives, and never use Daangn logo, name or carrot as a role. Status: accepted by the user on 2026-10-09; implementation in progress on branch 6-seed-ui-redesign.
- Consequences: Colors in src/ui outside the token module are a defect. Updating SEED means re-vendoring at a new commit and regenerating tokens.
- Evidence:
  - GitHub issue #6 and its plan review
  - https://github.com/daangn/seed-design/tree/22b68ce014c894edf286a2328a96250a36ab33a2/packages/rootage
  - npm @seed-design/rootage-artifacts 3.0.2
  - user decision in conversation 2026-10-09
- Alternatives / rejected options:
  - Carrot as the accent: risks reading as a Daangn product under the brand clause; rejected by the user.
  - Recolor the existing zoetrope UI only: leaves the copied structure the user wanted replaced.
  - Parse YAML at runtime: adds a dependency and startup cost; generated, committed tokens instead.
