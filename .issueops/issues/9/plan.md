# 이슈 #9 계획: v0.2.0 화면 다듬기와 그래프 기본 화면

- Lifecycle ID: `io-4b513b64dc69`
- Issue: https://github.com/m16khb-org/herdr-agents-graph/issues/9
- Branch: `9-ui-polish-graph-default`, base `main` @ `5825116416ca9d306cc35ac2964901aa27a17dc9`
- 사용자 승인 범위(2026-10-09): 이슈 정리, 브랜치 준비, 계획 작성과 검토까지. 워크트리 준비, 구현 세션 인계, PR, 병합, 릴리스는 사용자가 다음 단계를 승인한 뒤 진행한다. 구현 세션이 인계를 받으면 종료점은 draft PR 발행과 `execution complete`다.
- 버전: 기본 화면이 바뀌고 기능이 늘어나므로 병합 뒤 릴리스는 `v0.3.0`으로 한다(0.x에서 기능 추가는 minor).

## 목표와 비목표

목표는 이슈 #9의 일곱 항목이다.

1. 선택한 채 그래프 보기로 넘어가도 그래프 전체가 보인다.
2. 상단 바의 재생 상태와 에이전트 상태가 다른 낱말과 기호를 쓴다.
3. omp 서브에이전트의 '지금' 줄이 맡은 작업을 보여 준다.
4. 세부 패널 사실 줄이 넘치면 `…`로 끝나고, 도구 수를 `1 tool` / `N tools`로 표기한다.
5. 같은 부모 아래 완료된 서브에이전트 2개 이상을 그래프 카드 하나와 '지금' 한 줄로 접는다.
6. 켜면 그래프 보기가 나온다.
7. 미니맵이 확대, 축소, 이동과 관계없이 같은 모양을 유지하고, 보이는 범위를 테두리로 표시한다.

비목표:

- 레인 보기의 접기, Ctrl+휠(휠 확대는 이미 그래프 캔버스에서 동작한다: `src/handler.rs:64-68` → rataflow `event_handlers.rs:515-522`), 미니맵 클릭 이동.
- 세부 패널의 빈 공간 채우기. 세부 패널은 이미 모든 도구 호출을 스크롤 목록으로 보여 준다(`src/ui/views/detail.rs` `tool_list_lines`, `visible_tool_lines`). 120x40 omp 픽스처에서 비어 보이는 것은 메인 에이전트의 도구 호출이 1개뿐이기 때문이다(`assets/omp/demo.model.txt`). 이 완료 기준은 이슈 본문에서 빼고 계약 변경 feedback으로 기록한다.
- `inspect` 출력 형식 변경, 새 CLI 플래그, 설정 파일.

## 적용되는 결정과 주의사항

