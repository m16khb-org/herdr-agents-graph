---
name: 2026-10-09-poll-with-byte-offsets-and-redraw-only-on-visible-change
description: Accepted decision record with rationale, alternatives, and consequences.
---

# Poll with byte offsets and redraw only on visible change

- Date: 2026-10-09
- Kind: `adr`
- Source: project-bootstrap enrichment 2026-10-09
- Summary: Transcripts are read line by line and tailed by polling with offset and inode identity, with idle backoff; the TUI ticks every 16 ms but draws only when the frame would change.
- Context: zoetrope read whole files into memory (RSS about the file size: 562 MB for a 521 MB Codex rollout) and redrew at about 60 fps even when idle (about 4-5 % CPU). omp rewrites its session files by rename on title change, session switch and branch.
- Decision: Stream the initial load with read_until and hand the consumed byte offset to the tailer; reset on (dev, ino) change or truncation; poll at 200 ms, backing off after 30 s idle; gate draws with RedrawGate/FrameStamp over every time-dependent visible element.
- Consequences: Any new time-dependent UI element must be added to the redraw decision or the screen looks frozen. Gates: inspect RSS <= 100 MB on the 521 MB rollout, idle CPU <= 0.5 %.
- Evidence:
  - src/tailer/{live.rs,bytes.rs,replay.rs}
  - src/state/frame.rs
  - .issueops/issues/1/gates.md G4 G5 G11
  - README.md benchmark table
- Alternatives / rejected options:
  - File-system watchers (FSEvents/notify): coalescing and platform variance; rename rewrites are already caught by the inode check.
  - Fixed 200 ms polling and unconditional redraw: measured idle cost the gates reject.
