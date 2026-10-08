---
name: 2026-10-09-codex-holds-a-new-session-at-hooks-need-review-until-herdr-s
description: Caution record for a solved false case or recurring risk.
---

# Codex holds a new session at Hooks need review until herdr's changed hook is trusted

- Date: 2026-10-09
- Kind: `caution`
- Source: issue #1 T10 live QA
- Summary: Codex panes report no agent_session while herdr's updated Codex hook is untrusted.
- Context: After herdr updated its Codex integration hook (v8), every new Codex session stops at 'Hooks need review'. Continuing without trusting leaves the hook off, so herdr never learns the pane's session id and the plugin says there is no session for the codex pane. Trusting hooks is a user decision; QA must not make it.
- Resolution: Trust the hook once in Codex (Review hooks / Trust all) or run herdr integration install codex again. For QA without trusting, skip the prompt and report the session the way the hook does: herdr pane report-agent-session <pane> --source herdr:codex --agent codex --agent-session-id <thread id> (.issueops/issues/1/gate.py codex_pane).
