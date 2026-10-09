---
name: 2026-10-09-fork-import-pr-failed-commit-lint-on-upstream-history
description: Caution record for a solved false case or recurring risk.
---

# Fork-import PR failed commit lint on upstream history

- Date: 2026-10-09
- Kind: `caution`
- Source: project-bootstrap enrichment 2026-10-09
- Summary: The first PR after importing zoetrope failed Lint commits because the PR range included upstream ci: and merge commits, and crate-ci/committed@master no longer resolved.
- Context: PR #2 CI on 2026-10-08: committed reported disallowed types build/ci and a non-conventional GitHub merge commit (6ae8a05), all from zoetrope's history except one; the action reference @master failed to resolve.
- Resolution: Commit 114e4f9: pin crate-ci/committed@v1.1.11, allow build and ci, and lint from b1f31dd when the base lacks it. Verify locally with committed b1f31dd..HEAD before pushing a PR that brings in new upstream history.
- Evidence:
  - CI run 37836547590 (Lint commits failed)
  - CI run 37860875516 (all jobs passed)
  - commit 114e4f9