- `.issueops/CONSTITUTION.md` "Goldens are contracts": provider 골든은 의도한 변경만 허용한다. 이 계획에서 바뀌는 provider 골든은 항목 3 때문인 `assets/omp/demo.model.txt` 하나뿐이고, 그 diff를 커밋 본문에 설명한다.
- `.issueops/CONSTITUTION.md` "Measured performance is a contract": 끝난 세션 유휴 CPU ≤ 0.5 %를 기본 화면이 그래프인 상태에서도 다시 잰다(G6).
- `.issueops/CONSTITUTION.md` "No session content leaves the machine": 실제 omp 세션으로 확인할 때 키 이름과 길이만 적고 내용은 옮기지 않는다.
- `.issueops/adr/2026-10-09-the-terminal-ui-draws-only-from-pinned-seed-tokens-and-state.md`: 색은 `ui::seed::theme::Theme` 토큰으로만 쓴다. `src/state`는 `src/ui`를 가져오지 않는다. 접기 상태와 행 목록은 `src/state`의 순수 데이터로 두고, 미니맵 위젯은 `src/ui/views/`에 둔다.
- `.issueops/adr/2026-10-09-poll-with-byte-offsets-and-redraw-only-on-visible-change.md`: 화면이 시간에 따라 바뀌는 요소는 다시 그리기 판단에 들어가야 한다. 접기와 미니맵은 시간에 따라 바뀌지 않고, 모델 변경과 viewport 변경에만 반응하므로 기존 `RedrawGate` 입력(모델 stamp, viewport 이벤트)으로 충분하다.
- `.issueops/cautions/2026-10-09-omp-usage-cost-is-an-object-in-real-sessions-but-a-number-in.md`: omp 줄은 `serde_json::from_str(line).ok()`로 읽혀서(`src/provider/omp/wire.rs` `parse_line`) 필드 타입이 하나만 어긋나도 줄 전체가 버려진다. `session_init.task`는 `Option<serde_json::Value>`로 받아 문자열일 때만 쓴다.
- `.issueops/cautions/2026-10-09-a-late-terminal-background-reply-reaches-the-keymap-unless-i.md`: 시작 순서 `Theme::detect → ratatui::init → drain_input → EventStream`(`src/tui.rs`)을 건드리지 않는다. 기본 화면 변경은 `App::new` 초기값만 바꾼다.
- `.issueops/conventions/overview.md` "UI and SEED tokens": UI 문구는 영어다(`✓ 3 done`, `quiet`, `map`). 새 위젯은 `&Theme`을 받는다. `Color::` 리터럴을 `src/ui/seed` 밖에 쓰지 않는다.
- `.issueops/testing/overview.md`: 화면 골든(`assets/ui/*.txt`)은 `UPDATE_GOLDEN=1`로 다시 만들고 diff를 검토한다. `tests/real_sessions.rs`로 실제 세션 파싱 실패 0을 확인한다.
- `src/provider/omp/wire.rs:100-106` 주석의 기존 결정: `task`와 `systemPrompt`는 수 KB의 자유 텍스트라 짧은 설명으로 오용하지 않으려고 읽지 않는다. 이 계획은 그 결정을 좁게 뒤집는다. `task` 전체가 아니라 **빈 줄, Markdown 제목 줄, `:`로 끝나는 소개 줄을 건너뛴 첫 문장 줄 하나**만 `excerpt`(240자 상한, `src/state/session.rs:1065`)로 줄여 쓰고, 그 이유를 주석에 고쳐 적는다. 실제 세션 96개 `tasks[]` 항목에서 `task` 길이 중앙값은 1,759자, 최대 7,774자였다(키 이름과 길이만 측정).
- `.issueops/CAUTIONS.md`의 herdr 포커스 caution: 실제 설치 점검에서 분할 창을 열면 포커스가 옮겨 가므로, 점검 전 포커스를 기록하고 `herdr agent focus`로 되돌린다.

## 재사용하는 기존 구현

