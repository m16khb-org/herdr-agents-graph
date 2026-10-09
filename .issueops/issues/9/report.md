# Issue #9 implementation report

~~~text
Status: draft (implementation and gates done; slop-clean, docs, verify, commit and PR follow)
Lifecycle: io-4b513b64dc69
Mode/host/model: direct / omp → claude / claude-opus-5-5
Worktree/branch: /Users/m16khb/Workspace/herdr-agents-graph.worktrees/9-ui-polish-graph-default / 9-ui-polish-graph-default
Lease generation: 3 (generation 2 was the omp session after the generation-1 handoff; it stopped on an API 429 rate limit mid review-fix, and with the user's approval a Claude session revoked it, closed the idle omp session and the worktree's reviewr pane for quiescence, finalized and claimed)
Base: 5825116416ca9d306cc35ac2964901aa27a17dc9 (main; sync-base preview: merge not needed)
Acceptance evidence: .issueops/issues/9/gates.md (G1-G6, G8-G12, G10b, G10c, G10d met; G7 abandoned, see below)
Left for the prep session: CI (G13), merge, v0.3.0 release, real install QA (G14), cleanup
Blockers: none
~~~

## Scope change during implementation

The user corrected item 1 while it was being built: the intent is that
`prefix+shift+z` (the Herdr `open` action) shows the graph straight away, not
that switching to the graph with an agent selected keeps every card in view.
`herdr-plugin/herdr/open.sh` runs `agents-graph --provider P --follow ID` with
no view argument, so item 6 (the app opens on `Graph`) delivers it. The plan's
item-1 design (fit on `set_view(Graph)` under the Overview camera, skip
`pending_center` when the card is on screen) and G7 were not built; the
selection-centering behaviour is unchanged. Recorded as a `contract_change`
feedback, the issue body was updated (`remote sync-issue`, readback checked)
and G7 was abandoned in the ledger with the reason.

## What changed

| Item | Change | Files |
| --- | --- | --- |
| 1, 6 | `App::new` opens on `View::Graph`; tests that assumed `Now` set it | `src/state/mod.rs`, `src/handler.rs`, `src/state/frame.rs`, `src/ui/mod.rs` |
| 2 | Transport badge `■ idle` → `◦ quiet`, `● live` → `◉ live` | `src/ui/chrome.rs` |
| 3 | omp `session_init.task` (read as `Option<Value>`) gives a child's `description`: the first line that is not blank, a `#` heading, or ends in `:`, cut by `excerpt` | `src/provider/omp/{wire,mod}.rs`, `src/state/session.rs`, `assets/omp/demo.model.txt` |
| 4 | Detail facts line truncated with `…`; `fmt_tool_count` (`1 tool` / `N tools`) shared by the detail and the graph card | `src/ui/text.rs`, `src/ui/views/{detail,graph}.rs` |
| 5 | `fold_set` / `rows` in `state::view`; `Selection.folded`; `App::{folds, settled, reconcile_folds, select_fold, expand_fold, collapse_fold, normalize_fold_selection}`; `graph::sync` takes the folds (members leave by `retain_nodes`, positions parked, one card per fold); Now rows, fold detail, row and card clicks, `enter`/`esc`, `j`/`k`; the loop reconciles once per turn | `src/state/{view,mod,graph,timeline}.rs`, `src/ui/{mod,views/now,views/detail,views/graph}.rs`, `src/handler.rs`, `src/tui.rs` |
| 7 | Own minimap (26 × 10, ` map `, node-bounds scale, viewport frame) replaces rataflow `MiniMap`; `?` help names it | `src/ui/views/{minimap,graph,mod}.rs`, `src/ui/chrome.rs` |
| docs | README screen, keys, "Changed from 0.2.0"; `docs/DESIGN.md` loop, graph sync (folds), UI | `README.md`, `docs/DESIGN.md` |
| goldens | 12 view goldens regenerated and reviewed | `assets/ui/*.txt` |

## Deviations from the plan

- Item 1 as above.
- The minimap uses one scale for both axes, centered, instead of stretching
  the bounds over the map: world units are terminal cells at zoom 1, so this
  keeps the graph's proportions. A view that covers every card draws its frame
  around them rather than always on the map's inner edge; G8's test checks
  that every card lies inside the frame when zoomed out.
- The fold card and the fold row name their members (`reviewer, subagent`).
  The plan fixed only the title (`N done`), status and sums.
- `esc` folds back by checking each expanded parent (`fold_set` without it)
  for the selected agent, so no second tree-parent rule is needed.

## Implementation review fixes

The implementation review (verdict revise) raised four findings. Each got a
test that failed before the fix (G10d):

| Finding | Fix | Test |
| --- | --- | --- |
| P1: a fold formed on a live session's first read had no parked member position, so its card landed at `(0, 0)` on top of `main` | `graph::local_position` (the new-node placement, now shared) places the card where its first member would have gone | `a_fold_formed_on_first_load_sits_below_its_parent` |
| P2: an agent open in the full detail that folded away left `selection.detail` set, so `j`/`k` scrolled an invisible detail | `normalize_fold_selection` clears `detail` with `expanded` | `folding_the_detailed_agent_closes_the_detail` |
| P3: with nested folds unfolded, `esc` could fold an ancestor's fold, depending on `HashSet` order | `collapse_fold` matches a direct member (`by_parent`), not `member_of` | `done_siblings_fold_nested_tree` (32 fresh apps) |
| P3: a replay that settles into a fold kept the camera framed on the unfolded cards | the replay-end branch of `tick_timeline` requests a fit under Overview when its resync changed structure | `a_replay_ending_in_a_fold_refits_the_overview` |

