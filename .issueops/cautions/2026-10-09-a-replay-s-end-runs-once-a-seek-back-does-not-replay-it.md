---
name: 2026-10-09-a-replay-s-end-runs-once-a-seek-back-does-not-replay-it
description: Caution record for a solved false case or recurring risk.
---

# A replay's end runs once; a seek back does not replay it

- Date: 2026-10-09
- Kind: `caution`
- Source: issueops-docs io-4b513b64dc69
- Summary: Timeline::ended latches across seeks, so Timeline::just_ended fires once per load. A test that seeks an already-ended fixture back and then forward or ticks never reaches the replay-end branch of App::tick_timeline, and a loop that waits for App::settled after seeking back with follow_head false never ends (advance returns early).
- Context: Issue #9 review fix: the first draft of a_replay_ending_in_a_fold_refits_the_overview started from ui::snapshots::fixture_app (already played to its end), seeked back one second and looped until settled; the test hung and had to be killed.
- Resolution: Exercise the replay-end path from an unplayed load (ui::snapshots::loaded_fixture) and tick it, drawing a frame per tick, until App::settled. Bound any wait loop on timeline state in tests.