- **항목 1**: `App::set_view`(`src/state/mod.rs:1045-1056`)가 그래프로 바꿀 때 `pending_center`를 선택 에이전트로 채우고, 그래프 렌더(`src/ui/views/graph.rs:64-66`)가 `center_node(id, false)`로 현재 배율 그대로 가운데로 옮긴다(`src/state/mod.rs:885-917`). 여기에 두 규칙을 더한다. (a) `set_view(Graph)`에서 카메라가 `Camera::Overview`면 `pending_center`를 비우고 `flow.request_fit_view()`를 부른다. (b) `pending_center`를 소비할 때 선택 노드의 터미널 사각형(rataflow `node_terminal_rect`, `viewport.rs:251`)이 캔버스 안에 다 들어 있으면 옮기지 않는다. 새 카메라 모드는 만들지 않는다.
- **항목 2**: `transport_badge`(`src/ui/chrome.rs:157-176`)의 `Transport::Idle` 문구와 기호만 바꾼다. `Idle` → `quiet`와 `◦`, `Live`의 `●` → `◉`. 에이전트 상태 기호(`src/state/session.rs:106-113`: `● ◌ ✓ ✗ ■`)와 그래프 카드의 숨쉬기 기호 `○`(`src/ui/views/graph.rs:125`)는 그대로 둔다. 구현 시 `◦`와 `◉`가 다른 곳에서 쓰이지 않는지 grep으로 다시 확인한다.
- **항목 3**: `SessionInitEntry`(`src/provider/omp/wire.rs:107-114`)에 `task` 필드를 더하고, `Entry::SessionInit` 가지(`src/provider/omp/mod.rs:226-238`)가 `description`에 첫 문장 줄을 넣는다. 모델의 `fold_kind` Agent 가지는 이미 비어 있는 `description`만 채우므로(`src/state/session.rs:443-500`, first-wins) 병합 규칙은 바꾸지 않는다. '지금' 줄은 `intent_line`(`src/state/session.rs:1049-1056`)의 기존 순서(실행 중 도구 의도 → description → 마지막 추론)를 그대로 쓴다.
- **항목 4**: 사실 줄은 `src/ui/text.rs:17`의 `truncate`로 폭에 맞춰 줄인다. 도구 수 표기는 `src/ui/text.rs`에 `fmt_tool_count(n)` 하나를 두고 세부 패널(`src/ui/views/detail.rs:192`)과 그래프 카드(`src/ui/views/graph.rs:147,153`)가 같이 쓴다.
- **항목 5**: 행 순서는 `SessionModel::tree_order`(`src/state/session.rs:1010`), 상태와 부모는 `AgentInfo`(`status`, `parent`, `kind`), 카드 내용은 기존 `AgentNode`(`src/state/graph.rs:24-37`)를 그대로 쓴다. 접힌 카드는 제목 `3 done`, 상태 `Done`, 도구 수와 토큰은 합계인 `AgentNode`라서 새 `NodeContent` 구현이 필요 없다. 접힌 구성원을 그래프에서 빼는 것은 기존 `retain_nodes`(rataflow `state/graph.rs:343`, 연결된 간선도 지움)와 `remove_agents`(`src/state/graph.rs:247-253`) 방식을 따른다. `set_node_hidden`을 쓰지 않는 이유: rataflow의 Sugiyama 배치(`layout.rs:202` `apply_layout`)는 `hidden`을 보지 않아 숨긴 노드가 자리를 차지한다.
- **항목 6**: `App::new`(`src/state/mod.rs:252-273`)의 `view: View::Now`를 `View::Graph`로 바꾼다. `View`의 `#[default]`(`src/state/view.rs:13-17`)와 `View::ALL` 순서(`Now, Lanes, Graph`, 키 `1 2 3`)는 그대로 둔다.
- **항목 7**: rataflow `MiniMap` 대신 `src/ui/views/minimap.rs`에 작은 위젯을 둔다. 입력은 공개 API만 쓴다: `Flow::nodes()`(rataflow `state/graph.rs:66`), `Node::bounds()`(`node.rs:300`), `Flow.viewport`의 `x, y, zoom`(`mod.rs:307`), `Flow::canvas_size()`(`viewport.rs:210`). 테두리는 ratatui `Block`, 색은 `Theme`. rataflow `MiniMap`을 고쳐 쓰는 대안은 아래 설계 검토에서 기각한다.

## 설계

### 항목 5: 완료 묶기

접기 판단의 주인은 `src/state/view.rs`의 순수 함수 하나다. '지금' 행, 그래프 노드, 선택 이동이 모두 이 결과만 쓴다.

- `fold_set(session, expanded: &HashSet<String>, settled: bool) -> FoldSet`. `settled`는 저장하지 않고 매번 타임라인에서 계산한다: `timeline.replay && timeline.ended() && timeline.at_edge()`. `ended`는 `Timeline`의 비공개 필드라(`src/state/timeline.rs:437-442`에서 세우고 `:176`, `:264`에서 내림) 읽기 전용 getter `ended()`를 더한다. 저장 플래그로 두면 끝까지 재생 → 뒤로 seek → `G`로 끝에 돌아왔을 때 `just_ended`가 다시 불리지 않아 접기가 사라지고, 계속 자라는 파일에서는 `ended`가 풀려도 플래그가 남는다.
  - 접을 수 있는 에이전트: `kind == Subagent`, `status == Done`, 그리고 `is_terminal()`(`src/state/session.rs:270`)이거나 `settled`(재생이 끝까지 와서 `end_of_stream`을 부른 뒤, `src/state/mod.rs:670-675`)인 것. 실시간 세션에서 시간만으로 Done이 된 비종결 에이전트는 다시 Running으로 돌아올 수 있어(`src/state/session.rs:815-826`) 접지 않는다. 그래야 같은 에이전트가 접혔다 풀렸다를 반복하지 않는다. `end_of_stream`은 `status`만 바꾸고 `terminal`은 세우지 않으므로(`src/state/session.rs:843-870`) `settled`가 따로 필요하다.
  - 규칙: 같은 부모 아래 접을 수 있는 형제가 2개 이상이고, 그 부모가 `expanded`에 없으면 한 묶음이다. **선택 상태는 보지 않는다.** 구성원의 자손도 함께 접힌다(구성원이 보이지 않으면 자손도 보이지 않는다).
  - 만드는 순서: `tree_order`를 따라가며, 이미 숨겨진 하위 트리 안의 에이전트는 묶음을 만들 때 건너뛴다. 그래서 숨겨진 구성원 아래에 다시 묶음이 생기지 않는다(omp는 서브에이전트가 자기 `task` 호출로 손자를 띄워 이런 트리를 만든다, `src/provider/omp/mod.rs:405-432`).
  - `FoldSet { by_parent: HashMap<String, Vec<String>>, member_of: HashMap<String, String> }`. 구성원 순서는 `tree_order` 순서다. `member_of`에는 구성원과 그 자손을 **모두** 넣고, 값은 보이는 묶음의 부모 id다. 그래서 숨겨진 자손을 선택해도 그 묶음으로 정규화된다.
  - 비용은 O(n)이다. '지금' 보기와 `select_step`은 프레임마다, 그래프는 sync할 때 계산한다.
