---
name: 2026-10-09-merge-pull-requests-with-a-merge-commit-and-lint-commits-fro
description: Accepted decision record with rationale, alternatives, and consequences.
---

# Merge pull requests with a merge commit and lint commits from the fork point

- Date: 2026-10-09
- Kind: `adr`
- Source: project-bootstrap enrichment 2026-10-09
- Summary: PRs are merged with merge commits, and the commit-lint job checks from upstream tip b1f31dd while a PR base does not contain it.
- Context: The fork-import PR carried zoetrope's own ci: and merge commits, which failed this repo's committed policy; squashing would erase upstream authorship; crate-ci/committed@master stopped resolving.
- Decision: Pin crate-ci/committed to v1.1.11, allow build and ci types, lint b1f31dd..head when the base lacks b1f31dd and base..head otherwise, and merge with a merge commit.
- Consequences: Later PRs lint base..head normally because main contains b1f31dd.
- Evidence:
  - commit 114e4f9
  - .github/workflows/ci.yml (commits job)
  - committed.toml
  - merge commit 1c744d2
- Alternatives / rejected options:
  - Rewrite upstream history to pass lint: falsifies imported history.
  - Squash merge: loses upstream commits and authorship.
  - Ignore commits by author: blunt and hides real violations.
