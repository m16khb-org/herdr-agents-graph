---
name: 2026-10-09-ship-checksummed-release-binaries-installed-inside-the-plugi
description: Accepted decision record with rationale, alternatives, and consequences.
---

# Ship checksummed release binaries installed inside the plugin checkout

- Date: 2026-10-09
- Kind: `adr`
- Source: project-bootstrap enrichment 2026-10-09
- Summary: The herdr plugin's build step downloads the tagged release archive, verifies SHA256SUMS, and installs agents-graph under herdr-plugin/bin; nothing goes on PATH and no brew, cargo or jq is needed.
- Context: herdr runs [[build]] without HERDR_PLUGIN_* variables, so install locations must be relative to the plugin checkout. zoetrope installed zoe via brew or cargo and its scripts needed jq; a PATH install would collide with an existing brew zoe.
- Decision: Tag v* triggers cd.yml: five targets, a checksums job that writes SHA256SUMS, archives with LICENSE and NOTICE. herdr-plugin/herdr/install.sh picks the target from uname, downloads with AG_RELEASE_BASE override, verifies, installs to herdr-plugin/bin/agents-graph. All herdr JSON handling lives in agents-graph herdr resolve|toggle.
- Consequences: Releases are the install path: a repo must be public for herdr's anonymous clone and for unauthenticated asset download. Windows has release archives but no plugin installer target.
- Evidence:
  - .github/workflows/cd.yml
  - herdr-plugin/herdr/install.sh
  - src/herdr.rs
  - commit 2807a63
  - v0.1.0 release and herdr plugin install QA on 2026-10-09
- Alternatives / rejected options:
  - brew/cargo install (zoetrope ensure-zoe.sh): extra toolchains on the user's machine and a shared PATH name.
  - Install into HERDR_PLUGIN_STATE_DIR: not set during [[build]] (herdr src/cli/plugin.rs scrubs HERDR_PLUGIN_*).
  - Keep jq in the shell bridge: an extra runtime dependency the Rust binary already covers.