- `rows(session, folds: &FoldSet) -> Vec<Row>`: `tree_order`를 따라가며 묶음의 첫 구성원 자리에 `Row::Folded { parent, members, depth }` 하나를 두고 나머지 구성원과 그 자손은 건너뛴다. 그 밖에는 `Row::Agent { id, depth }`.
- `App`에 `expanded_folds: HashSet<String>`(부모 id)과 `Selection.folded: Option<String>`(선택된 묶음의 부모 id)을 더한다. `Selection.agent`와 `Selection.folded`는 동시에 `Some`이 되지 않는다.
  - 선택된 에이전트가 끝나 묶음에 들어가면 선택을 그 묶음으로 옮긴다: 프레임마다 `Selection.agent`가 `member_of`에 있으면 `Selection.folded = parent`, `Selection.agent = None`으로 정규화한다(`App::normalize_fold_selection`). 이 정규화는 접는 보기에서만 한다: `set_view`로 '지금'이나 그래프에 들어갈 때 한 번, 그리고 그 두 보기에서는 렌더 전에 매 프레임. 레인 보기는 선택 표시와 스크롤을 `Selection.agent`로만 하므로(`src/ui/views/lanes.rs:294-298`, `334-339`) 레인 보기에서는 정규화하지 않는다.
  - `enter`(묶음 선택 중): `expanded_folds`에 부모를 넣고 선택을 첫 구성원으로 옮긴다.
  - `esc`(펼친 묶음의 구성원 선택 중): 부모를 `expanded_folds`에서 빼고 `Selection.folded = parent`, `Selection.agent = None`. 기존 esc 순서(도움말 → 정보 → 세부 → 펼침 → 선택 해제, `src/handler.rs`)에서 "펼침" 단계 다음, "선택 해제" 단계 앞에 둔다.
  - 선택 해제는 두 필드를 모두 비운다. `select_agent(None)`이 `Selection.agent`가 이미 `None`이면 일찍 돌아가는 부분(`src/state/mod.rs:1001-1003`)을 고쳐 `Selection.folded`도 비운다.
