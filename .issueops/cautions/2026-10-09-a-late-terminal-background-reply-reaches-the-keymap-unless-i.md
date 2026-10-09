---
name: 2026-10-09-a-late-terminal-background-reply-reaches-the-keymap-unless-i
description: Caution record for a solved false case or recurring risk.
---

# A late terminal background reply reaches the keymap unless input is drained after raw mode

- Date: 2026-10-09
- Kind: `caution`
- Source: issue #6 T8/T9 (io-4b513b64dc69)
- Summary: The startup OSC 11 query can leave the terminal's reply in the input queue, where the keymap would read it as keys; drain input after ratatui::init and before the event reader starts.
- Context: Theme::detect (src/ui/seed/theme.rs) asks the terminal for its background with OSC 11 + DA1 through terminal-colorsaurus before raw mode, waiting up to 1 s. A terminal that answers after that wait leaves `ESC ] 11 ; rgb:… ESC \` and `ESC [ ? … c` in the input. In raw mode crossterm parses those bytes as key events; handler.rs maps `]` to seek_prompt(true) without checking modifiers, so a slow reply jumps the playhead. In canonical mode the reply is not delivered at all (no newline), so draining before raw mode does nothing. Measured on 2026-10-09: a herdr 0.9.3 pane answers both queries in under 1 ms; a pty that never answers delays the first frame by about 1015 ms.
- Resolution: tui::run calls Theme::detect, then ratatui::init (raw mode), then drain_input (crossterm poll(0)/read loop), and only then spawns the EventStream reader. AG_THEME=light|dark skips the query, and the crate skips it itself for TERM=dumb or a non-tty. Keep that order when touching the loop's start.
- Evidence:
  - src/tui.rs run(): Theme::detect → ratatui::init → drain_input → EventStream
  - src/ui/seed/theme.rs decide()/detect(); tests decide_prefers_ag_theme_without_querying, decide_uses_query_then_colorfgbg_then_dark
  - local evidence .issueops/evidence/task-9-startup.txt (first frame 3 ms answered / 1015 ms unanswered / 8 ms TERM=dumb)
