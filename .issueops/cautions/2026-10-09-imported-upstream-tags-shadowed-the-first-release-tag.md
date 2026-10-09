---
name: 2026-10-09-imported-upstream-tags-shadowed-the-first-release-tag
description: Caution record for a solved false case or recurring risk.
---

# Imported upstream tags shadowed the first release tag

- Date: 2026-10-09
- Kind: `caution`
- Source: project-bootstrap enrichment 2026-10-09
- Summary: zoetrope's own v0.1.0 and v0.2.0 tags came along with the fork import, so git tag v0.1.0 failed locally while origin had no tags.
- Context: Releasing v0.1.0 on 2026-10-09: the local v0.1.0 pointed at zoetrope's 6cdcc30 changelog commit. Tagging locally or pushing local tags would have published upstream releases under this repo.
- Resolution: Created the release tag on origin directly (git push origin <sha>:refs/tags/v0.1.0), then, with the user's approval, deleted the local upstream tags and fetched the origin tag. Check git tag -l and git ls-remote --tags origin before tagging; never push --tags from a checkout that fetched upstream tags.
- Evidence:
  - git show v0.1.0 (before cleanup: 6cdcc30, Furkan Kalaycioglu)
  - git ls-remote --tags origin
  - cd.yml run 37861198441