- `select_step`(`src/state/mod.rs:1026`): '지금'과 그래프 보기에서는 `rows()`를 따라 움직이고, **레인 보기에서는 지금처럼 접지 않은 `tree_order`(`agent_rows()`, `src/state/mod.rs:988-994`)를 따른다.** 레인 보기는 모든 레인을 그리므로(`src/ui/views/lanes.rs:246`) `j`/`k`로 모든 레인에 닿아야 한다.
- '지금' 보기(`src/ui/views/now.rs`)는 `Row::Folded`를 `✓ 3 done` 한 줄로 그린다. `HitMap.rows`에는 묶음 줄을 접두어 `\u{1f}fold:` + 부모 id 키로 등록한다(에이전트 id에 제어 문자가 들어가지 않으므로 충돌하지 않는다). `handle_mouse`의 행 클릭(`src/handler.rs:58-61`)은 이 접두어를 풀어 `App::select_fold(parent)`를 부르고, 그 밖의 키만 `select_agent`로 보낸다.
- 세부 패널은 묶음이 선택되면 구성원 목록(상태 기호, 이름, 걸린 시간, 토큰)을 보여 준다. 기존 `fmt_timing`, `fmt_tokens`를 쓴다. `src/ui/mod.rs:86-88`의 선택 필터는 `Selection.folded`도 다룬다.
- 그래프(`src/state/graph.rs` `sync`)는 `FoldSet`을 인자로 받는다.
  - 묶음 구성원(과 그 자손)의 노드를 `retain_nodes`로 빼고, 부모 아래에 묶음 카드 하나(id `"{parent}\u{1f}done"`, 간선 `e-{카드 id}`)를 둔다. 더 이상 존재하지 않는 묶음의 카드는 지운다(seek-back, Done→Running 되돌림, 펼치기).
  - 빼는 노드의 위치는 `App.parked_positions: HashMap<String, Position>`에 남기고, 다시 더할 때 그 위치를 쓴다. 묶음 카드는 첫 구성원의 위치에 둔다. 그래서 사용자가 끌어 둔 카드 위치가 접기와 펼치기, seek를 오가도 유지된다.
  - **자동 재배치는 하지 않는다**(기존 규칙: 배치는 `r`로만 다시 한다, `src/state/mod.rs:795-798`, `docs/DESIGN.md:307`). 접기로 구조가 바뀌면 `layout_dirty`를 세우고, 카메라가 `Overview`면 전체 맞춤만 요청한다(노드 위치는 그대로, 카메라만 움직인다). seek 경로(`rebuild_to`, `commit_seek`, `src/state/mod.rs:718-785`)에서는 맞춤도 요청하지 않는다.
  - 구조 변화는 sync를 부르는 모든 경로가 같은 방식으로 처리한다: `resync`(`src/state/mod.rs:788-806`), `status_tick`(`:838-853`, `recompute_liveness`가 시간만으로 Running↔Done을 바꿀 수 있어 묶음이 생기거나 풀린다, `src/state/session.rs:815-826`), 재생 끝 정리(`:670-675`). 지금은 `status_tick`이 sync의 구조 결과를 버리므로 이를 받도록 고친다.
  - **묶음 입력이 어느 경로로 바뀌어도 그래프가 따라가도록 한 곳에서 맞춘다.** `App`에 `synced_folds: FoldSet`(마지막으로 `graph::sync`에 넘긴 묶음)을 두고, `App::reconcile_folds() -> bool`이 현재 `fold_set`을 계산해 `synced_folds`와 다르면 묶음 반영 sync를 부르고 `true`를 돌려준다. `tui::run` 루프는 `tick_timeline`과 `status_tick` 다음(`src/tui.rs:93-104`)에 매 반복 한 번 이것을 부르고, `true`면 `gate.mark()`로 다시 그린다. 그래서 위에 적은 경로(`resync`, `status_tick`, 재생 끝 정리, seek)뿐 아니라 끝에서 멈춘 재생 파일이 자라 `settled`가 바뀌는 경우(`src/state/mod.rs:458-467`의 따라가지 않는 Batch 가지는 sync를 부르지 않는다, `src/state/timeline.rs:263-265`)처럼 따로 sync를 부르지 않는 경로도 다음 반복에서 맞춰진다.
  - 사용자가 `enter`/`esc`로 펼치거나 접으면 그 자리에서 `reconcile_folds()`를 부른다(모델 변화가 없어도). 이때도 재배치는 하지 않는다.
  - 선택 반영(`src/ui/views/graph.rs:47-56`): `Selection.folded`가 있으면 묶음 카드를, `Selection.agent`가 묶음 구성원이면 그 묶음 카드를 선택 상태로 둔다. `pending_center`와 Follow 카메라의 `track_activity`도 대상이 묶음 구성원이면 묶음 카드 id로 바꿔 쓴다.
  - 그래프에서 묶음 카드를 고르면(`process_flow_events`의 `SelectionChanged`, `src/state/mod.rs:1161-1170`) 카드 id를 풀어 `select_fold(parent)`를 부른다.
- 레인 보기는 접지 않는다. 묶음이 선택된 채 레인 보기로 가면 선택 표시 없이 그리고, `j`/`k`는 첫 구성원부터 움직인다.
- 세션이 바뀌면(`SessionReset` 처리, `src/state/mod.rs:492-520`, 새 flow를 만드는 `:502` 옆) `expanded_folds`, `parked_positions`, `Selection.folded`를 비운다. 루트 id는 세션마다 `main`이고(`src/state/session.rs:17`) omp 작업 이름도 세션마다 되풀이되므로, 비우지 않으면 이전 세션에서 펼친 상태와 위치가 새 세션에 남는다.
- `inspect`(`src/state/render.rs`)와 `SessionModel`은 접기를 모른다.