## UI 판단

- 디자인 시스템: 새 화면 요소(묶음 줄, 묶음 카드, 묶음 세부, 미니맵)는 기존 `ListItem`, `Badge`, `Block`, SEED 토큰(`stroke.neutral-solid`, `stroke.neutral-weak`, `stroke.neutral-contrast`, `fg.brand`)만 씁니다. G3로 `Color::` 리터럴 0을 확인했습니다.
- 접근성: 묶음은 색이 아니라 `✓` 기호와 `N done` 낱말로 드러나고, 상단 바 재생 상태도 `◦ quiet`처럼 기호와 낱말을 함께 씁니다. 미니맵의 선택 카드는 색으로만 구별되지만 같은 선택이 캔버스 카드 테두리와 세부 패널에도 나타납니다.
- 반응형: 80x24와 120x40 골든을 다시 만들어 확인했습니다. 미니맵은 캔버스가 60x20보다 작으면 그리지 않습니다. 묶음 줄은 60열 미만에서 토큰 수를 숨깁니다.
- 모션: 새 애니메이션은 없습니다. 접기와 펼치기는 카드를 움직이지 않고(재배치는 `r`), Overview 카메라만 다시 맞춥니다.
- 실제 화면: 릴리스 바이너리를 120x40 가상 터미널(pyte)에서 실행해 첫 화면(그래프), `G` 뒤 `◦ quiet`와 `✓ 2 done` 카드, 미니맵, `-`·`H`·`J` 뒤 미니맵, '지금' 보기의 묶음 줄과 세부, `enter` 펼침과 `esc` 다시 접기, `?` 도움말을 확인했습니다. `--provider omp --follow`(herdr open과 같은 인자)로 켜도 첫 화면이 그래프입니다.

## Side effects

- No file, remote or state writes; session files are read only.
- `inspect` output gains a description line for omp children that have a
  `session_init.task` (`assets/omp/demo.model.txt`: `Review the diff.`, `Look
  around for related tests.`). Claude and Codex goldens and every
  `*.timeline.txt` are unchanged (G4).
- The loop calls `App::reconcile_folds` once per turn: one `fold_set` (O(n))
  and a `HashMap` compare; the graph re-syncs only when the folds changed.

## Verification

| Check | Result |
| --- | --- |
| `cargo test --locked --all-features` (G1) | 340 passed, 0 failed |
| `cargo clippy --all-targets --all-features -- -D warnings` (G2) | rc=0 |
| `cargo check --lib --locked --no-default-features` | rc=0 |
| `Color::` outside `src/ui/seed` (G3) | 0 |
| provider goldens changed (G4) | `assets/omp/demo.model.txt` only |
| `AG_REAL_SESSIONS=1` real_sessions (G5) | total=3204 failed=0 |
| idle CPU, default Graph view (G6) | finished demo 0.23 %, Running copy 0.10 % |
| named tests G8-G12, G10b, G10c, G10d | all passed |
| real omp sessions (14 newest with children, counts only) | 111 of 121 subagents carry a description; 0 inspect failures |
| pty smoke, release binary | as listed under UI 판단 |

## ai-slop-clean

- duplication: `graph::same_content` compared a freshly built fold card
  field by field before assigning it; the content is already built, so the
  assignment alone does the same. Removed.
- Kept on purpose: `fold_fixture` in `state::tests` binds through `cfg` so
  the module still compiles without `native`; `(None, None) => {}` in
  `ui::render_body` keeps the match exhaustive under its guard.
- SNR (shell estimate over added `src` lines plus `minimap.rs`; `//`, `///`
  and `/* */` lines count as noise, attributes do not): 0.845 before
  (1533 lines, 237 noise), 0.845 after (1521 lines, 236 noise). Doc comments
  are this repo's convention, so this sits at its ceiling.
- After the pass: `cargo test --locked --all-features` 337 passed, 0 failed;
  clippy `-D warnings` rc=0; `cargo fmt --all --check` ok; `git diff --check`
  ok.

After the review fixes (second pass, Claude session):

- duplication: the fold card's fallback placement reuses the new-node
  placement through `graph::local_position` instead of a second copy of the
  sibling-index arithmetic.
- `ui::snapshots::loaded_fixture` splits the unplayed load out of
  `fixture_app` so the replay-end test can play the fixture to its end; the
  goldens still use `fixture_app` unchanged.
- SNR, same estimate: 0.844 (1650 lines, 257 noise); the rise in lines is the
  four regression tests and their doc comments.
- After the pass: `cargo test --locked --all-features` 340 passed, 0 failed;
  clippy `-D warnings` rc=0; `cargo check --lib --no-default-features` rc=0;
  `cargo fmt --all --check` ok; `git diff --check` ok; gates G1-G6, G8-G12,
  G10b-G10d met.