### 항목 7: 미니맵

- 축척은 보이는(그래프에 있는) 노드들의 경계만으로 정하고, 한 축이 0이면 1로 둔다. viewport는 축척 계산에 넣지 않는다.
- 미니맵 크기는 26x10 셀(테두리 포함), 오른쪽 위, 캔버스와 1칸 떨어뜨린다. 캔버스가 60x20보다 작으면 그리지 않는다.
- 노드는 셀 단위 사각형으로 칠한다(반 칸 블록을 쓰지 않는다). 선택 노드는 brand 토큰, 나머지는 neutral stroke 토큰이다. 축척 뒤 두 노드가 같은 셀에 겹치면 겹친 칸만 같은 색이 될 뿐 모양이 확대 배율에 따라 바뀌지 않는다.
- 보이는 범위는 `─│┌┐└┘` 테두리로 그리고 미니맵 안쪽에 맞춰 자른다. 보이는 범위가 노드 경계를 모두 덮으면 미니맵 안쪽 가장자리에 붙는다.
- 테두리 제목은 ` map `, `?` 도움말에 `map: whole graph; the frame is what you see` 한 줄을 더한다.

### 항목 1, 2, 3, 4, 6

위 "재사용하는 기존 구현"에 적은 변경 그대로다. 항목 3의 첫 문장 줄 규칙은 `trim` 뒤 비지 않고, `#`으로 시작하지 않고, `:`로 끝나지 않는 첫 줄이다. 그런 줄이 없으면 `description`을 비워 둔다. `:`로 끝나는 줄을 건너뛰는 이유: 실제 omp 세션의 `session_init.task` 108개는 모두 같은 소개 줄(`:`로 끝남) → `#` 제목 → 본문 순서였고, `#`만 건너뛰면 108개 모두 같은 소개 줄이 나왔다. `:` 줄까지 건너뛰면 108개 중 95개가 서로 다르고 빈 결과는 없었다(내용은 옮기지 않고 개수만 셈).

## 성능 영향

- hot path는 16 ms 틱의 다시 그리기다. `rows()`는 프레임마다 `tree_order`(이미 프레임마다 O(n)으로 호출됨)를 한 번 더 훑어 O(n)이고, 접기 집합 조회는 `HashSet`이라 O(1)이다. 에이전트 수 n은 세션당 수백 이하다.
- `reconcile_folds()`는 루프 반복마다(움직일 때 16 ms, 쉴 때 200 ms) `fold_set` 계산과 `HashMap` 비교를 한 번씩 해 O(n)이다. 바뀌었을 때만 graph sync를 부른다. 유휴 CPU는 G6로 다시 잰다.
- 미니맵은 기존 rataflow `MiniMap`처럼 노드마다 사각형 하나를 칠해 O(n + 미니맵 셀 수)다.
- 접기로 그래프 노드 수가 줄어 큰 세션에서는 Sugiyama와 캔버스 렌더 비용이 줄어든다(측정은 G6 유휴 CPU와 real_sessions로 확인).
- 기본 화면이 그래프가 되면 끝난 세션의 첫 화면이 그래프다. 그래프 카드의 숨쉬기(`animation_phase`)는 `Running` 에이전트에만 있으므로 끝난 세션의 유휴 CPU는 늘지 않아야 한다. G6에서 `scripts/idle-cpu.sh`로 잰다.
- transcript 읽기와 200 ms 꼬리 읽기 경로는 바뀌지 않는다. omp `session_init` 한 줄에서 문자열 하나를 더 읽는다.

## 하위 호환성과 side effect

- CLI 인자와 `inspect` 형식은 그대로다. `inspect` 출력 내용은 omp 서브에이전트에 `description` 줄이 생기는 만큼 바뀐다(`src/state/render.rs:84-87`은 `agent_type`이 있을 때 description을 출력한다). 그래서 `assets/omp/demo.model.txt`가 바뀐다. 다른 provider 골든(`assets/claude/**`, `assets/codex/**`)과 `*.timeline.txt`는 바뀌지 않아야 한다(G4).
- 화면 골든 `assets/ui/*.txt` 12개는 접기, 상단 바 문구, 미니맵, 도구 수 표기로 바뀐다. diff를 검토하고 커밋한다.
- 기본 보기가 `Now`라는 데 기대던 테스트(`src/handler.rs:401-402`, `424-447`)에는 `set_view(View::Now)`를 명시한다.
- 사용자에게 보이는 변경: 켜면 그래프 보기, 상단 바 `quiet`/`◉`, 접힌 행과 카드, 미니맵 테두리. README의 화면 설명, 키 표, "Changed from 0.2.0" 표와 `docs/DESIGN.md`의 UI 절을 고친다.
- 파일, 원격, 상태 side effect는 없다. 세션 파일은 읽기만 한다.
- 롤백: v0.2.0으로 재설치(`herdr plugin install m16khb-org/herdr-agents-graph/herdr-plugin --ref v0.2.0`).
- 이 계획은 LLM 프롬프트 본문을 바꾸지 않는다.

## 게이트

```text
- [ ] G1: 전체 테스트 통과
  CHECK: cargo test --locked --all-features
  EXPECT: test result: ok, failed 0
- [ ] G2: clippy 경고 0
  CHECK: cargo clippy --all-targets --all-features -- -D warnings
  EXPECT: exit 0
- [ ] G3: src/ui(seed 제외)와 src/state에 Color:: 리터럴 0
  CHECK: grep -rn 'Color::' src/ui src/state --include=*.rs | grep -v '^src/ui/seed/' | grep -vc 'Color::Reset'
  EXPECT: 0
- [ ] G4: provider 골든 중 assets/omp/demo.model.txt만 바뀜
  CHECK: git diff --name-only 5825116 -- 'assets/claude' 'assets/codex' 'assets/omp'
  EXPECT: assets/omp/demo.model.txt 한 줄
- [ ] G5: 실제 세션 파싱 실패 0
  CHECK: AG_REAL_SESSIONS=1 cargo test --release --test real_sessions -- --nocapture
  EXPECT: failed=0
- [ ] G6: 끝난 데모와 Running 사본의 유휴 CPU ≤ 0.5 % (기본 그래프 화면)
  CHECK: cargo build --release --locked && scripts/idle-cpu.sh assets/claude/demo.jsonl && scripts/make-running-demo.sh /tmp/g6-running.jsonl && scripts/idle-cpu.sh /tmp/g6-running.jsonl
  EXPECT: 두 idle_cpu_percent 모두 ≤ 0.5
- [ ] G7: 선택 후 그래프 전환에서 모든 카드가 캔버스 안에 있음(120x40 omp 픽스처)
  CHECK: cargo test --locked --all-features graph_switch_keeps_every_card_in_view
  EXPECT: 1 passed
- [ ] G8: 미니맵 모양이 확대, 축소, 이동과 무관함
  CHECK: cargo test --locked --all-features minimap_shape_ignores_zoom_and_pan
  EXPECT: 1 passed
- [ ] G9: 기본 화면이 그래프
  CHECK: cargo test --locked --all-features app_opens_on_the_graph_view
  EXPECT: 1 passed
- [ ] G10: 완료 묶기 규칙과 '지금'·그래프 일치
  CHECK: cargo test --locked --all-features done_siblings_fold
  EXPECT: 아래 테스트가 모두 passed
    - done_siblings_fold_in_rows: 재생을 끝낸 픽스처에서 reviewer와 subagent가 Row::Folded 하나, stopped인 scout는 따로
    - done_siblings_fold_graph_matches_now: 레인 보기에서 행을 클릭한 뒤, seek-back 뒤, 각각 그래프 노드 id 집합이 rows()와 일치
    - done_siblings_fold_again_on_esc: 묶음 선택 → enter(선택이 첫 구성원) → esc(선택이 묶음) 뒤 rows()에 두 구성원을 가진 묶음이 다시 있음
    - done_siblings_fold_skips_live_heuristic_done: 실시간 모드에서 비종결 Done 에이전트는 접지 않음
    - done_siblings_fold_row_click_selects_the_fold: '지금'의 묶음 줄 클릭이 Selection.folded를 세움
    - done_siblings_fold_survives_seek_back_and_end: 끝까지 재생 → seek_to_fraction(0.0) → go_live() → tick_timeline 뒤 rows()에 Reviewer와 __sidecar 묶음이 다시 있음
    - done_siblings_fold_nested_tree: main → {A, B} 완료, A → {C1, C2} 완료인 합성 모델에서 그래프 노드 id 집합이 rows()와 같고, 선택한 C1이 Selection.folded = main으로 정규화됨
    - done_siblings_fold_reset_on_session_switch: SessionReset 뒤 expanded_folds와 parked_positions가 비어 있음
    - done_siblings_fold_follows_growth_while_paused_at_end: 끝까지 재생 → toggle_play_pause → 헤드 뒤 시각의 Batch 전달 → reconcile_folds() 뒤 그래프 노드 id 집합이 rows()와 같음
- [ ] G10b: 레인 보기의 j/k가 모든 레인에 닿음
  CHECK: cargo test --locked --all-features lanes_j_reaches_every_lane
  EXPECT: 1 passed (픽스처, set_view(Lanes), 누를 때마다 ui::draw로 한 프레임을 그리고, j 네 번 → main, Reviewer, Helper, __sidecar 순서로 ▶ 표시가 옮겨 감)
- [ ] G10c: 접기와 seek가 끌어 둔 카드 위치를 바꾸지 않음
  CHECK: cargo test --locked --all-features fold_keeps_dragged_positions_across_seek
  EXPECT: 1 passed (Overview 카메라에서 끈 카드가 묶음이 생기고 풀리는 seek 뒤에도 같은 위치)
- [ ] G11: omp description이 session_init.task의 첫 문장 줄
  CHECK: cargo test --locked --all-features omp_subagent_description_is_the_first_task_line
  EXPECT: 1 passed (실제 모양을 흉내 낸 합성 task: `:`로 끝나는 소개 줄, `#` 제목, 본문 줄 → 본문 줄)
- [ ] G12: 사실 줄 생략 표시와 단수 복수
  CHECK: cargo test --locked --all-features detail_facts_line_ends_with_ellipsis tool_count_is_singular_for_one
  EXPECT: 2 passed
- [ ] G13: CI 전 항목 통과(준비 세션이 PR 뒤 확인)
  CHECK: gh pr checks <PR>
  EXPECT: 모든 job pass
- [ ] G14: 실제 설치 점검(준비 세션)
  CHECK: v0.3.0 설치 후 omp 창에서 그래프 열기, 기본 화면이 그래프이고 접힌 카드와 미니맵 테두리가 보임, 다시 눌러 닫힘
  EXPECT: herdr pane read에 ` map ` 테두리와 `done` 카드가 보이고 두 번째 토글 뒤 창이 닫힘
```

## 검증 전략

- 동작 변경마다 실패하는 테스트를 먼저 쓴다(RED → GREEN). G7~G12의 테스트 이름을 그대로 쓴다.
- 120x40 가상 터미널(`pyte` 기반 throwaway 스크립트, 저장소에 넣지 않음)로 이슈 재현 절차 2~7을 다시 돌려 화면 텍스트를 PR 본문에 붙인다.
- 실제 omp 세션으로 서브에이전트 줄과 접기를 확인하되 내용은 옮기지 않는다.

## 설계 검토에서 기각한 대안

- 미니맵: rataflow `MiniMap`을 고쳐 쓰기(상위 PR 또는 `[patch.crates-io]`). rataflow는 crates.io 의존성이고 `Cargo.toml`에 경로 의존성을 두지 않는 규칙이 있으며(`Cargo.toml` 주석), 상위 반영 일정을 알 수 없다. 공개 API만으로 같은 위젯을 100줄 안팎으로 쓸 수 있다.
- 접기: rataflow `set_node_hidden`으로 숨기기. Sugiyama가 숨긴 노드도 배치해 빈자리가 남는다.
- 접기: `Selection.agent`에 가짜 id를 섞기. 에이전트 id와 섞이면 `SessionModel` 조회가 모두 접두어 검사를 해야 한다. 별도 필드 `Selection.folded`로 둔다.
- 그래프 전환: 전환할 때마다 무조건 전체 맞춤. 사용자가 확대해 둔 `Manual` 카메라를 매번 깨뜨린다. Overview 카메라일 때만 맞추고, 그 밖에는 선택 노드가 화면 밖일 때만 옮긴다.
- 항목 3: `solutionSpace`를 설명으로 쓰기. 그 필드는 문제가 얼마나 열려 있는지를 적는 칸이지 맡은 작업이 아니다.
