# agents-graph UI 재설계: SEED 디자인 토큰 층 + '지금' 중심 화면

- lifecycle ID: io-4b513b64dc69
- 이슈: https://github.com/m16khb-org/herdr-agents-graph/issues/6 (관련: #1)
- 브랜치: `6-seed-ui-redesign` (base `main`, 봉인 SHA 는 record 에 있음)
- 사용자 요청 범위: "당근 디자인시스템을 확인하고 가져와서 세팅하고 그걸 기반으로 ui 층을 새로 설계". 결정: 강조색은 carrot 이 아닌 다른 팔레트(이 계획은 purple), 진행 범위는 전체 사이클(이슈 → 계획 리뷰 → 구현 → PR → CI → 머지 → v0.2.0 릴리스 → 실설치 QA → 정리). 이번 계획의 승인된 종료점은 draft PR 발행과 `execution complete` 이며, 머지·릴리스·QA·정리는 이 준비 세션이 이어서 맡는다.
- 계획 개정 이력: 2차 delta 리뷰(revise, 필수 2건: 테마 타임아웃 수치 불일치, 레인 공백 접기 명세 부족)와 1차 적대 리뷰(revise, 필수 11건: omp 비용 객체 형태, 모든 실행 중 행 재그리기, G5·G6 명령, 테마별 brand 단계, omp summary 비변경, rataflow 팔레트, 키맵 충돌, 시간대 고정, G10 기준, 바이너리 배포물의 Apache 사본)를 반영했다. 바뀐 곳은 본문 그대로 고쳤다.
- 세션 인계: 계획 스테이징 → `execution prepare --mode direct` → 준비 세션이 Herdr 안이므로 Herdr 새 세션으로 인계. 인계 뒤 준비 세션은 구현하지 않는다.

## TL;DR

- Summary: SEED rootage 토큰(3.0.2)을 `design/seed/` 에 고정하고 Rust 토큰 모듈과 테마(라이트/다크, 트루컬러/256)를 만든 뒤, SEED 컴포넌트 의미를 ratatui 위젯으로 옮긴다. 그 위에 UI 층을 새로 쓴다: 기본 '지금' 뷰, 시간 레인 뷰, 토큰으로 재스타일한 그래프 뷰, 상세 패널, 단순 키맵, 반응형 배치. fact 에 도구 의도와 비용을 더한다.
- Deliverables: `design/seed/*`, `src/ui/seed/*`(토큰·테마·위젯), `src/ui/views/*`, 새 `src/ui/mod.rs`·`src/handler.rs` 키맵, fact/provider 확장, 스냅샷 테스트, README 의 새 화면 설명·NOTICE.
- Effort: XL
- Parallel: YES — 3 waves
- Critical Path: T1(토큰 vendoring·생성) → T2(테마·위젯) → T5('지금' 뷰) → T8(셸·키맵·RedrawGate) → T10(스냅샷·QA)

## Context

### Original Request

"근데 ui가 너무 zoetrope를 배꼈어 더 직관적이고 독창적인 ui/ux로 개선이 가능할까?" → (제안: '지금'이 먼저 보이는 화면) → "당근 디자인시스템을 확인하고 가져와서 세팅하고 그걸 기반으로 ui 층을 새로 설계해줘". 결정: 강조색은 SEED 구조 + 다른 팔레트, 범위는 전체 사이클.

### Interview Summary

blocking 질문 두 개만 물었다(강조색, 진행 범위). 나머지(테마 감지 방식, 의존 crate, 뷰 구성, 키맵)는 조사와 관례로 정했다.

### Gap Analysis

- 브랜드: SEED NOTICE 는 "브랜드 리소스(로고·상호·캐릭터 등 당근으로 식별될 수 있는 요소)" 를 상표로 별도 보호한다. 토큰 데이터 자체는 Apache-2.0 이다. brand 역할을 purple 로 매핑하고, carrot 팔레트는 vendored 데이터에는 남기되 어떤 시맨틱 역할에도 매핑하지 않는다. README 에 "당근마켓과 무관하며 SEED 토큰(Apache-2.0)을 사용" 을 적는다.
- purple 선택 근거: SEED 시맨틱에서 blue 는 `informative` 톤(fg/bg/stroke `informative-*`)이 쓴다. purple 팔레트는 SEED 시맨틱 어디에서도 참조되지 않는다(`bg.magic-weak` 는 hex 리터럴). 그래서 brand 와 상태 톤이 겹치지 않는다.
- 터미널 제약: 글꼴 크기·라운드 모서리·그림자·알파 합성이 없다. 매핑 규칙(아래 "SEED → TUI 매핑")으로 흡수하고, 알파 토큰은 생성 시점에 레이어 배경 위에 합성한 불투명색으로 만든다.
- 결합 정리: 지금 `state/graph.rs:13-14` 가 `ui::nodes`/`ui::edges` 를, `state/mod.rs:24` 가 `ui::chips::ChipTray` 를 가져온다. 새 설계는 상태가 UI 를 참조하지 않게 뒤집는다(그래프 노드·간선 타입은 `ui/views/graph` 로, 칩 트레이는 제거).
- 칩(ChipTray, TTL 2.5 s/4 s)은 '지금' 뷰의 "현재 도구 + 최근 도구" 줄로 대체한다. 최근 도구는 `SessionModel` 의 `tool_calls` 에서 결정적으로 구한다. 시간 기반 페이드는 SEED `duration.color-transition`(150 ms)에 맞춰 상태 변화 직후 한 번 강조하는 것으로 줄인다.
- 기존 동작 유지: 리플레이·스크럽(`Timeline` cursor/seek, `[`/`]` 프롬프트 이동, `G` 라이브), `--follow`, `inspect`.

## 적용되는 결정과 주의사항

운영 문서(CONSTITUTION·ARCHITECTURE·CONVENTIONS·ADR·TESTING)는 이 저장소에 없다. `.issueops/` 에는 이전 사이클의 caution 2건과 issue #1 산출물만 있다(`find .issueops -type f`). 적용되는 것은 아래와 같다.

- caution `2026-10-09-a-herdr-plugin-pane-s-own-context-names-the-focus-at-pane-st`: 그래프 패널은 action 의 `focused_pane_id` 를 `AGENTS_GRAPH_PANE` 으로 받아야 하며 `herdr resolve` 는 그 값을 컨텍스트보다 먼저 읽는다(테스트 `the_named_pane_outranks_the_pane_commands_own_focus`, `src/herdr.rs`). 이 계획은 `src/herdr.rs` 와 브리지를 바꾸지 않으므로 그 동작과 테스트를 그대로 유지한다.
- caution `2026-10-09-codex-holds-a-new-session-at-hooks-need-review-until-herdr-s`: Codex 패널 QA 는 hook 신뢰 여부에 좌우된다. 실설치 QA(G14)는 omp 패널을 기준으로 하고, Codex 패널을 쓸 때는 caution 의 보고 절차를 따르며 hook 신뢰를 대신 결정하지 않는다.

record 의 결정도 제약이다.

- 결정(scope) "no split": 한 PR. 토큰 층의 유일한 소비자가 새 UI 라 따로 머지해도 사용자 가치가 없다.
- 결정(architecture) "SEED 토큰 고정·브랜드는 purple": rootage-artifacts 3.0.2 / `daangn/seed-design@22b68ce` 고정, LICENSE·NOTICE 귀속, YAML 파서는 dev-dependency, brand=purple.
- 이전 사이클의 성능 기준(이슈 #1, PR #2): 521 MB Codex RSS ≤ 100 MB, 끝난 세션 유휴 CPU ≤ 0.5 %, 실파일 전수 패닉 0, Claude/Codex/omp 골든 무변경. 새 UI 도 `RedrawGate`(`src/state/frame.rs:61-75`)로 그리기를 억제해야 한다.
- PR #2 리뷰 메모: 그리기 판단이 시간 의존 표시를 빠뜨리면 화면이 멈춘 것처럼 보인다. 새 뷰의 시간 의존 요소(현재 도구 경과 시간, 시간 레인의 '지금' 끝, snackbar 만료)를 모두 술어에 넣는다.

## 재사용하는 기존 구현

| 재사용 | 위치 | 방식 |
|---|---|---|
| 세션 모델 | `src/state/session.rs` `SessionModel`(:30), `AgentInfo`(:191: kind, agent_type, description, parent, spawned_by, status, model, tool_calls, output_tokens, first_ts, last_ts), `ToolCallInfo`(:160), `status glyph()`(:121)·`status_word`(:136) | 뷰의 유일한 데이터 원천. 읽기 전용. glyph/word 는 상태 배지 텍스트로 그대로 씀 |
| 타임라인 | `src/state/timeline.rs` `Timeline`(:43) cursor/head/follow_head, `advance`(:341), `now_reference`(:430), `progress`(:485) | 시간 레인 뷰의 가로축과 커서. 스크럽 동작 그대로 |
| App 상태와 동작 | `src/state/mod.rs` `App`(:153), `seek_to_fraction`(:306), `seek_prompt`(:322), `go_live`(:364), `toggle_play_pause`(:1012), `selected_agent_id`(:984), `tick_timeline`(:648), `status_tick`(:836) | 키맵이 호출하는 동작. 선택 상태는 뷰 공통 `Selection` 으로 일반화 |
| 그래프 레이아웃 | `src/state/graph.rs` `sync`(:109)·`relayout`(:245, Sugiyama vertical), rataflow `Flow` | 그래프 뷰(보조)로 유지. 노드·간선 렌더만 토큰 위젯으로 교체, 타입은 `ui/views/graph.rs` 로 이동 |
| 그리기 억제 | `src/state/frame.rs` `FrameStamp`(:22-30)·`RedrawGate`(:61)·`due`(:75), 테스트 `draw_skipped_when_clean`(:176) | 술어만 새 뷰 요소로 교체 |
| 문자열 보조 | `src/ui/mod.rs` `truncate`(:823)·`wrap`(:854)·`truncate_tail`(:925), `ui/nodes.rs` `fmt_tokens`(:55), `ui/panel.rs` `fmt_timing`(:503) | `src/ui/text.rs` 로 옮겨 재사용(동작 동일, 기존 테스트 이동) |
| 스냅샷 방식 | ratatui `TestBackend`(이미 `frame.rs:214-216` 에서 사용), 골든 갱신 관례 `UPDATE_GOLDEN=1`(`provider/harness.rs:89-100`) | 뷰 스냅샷 골든과 토큰 생성물에 같은 관례(`UPDATE_GOLDEN=1`, `UPDATE_SEED=1`) |

새로 만드는 것과 이유: SEED 토큰 모듈·테마·위젯(지금 UI 는 색을 파일마다 하드코딩하므로 재사용할 토큰 층이 없음), '지금'·시간 레인 뷰(기존에 없는 화면), 하단 힌트 바(기존 상태바 `render_status_bar`(:702)를 대체). 기존 `ui/chips.rs`(918줄)·`ui/panel.rs`(612줄)·`ui/nodes.rs`·`ui/edges.rs` 는 대체되어 삭제한다.

## 디자인 시스템 층 (세팅)

### vendoring

- `design/seed/` 에 `@seed-design/rootage-artifacts` 3.0.2(`daangn/seed-design@22b68ce26...`)의 `packages/rootage/{color,duration,timing-function,dimension,radius,font-weight}.yaml` 원본과 `LICENSE`, `NOTICE` 를 그대로 둔다. `design/seed/SOURCE.md` 에 출처 URL, 커밋, 받은 날짜, 갱신 절차를 적는다.
- 갱신 절차: `scripts/sync-seed.sh <git-sha>` 가 raw.githubusercontent.com 에서 같은 파일 목록을 받아 덮어쓰고, 이어서 `UPDATE_SEED=1 cargo test --locked seed_tokens` 로 생성물을 갱신한다.
- 저장소 `NOTICE` 에 "SEED Design, Copyright 2025 주식회사 당근마켓, Apache-2.0, 브랜드 리소스 미사용, brand 역할 purple 치환" 귀속을 추가한다.
- 바이너리 배포물에도 Apache-2.0 사본과 SEED NOTICE 를 넣는다: `.github/workflows/cd.yml` 의 `Package (Unix)`·`Package (Windows)` 단계에서 `design/seed/LICENSE` 를 `LICENSE-APACHE-SEED` 로, `design/seed/NOTICE` 를 `NOTICE-SEED` 로 스테이징 디렉터리에 복사한 뒤 함께 묶는다(tar `-C` 로는 이름을 바꿀 수 없으므로 복사본 사용, 묶은 뒤 복사본은 지워 별도 asset 으로 올라가지 않게 함). `Cargo.toml` `include` 에 `design/seed/LICENSE`, `design/seed/NOTICE` 를 넣는다(PR CI 의 `cargo publish --dry-run` 이 검증). `install.sh` 는 아카이브에서 `agents-graph` 만 풀므로 영향 없음.

### 생성물 `src/ui/seed/tokens.rs`

- dev-dependency `yaml-rust2 = "0.13"`(MIT/Apache, MSRV 1.85) 만 쓴다. 런타임 의존 없음.
- 테스트 `seed_tokens_match_vendored_yaml`(`src/ui/seed/gen.rs`, `#[cfg(test)]`)가 YAML 을 읽어 참조(`$color.palette.gray-1000`)를 끝까지 풀고, 아래 형태의 Rust 소스를 만든 뒤 커밋된 `tokens.rs` 와 문자열 비교한다. `UPDATE_SEED=1` 이면 파일을 다시 쓴다.
- 생성 형태:
  - 파일 머리말: "SEED Design(daangn/seed-design@22b68ce, Apache-2.0)의 rootage 토큰에서 생성. brand 역할의 carrot 참조를 같은 단계의 purple 로 치환하고 알파 토큰을 불투명색으로 합성함(Apache-2.0 §4(b) 변경 고지). 직접 고치지 말 것." 모듈 선언에 `#[rustfmt::skip]` 을 붙여 CI fmt 와 문자열 비교가 충돌하지 않게 한다.
  - `pub struct Rgb(pub u8, pub u8, pub u8);`
  - 시맨틱 색: `pub mod fg { pub const NEUTRAL: Themed = Themed { light: Rgb(..), dark: Rgb(..) }; ... }`, `bg`, `stroke` 같은 방식. 이 계획이 쓰는 토큰만이 아니라 시맨틱 전부(fg 22·bg 45·stroke 16)를 생성한다. 알파 토큰(예: `bg.neutral-weak-alpha`, `overlay`)은 각 테마의 `bg.layer-default` 위에 합성한 불투명색으로 만든다.
  - `brand` 역할: SEED 의 `fg.brand`·`bg.brand-*`·`stroke.brand-*` 가 참조하는 carrot 단계를 같은 단계의 purple 로 치환한 값을 `brand` 모듈로 생성한다(테마마다 SEED 가 참조하는 단계를 그대로 따른다. 예: `bg.brand-solid` 는 light 가 carrot-600 → purple-600(#9f84fb), dark 가 carrot-700 → purple-700(#a78df0)). 치환 규칙은 생성기 상수 `BRAND_PALETTE = "purple"` 하나로 둔다.
  - 256색 대체: 각 Rgb 에 가장 가까운 xterm-256 인덱스(16~255, CIE76 거리)를 생성 시점에 계산해 `Themed` 에 함께 넣는다.
  - `pub mod duration { D1..D6: Duration, COLOR_TRANSITION }`, `pub mod dimension { X1..X16 }`(px), `radius`, `font_weight`.

### 테마 `src/ui/seed/theme.rs`

- `Theme { mode: Light|Dark, depth: TrueColor|Ansi256 }`, `Theme::color(Themed) -> ratatui::style::Color`.
- 모드 결정 순서: `AG_THEME=light|dark` > 터미널 배경 질의(`terminal-colorsaurus` 1.0.3, MIT/Apache, MSRV 1.74; crate 가 스스로 raw mode 를 켜고 복원하며, TERM 이 dumb/미설정이거나 입출력이 tty 가 아니면 질의하지 않고 unsupported 를 돌려준다) > `COLORFGBG` > 다크. 타임아웃은 crate 권장값(1 s, SSH 지연 대비)을 쓰되, 질의는 raw mode 진입 전에 하고, `enable_raw_mode` 직후 crossterm `event::poll(0)` 루프로 남은 입력을 비워 늦게 온 OSC/DA1 응답이 키 입력으로 새지 않게 한다(canonical 모드에서는 줄바꿈 없는 응답이 읽히지 않으므로 비우기는 raw mode 에서 한다). herdr 패널이 OSC 11 에 응답하는 시간은 T2 에서 실측해 README 에 적는다.
- 결정 로직은 환경 값을 인자로 받는 순수 함수 `decide(env: &EnvSnapshot, query: impl FnOnce() -> Option<Rgb>)` 로 두어, 테스트가 프로세스 환경 변수를 바꾸지 않게 한다(edition 2024 의 `set_var` unsafe·병렬 테스트 경쟁 회피).
- 색 깊이: `COLORTERM` 이 `truecolor`/`24bit` 이면 트루컬러, 아니면 256색. `AG_COLOR=256` 으로 강제 가능.
- 배경: 화면 바탕은 `bg.layer-basement`, 리스트·패널 표면은 `bg.layer-default`, 오버레이(도움말)는 `bg.layer-floating`. 사용자가 터미널 투명 배경을 쓰는 경우를 위해 `AG_BG=none` 이면 basement 를 칠하지 않는다.

### SEED → TUI 매핑

| SEED | TUI 규칙 |
|---|---|
| `fg.neutral` / `neutral-muted` / `neutral-subtle` / `placeholder` | 본문 / 보조 텍스트 / 메타(시간·토큰) / 빈 상태 |
| `font-weight.bold` / `medium` / `regular` | `Modifier::BOLD` / 일반 / 일반. 제목·선택 행만 bold |
| `fg.positive`·`critical`·`warning`·`informative`·`brand` + 대응 `bg.*-weak` | 상태 톤. 배지·callout 의 글자색과 약한 배경 |
| `stroke.neutral-weak` / `neutral-contrast` | 구분선·상자 테두리 / 강조 테두리(선택 패널). `neutral-muted`·`neutral-subtle` 은 알파 토큰이라 라이트에서 weak 보다 옅으므로 쓰지 않는다 |
| `dimension.x1`(4 px) | 반 셀은 표현 불가. 가로 1셀 = x2(8 px), 세로 1줄 = x4(16 px) 로 환산. `spacing-x.global-gutter`(x4=16 px) → 좌우 여백 2셀, `between-chips`(x2) → 칩 간격 1셀 |
| `radius.r2` 이상 / `full` | `BorderType::Rounded`(╭╮) / 배지 양끝 공백 패딩 |
| `duration.d1`~`d6`, `color-transition`, `timing-function.*` | 애니메이션 단계 길이(그래프 카메라 이동은 d6=300 ms 로, 기존 `GLIDE_SECS=0.5` 대체), 상태 변화 강조 지속 150 ms |
| `shadow`, `gradient` | 쓰지 않음 |

### 위젯 `src/ui/seed/widgets/*.rs` (SEED 컴포넌트 → ratatui `Widget`)

| 위젯 | SEED 원천 | 쓰임 |
|---|---|---|
| `Badge { tone, variant: Weak|Solid|Outline, glyph, label }` | `components/badge.yaml` variants·tones | 에이전트 상태(● 실행 중, ✓ 완료, ✗ 실패, ◌ 대기, ⏸ 멈춤) |
| `Chip { label, selected }` | `chip`, `chip-tablist` | 도구 이름, 뷰 전환 탭 |
| `Callout { tone, title, body }` | `callout` | 실패·막힘·세션 없음 안내 |
| `ListItem { leading, title, detail, trailing, selected, depth }` | `list-item` | '지금' 뷰의 에이전트 행 |
| `TabList { items, active }` | `tablist`/`segmented-control` | 상단 뷰 전환(지금 · 시간 · 그래프) |
| `ProgressDots`, `Spinner` | `progress-circle` | 실행 중 표시(틱마다 바뀌지 않고 1 s 단위) |
| `Snackbar { message, until }` | `snackbar` | "세션 전환됨", "라이브로 이동" 같은 일시 알림(d6×10=3 s) |
| `Divider` | `divider` | 구역 구분 |
| `Skeleton` | `skeleton` | 세션 적재 중 자리 표시 |
| `KeyHint { key, label }` | (SEED 에 없음, chip weak 스타일 재사용) | 하단 힌트 바 |

각 위젯은 `Theme` 를 받아 시맨틱 토큰으로만 색을 정한다. `src/ui/seed/` 밖에서는 `Color::` 리터럴을 쓰지 않는다.

## 화면 설계

### 공통 골격

```text
┌ 상단 바 ─────────────────────────────────────────────────────────────┐
│ omp · herdr-agents-graph   ● 실행 중 4m12s  311k tok  $1.84   [지금] 시간  그래프 │
├──────────────────────────────────────────────────────────────────────┤
│ (선택한 뷰 본문)                                                       │
├──────────────────────────────────────────────────────────────────────┤
│ ⚠ 실패 1 · ⏳ 30초 넘은 도구 1        j/k 이동 · enter 펼치기 · tab 뷰 · ? 도움말 │
└──────────────────────────────────────────────────────────────────────┘
```

- 상단 바: 에이전트 이름(provider) · 세션 제목 · 상태 배지와 경과 시간 · 출력 토큰 합계 · 비용(기록이 있는 provider 만, 없으면 칸 자체를 생략) · 뷰 탭. 리플레이 중이면 상태 배지 자리에 `⏵ 리플레이 3/120 · 12:04:31` 과 진행률.
- 하단 힌트 바: 왼쪽은 주의 요약(실패 수, 30 s 넘은 도구 수, 막힘), 오른쪽은 현재 뷰에서 쓸 수 있는 키. 폭이 모자라면 오른쪽부터 줄인다.

### 1) '지금' 뷰 (기본)

```text
 ┃ ✗ codex · Windows 경로 테스트 실패 — exit 1                       (callout, critical)
 ▶ ● main      스트리밍 적재 테스트 실행 중          bash  12s   311k
   ├ ✓ scout   herdr 플러그인 표면 조사            done  2m03s  42k
   ├ ● review  계획 delta 리뷰                     read  48s   18k
   │   └ 최근  read src/tailer/bytes.rs ✓ · grep "offset" ✓ · read replay.rs ●
   └ ✗ codex   Windows 경로 테스트                 실패  exit 1
```

- 행 = `ListItem`: 들여쓰기(트리 깊이), 상태 `Badge`, 이름(agent_type 또는 id), 의도 텍스트, 현재 도구(`Chip`)와 경과 시간, 토큰.
- 의도 텍스트 우선순위: 현재 실행 중인 도구 호출의 `intent` > 그 에이전트의 `description` > 마지막 `Reasoning` 첫 줄 > 마지막 도구 `summary`.
- 정렬: main 이 맨 위, 자식은 spawn 순서. 단 실패·막힘이 있으면 맨 위 `Callout` 에 올리고(최대 2개, 나머지는 하단 바 숫자), 해당 행에 critical 배지.
- `Enter` 로 선택 행을 펼치면 최근 도구 5개(이름·요약·결과·시간)를 한 줄씩 보여 준다. `Esc` 로 접는다.
- 폭 규칙: < 60열이면 토큰·현재 도구 칸을 숨기고 의도만, 60~99열은 위 배치, ≥ 100열이면 오른쪽에 선택 에이전트 상세 패널(아래 4)을 함께 띄운다.

### 2) 시간 레인 뷰

```text
 시간  12:00 ─────────── 12:02 ─────────── 12:04 ───────────▶ 지금
 main   ━read━━bash━━━━━━━edit━━━━━━━━━━━━━━━━━━━━━━━━━━bash▮▮▮
 scout       ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━✓
 review                  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━▮▮
 codex                        ━━━━━━━━━━━━✗
          ▲ 커서 12:03:10  (←/→ 이동 · space 재생 · G 라이브)
```

- 행 = 에이전트, 가로 = 시간. 도구 호출 구간을 결과 톤으로 칠한다(진행 중 informative, 성공 neutral-muted, 실패 critical). 폭이 충분한 구간에만 도구 이름을 쓴다.
- 축 범위: 세션 첫 시각 ~ `Timeline::now_reference`. 기본은 균일 축(열당 시간 = 범위 / 본문 폭). 기존 `compress_gaps` 는 재생 속도 규칙(bool 필드 `timeline.rs:106`, `compress_gap` `:135`)이라 축에 쓰지 않는다.
- 공백 접기: '활동' 은 어떤 에이전트든 도구 호출 구간(실행 중 포함)이 걸쳐 있거나 fact 가 있는 시각이다. 활동이 하나도 없는 구간이 균일 축 기준 본문 폭의 25 % 를 넘으면 그 구간을 `┆ 12m ┆` 3열로 접는다. 실행 중 도구 구간은 활동이므로 절대 접히지 않고, 마지막 활동 뒤 '지금' 까지의 끝 구간도 접지 않는다(라이브 추종 중 라벨이 벽시계로 바뀌는 일을 없앰). `z` 로 켜고 끈다(기본 켬).
- 커서는 기존 `Timeline` cursor. `←`/`→` 는 한 열씩, `[`/`]` 는 프롬프트 단위로 이동한다.

### 3) 그래프 뷰 (보조)

- 기존 rataflow Sugiyama 레이아웃(`state/graph.rs` `relayout`)을 유지하고, 노드 카드를 `ListItem`+`Badge` 조합으로, 간선을 `stroke` 토큰으로 다시 그린다. 실행 중 간선은 `stroke.informative-solid`.
- 캔버스 바탕·패턴·선택 강조·미니맵 색은 rataflow `Palette` 에서 온다(`Background`·`MiniMap`, `ui/mod.rs:686-694`; 지금은 `Theme::Dark.palette()`, `state/graph.rs:30`). `App` 의 테마로 `rataflow::Theme::Custom(Palette{ canvas_bg: bg.layer-basement, surface: bg.layer-default, muted: fg.neutral-subtle, subtle: stroke.neutral-weak, accent: brand(fg), text: fg.neutral, success: fg.positive, error: fg.critical })` 를 만들어 쓴다. 테마가 바뀌면 팔레트도 다시 만든다.
- 패닝(`h`/`l` 가로, `H`/`J`/`K`/`L` 네 방향)·줌(`+`/`-`, `0`)은 이 뷰에서만 동작한다. `j`/`k` 는 다른 뷰와 같이 선택 이동이다.

### 4) 상세 패널

- 선택 에이전트의 헤더(배지·모델·시작/경과·토큰·비용), 프롬프트(최대 6줄, 기존 `PROMPT_MAX_LINES`), 도구 호출 목록(이름·의도/요약·결과·시간)을 보여 준다. 기존 `ui/panel.rs` 의 정보 구성을 따르되 위젯과 토큰으로 다시 쓴다.
- 폭 ≥ 100열이면 '지금' 뷰 오른쪽 40 %, 그보다 좁으면 `Enter` 두 번째 누름으로 전체 화면 패널.

### 키맵 (전 뷰 공통, 하단 바에 표시)

| 키 | 동작 | 기존 |
|---|---|---|
| `Tab` / `1` `2` `3` | 뷰 전환(지금 · 시간 · 그래프) | 지금 `Tab`/`BackTab` 은 rataflow 선택 다음/이전(`handler.rs:200-201`, rataflow `actions.rs:386-393`) → 뷰 전환으로 바뀜. 테스트 `selection_nav_leaves_the_camera_to_our_glide`(handler.rs:404)를 `j` 로 고친다 |
| `j`/`k`, `↑`/`↓` | 선택 이동(모든 뷰) | `j/k` 는 상세 스크롤(`handler.rs`), `↑/↓` 는 rataflow 공간 선택 → 이제 모든 뷰에서 선택 이동, 상세 패널 안에서는 스크롤 |
| `Enter` / `Esc` | 펼치기·상세 / 뒤로 | `Esc` 유지 |
| `space` | 재생·멈춤 | 유지 |
| `[` `]` | 이전·다음 프롬프트 | 유지 |
| `G` | 라이브로 | `End/g/G` 유지 |
| `←` `→` | 시간 레인: 커서 이동 / 그래프: 공간 선택(좌우) | 지금은 rataflow 공간 선택(SelectLeft/Right, rataflow `actions.rs:401-404`) |
| `h` `l`, `H` `J` `K` `L` | 그래프 뷰 패닝 | 지금은 `h/j/k/l` 패닝(rataflow `actions.rs:407-408`) → `j/k` 는 선택이므로 세로 패닝은 `J/K` |
| `f` | 선택 에이전트 따라가기 | 유지 |
| `z` | 시간 레인: 공백 접기 켜고 끄기 | 없음 |
| `?` | 도움말(전체 키) | 유지 |
| `q` | 종료 | 유지 |

`o`(overview), `r`(relayout), `s`(gap 압축), `i`(info)는 도움말에만 남기고 하단 바에서는 뺀다. 마우스: 행 클릭 = 선택, 탭 클릭 = 뷰 전환, 시간 축 클릭 = 커서 이동.

### 접근성

- 모든 상태는 글리프 + 단어 + 톤을 함께 쓴다(● 실행 중, ✓ 완료, ✗ 실패, ◌ 대기, ⏸ 멈춤, ⏳ 30초 넘음). 색만으로 구분하지 않는다.
- 선택 행은 색 반전 대신 `bg.neutral-weak` + 왼쪽 `▶` 표시.
- 256색 모드와 라이트 테마에서 본문 대비를 스냅샷과 함께 대비 계산 테스트로 확인한다(fg.neutral 대 bg.layer-default 대비 ≥ 4.5:1, fg.neutral-subtle ≥ 3:1).

## 데이터 변경 (fact)

- `FactKind::ToolStart { call, name, summary, intent: Option<String> }`(`src/fact.rs:89-93`). provider 별 채움:
  - omp: `toolCall.intent`(지금 `summary` 로 들어가는 값, `provider/omp/mod.rs:329-346`; omp 규칙은 intent 를 그대로 쓰고 arguments 에서 재구성하지 않음, :329-331)를 `intent` 에도 같은 값으로 채운다. `summary` 는 지금처럼 둔다. 새 인자 요약기는 만들지 않는다(provider 파싱 규칙 변경 금지). 기존 테스트 `tool_call_pairs_with_its_result_and_uses_intent_as_summary`(omp/mod.rs:657)는 그대로 통과해야 한다.
  - Claude: spawn 도구(Agent/Task/Workflow)의 `description`(`claude/mod.rs:447-453`)을 `intent` 로. Bash 의 `description` 이 있으면 그것도 `intent`.
  - Codex: `spawn_agent` 의 `task_name`(`codex/mod.rs:508-513`)을 `intent` 로. 그 밖은 `None`.
- `FactKind::Tokens { output, dedup, cost_usd: Option<f64> }`: omp 는 `usage.cost` 를 채운다(`omp/wire.rs:182-190`, 지금은 읽지 않음). 실제 omp 18.x 세션의 `usage.cost` 는 객체(`{input, output, cacheRead, cacheWrite, total}`)이고 이 머신 200개 세션에서 7,145건 모두 객체였다. 출처는 `usage.cost.total` 이며, 스칼라 숫자(fixture 형태)는 보조로만 받는다. 역직렬화는 `serde_json::Value` 를 받아 직접 해석해, 어떤 형태든 실패해도 그 줄의 나머지(도구 호출·토큰·모델)는 버려지지 않게 한다(omp `parse_line` 은 `from_str(line).ok()` 라 필드 하나의 타입 불일치가 줄 전체를 버린다, `omp/wire.rs:392`). Claude·Codex 는 기록에 비용이 없으므로 `None`(추정하지 않음). 같은 `dedup` 키는 한 번만 더한다(기존 규칙).
- `ToolCallInfo` 에 `intent`, `AgentInfo` 에 `cost_usd: Option<f64>` 합계를 더한다(`state/session.rs:160`, `:191`).
- 골든 영향: 타임라인 골든은 fact 이름만 쓰고(`assets/omp/demo.timeline.txt`), 모델 골든을 만드는 `state/render.rs` 는 summary·intent·cost 를 출력하지 않는다. 따라서 Claude·Codex·omp 골든 전부 무변경이다. 실제 형태(객체 cost)를 담은 테스트 JSON 은 `assets/` 가 아니라 테스트 안에 인라인으로 둔다(G12 보호).

## 성능 영향

- hot path: TUI 루프(16 ms tick)와 그리기. 새 뷰는 매 그리기마다 `SessionModel` 을 순회하므로 에이전트 수 N, 도구 호출 수 M 에 대해 '지금' 뷰 O(N + 펼친 행의 5), 시간 레인 O(N × 본문 폭 + M). M 이 수천(이 머신 omp 세션 633, Claude 1,041)이므로 레인 칠하기는 도구 호출을 시작 시각 정렬 상태로 유지하고 보이는 범위만 이분 탐색으로 자른다. `ToolCallInfo` 는 시작 순서로 쌓인다고 보장되지 않는다: Codex 는 완료 기록에서 `ts=started` 로 `ToolStart` 를 내므로 병렬 호출이면 순서가 뒤집힌다(`codex/mod.rs:355-368`). 그래서 T4 가 시작 시각 정렬을 보장한다.
- 그리기 억제(`RedrawGate`) 술어를 새 뷰 기준으로 다시 정의한다: dirty(tailer 이벤트·입력·resize·뷰 전환), glide(그래프 뷰만), 그래프 뷰의 실행 중 간선 ants phase(기존 `ants` 항목 유지, 마지막 fact 뒤 30 s 안에서만), 보이는 모든 실행 중 도구(비-terminal 에이전트, `ui/chips.rs:286-288` 과 같은 규칙)의 경과 초 문자열(상한 1 s), 하단 바의 '30 s 넘은 도구' 개수, 상단 바 경과 시간 문자열(실행 중이면 now−시작, 아니면 last−first), 스피너 위상(1 s 단위), 시간 레인이 라이브 추종 중이면 열당 시간 경과, snackbar 만료, 상태 강조 150 ms 종료, transport 변화, status tick 변화, Playing. 이 값들은 `FrameStamp` 에 담아 직전 프레임과 비교한다(지금 `FrameStamp` 필드는 transport·ants·chips, `state/frame.rs:22-30`; 기존 계약 `pending_chip_duration_redraws`(:261)는 '보이는 모든 실행 중 도구' 로 일반화해 유지).
- 기존 측정 기준 유지(이전 사이클 G5, `.issueops/issues/1/gates.md:19`): 끝난 세션과 Running main 사본(`scripts/make-running-demo.sh`, 텍스트 assistant 줄만 덧붙임) 모두 ≤ 0.5 %. 새 술어에서 가장 비싼 '실행 중 도구 경과 시간' 경로는 따로 `scripts/make-running-demo.sh --pending-tool` 사본(pending `tool_use` 줄을 덧붙임)으로 측정하고 역시 ≤ 0.5 % 를 요구한다. 측정은 `scripts/idle-cpu.sh` 방식(200x60 pty, `/bin/ps`).
- 테마 감지는 시작 시 1회. OSC 11 이나 DA1 에 응답하는 터미널은 +100 ms 이하, 둘 다 응답하지 않는 터미널은 타임아웃(1 s) 때문에 +1.1 s 이하. `AG_THEME` 을 주면 질의하지 않는다. 적재 경로(스트리밍)는 손대지 않으므로 RSS·첫 결과 시간 기준은 그대로다(G4, G11 재측정).
- 바이너리 크기: `terminal-colorsaurus` 추가분과 chips/panel 삭제분을 비교해 README 에 기록한다.

## 하위 호환성과 side effect

- CLI: 플래그와 서브커맨드(`--provider`, `--follow`, `--speed`, `inspect`, `herdr resolve|toggle`) 변경 없음. 새 환경 변수 `AG_THEME`, `AG_COLOR`, `AG_BG` 는 선택 사항이며 없으면 자동 감지.
- 키맵: `j/k` 의 의미가 "상세 스크롤" 에서 "선택 이동" 으로, `Tab` 이 "그래프 선택 다음" 에서 "뷰 전환" 으로, 그래프 패닝 `j/k` 가 `J/K` 로 바뀐다(상세 패널 안에서는 `j/k` 스크롤). `o/r/s/i` 는 도움말로 이동. CHANGELOG 와 README 에 표로 적는다.
- fact 어휘: `ToolStart`·`Tokens` 에 필드 추가. 이 크레이트의 공개 API(`lib.rs` 의 export)를 쓰는 외부 소비자는 없다(포크 이후 이 저장소만 사용). 생성자 호출부는 provider 와 테스트뿐이므로 컴파일러가 모두 짚는다.
- 골든: 전부 무변경. 새 스냅샷 골든 `assets/ui/<view>-<theme>-<WxH>.txt` 를 추가한다(셀 문자와 스타일 요약을 텍스트로 직렬화).
- 온디스크: 새로 쓰는 파일 없음. herdr 플러그인 브리지·설치 스크립트 변경 없음.
- 라이선스: `design/seed/LICENSE`·`NOTICE` 원문 포함, 저장소 `NOTICE` 갱신, README 에 "당근마켓과 무관" 문구. 로고·상호·캐릭터·carrot 강조색 미사용.
- 롤백: v0.1.0 으로 재설치(`herdr plugin install ... --ref v0.1.0`).
- 데이터베이스·마이그레이션 없음. LLM 프롬프트 본문 변경 없음.

## Work Objectives

### Core Objective

SEED 토큰 위에서 동작하는 독창적인 UI 층으로 바꿔, herdr 패널에서 열자마자 "지금 무엇을 하는지, 어디서 막혔는지, 서브에이전트가 어디까지 했는지" 를 읽을 수 있게 한다. 성능 기준은 그대로 지킨다.

### Deliverables

1. `design/seed/`(YAML·LICENSE·NOTICE·SOURCE.md), `scripts/sync-seed.sh`.
2. `src/ui/seed/{tokens.rs(생성), gen.rs(테스트 생성기), theme.rs, widgets/*.rs}`.
3. `src/ui/views/{now.rs, lanes.rs, graph.rs, detail.rs}`, `src/ui/{mod.rs, chrome.rs(상단·하단 바), text.rs}`.
4. `src/handler.rs` 새 키맵, `src/state/frame.rs` 새 술어, `src/state/mod.rs` 의 뷰·선택 상태.
5. fact/provider/session 의 intent·cost.
6. 스냅샷 골든 `assets/ui/*.txt`, README(새 화면·키·테마 변수·SEED 귀속), NOTICE, CHANGELOG.

### Definition of Done

G1~G14 전부 EVIDENCE 충족.

### Must Have

- `src/ui/seed/` 밖 `src/ui` 와 `src/state` 에 `Color::` 리터럴 0.
- 라이트·다크 × 트루컬러·256 모두에서 렌더.
- 상태 표시는 글리프+단어+톤.

### Must NOT Have

- carrot 을 시맨틱 역할에 매핑, 로고·상호 사용.
- 런타임 YAML 파싱, 네트워크 접근.
- provider 파싱 규칙·`inspect` 출력 변경, 골든 변경.
- 웹 UI, herdr `pane.focused` 자동 전환.

### 이슈 완료 기준 ↔ TODO·게이트

| 이슈 기준 | TODO | 게이트 |
|---|---|---|
| 토큰 고정·귀속·생성 일치 | T1 | G2 |
| `src/ui` 색 리터럴 0 | T2, T5~T7 | G3 |
| 라이트/다크·256 대체 | T2 | G4 |
| '지금' 뷰 요약·트리·의도·callout | T3, T5 | G5, G6 |
| 시간 레인·그래프 뷰 전환 | T6, T7, T8 | G6 |
| 글리프+텍스트 | T2, T5 | G7 |
| 키맵·힌트 바 | T8 | G8 |
| 스냅샷(2 테마 × 2 크기 × 3 뷰) | T10 | G6 |
| 성능 기준 유지 | T9 | G9, G10, G11, G12 |
| CI·머지·릴리스·실설치 | T11 이후(준비 세션) | G13, G14 |

## Verification Strategy

- Test decision: TDD. 토큰 생성기·테마·위젯·술어·키맵은 단위 테스트 먼저. 뷰는 `TestBackend` 스냅샷 골든.
- QA: 에이전트가 실행하는 시나리오만. 실제 herdr 패널 QA 는 준비 세션이 머지 뒤 릴리스로 수행한다. 구현 세션은 pty(`script -q /dev/null`, 200x60·80x24)에서 실제 세션 파일로 화면을 띄워 `pane read` 또는 pty 캡처로 확인한다.
- Evidence: `.issueops/evidence/task-{N}-{slug}.{ext}`.

## Execution Strategy

### Parallel Execution Waves

- Wave 1: T1 토큰 vendoring·생성기, T3 fact intent·cost (독립).
- Wave 2: T2 테마·위젯(T1 뒤), T4 뷰 상태·선택 모델(T3 뒤).
- Wave 3: T5 '지금' 뷰, T6 시간 레인, T7 그래프·상세 재작성, T8 셸·키맵·RedrawGate (T2, T4 뒤), 이어서 T9 성능 재측정, T10 스냅샷·문서.

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|---|---|---|---|
| T1 | — | T2 | T3 |
| T2 | T1 | T5, T6, T7, T8 | T4 |
| T3 | — | T4, T5 | T1 |
| T4 | T3 | T5, T6, T7, T8 | T2 |
| T5 | T2, T4 | T8, T10 | T6, T7 |
| T6 | T2, T4 | T8, T10 | T5, T7 |
| T7 | T2, T4 | T8, T10 | T5, T6 |
| T8 | T5, T6, T7 | T9, T10 | — |
| T9 | T8 | T10 | — |
| T10 | T8, T9 | — | — |

## TODOs

- [ ] 1. SEED 토큰 vendoring 과 생성기
  - What to do: `design/seed/` 에 위 6개 YAML 과 `LICENSE`, `NOTICE` 를 `daangn/seed-design@22b68ce26...` 원문 그대로 받는다. `design/seed/SOURCE.md` 작성. `scripts/sync-seed.sh <sha>`. dev-dependency `yaml-rust2 = "0.13"`. `src/ui/seed/gen.rs`(cfg(test)): 참조 해석(체인 포함, 순환 시 패닉으로 실패), 알파 합성, brand=purple 치환, xterm-256 최근접 계산, Rust 소스 출력. 테스트 `seed_tokens_match_vendored_yaml`: 생성 결과 == `src/ui/seed/tokens.rs`, `UPDATE_SEED=1` 이면 덮어씀. 테스트 `brand_maps_to_purple_not_carrot`: `brand::BG_SOLID` 가 light=purple-600(#9f84fb), dark=purple-700(#a78df0) 이고 carrot 값이 어떤 시맨틱 상수에도 없음. 테스트 `alpha_tokens_are_composited`: `bg.overlay` 가 불투명 Rgb.
  - Must NOT do: 런타임 YAML, carrot 시맨틱 매핑, YAML 편집.
  - Recommended Agent: deep; Reason: 참조 해석·색 계산.
  - Parallelization: YES; Wave 1; Blocks T2; Blocked By —
  - References: https://github.com/daangn/seed-design/tree/22b68ce26/packages/rootage (color.yaml: palette·fg·bg·stroke, `$color.palette.*` 참조, theme-light/theme-dark), npm `@seed-design/rootage-artifacts` 3.0.2; 골든 관례 `src/provider/harness.rs:89-100`.
  - Acceptance Criteria: `cargo test --locked --lib ui::seed::gen 2>&1 | grep 'test result'` 가 `4 passed; 0 failed`(match·brand·alpha·cyclic); `UPDATE_SEED=1 cargo test --locked seed_tokens && git diff --exit-code src/ui/seed/tokens.rs` 가 0; `shasum -a 256 design/seed/*.yaml` 이 `SOURCE.md` 에 적힌 값과 같음.
  - QA Scenarios:
    - Channel: shell. Steps: 위 명령. Expected: 통과, diff 없음. Evidence: `.issueops/evidence/task-1-seed.txt`
    - Channel: shell(실패 경로). Steps: 임시 사본 YAML 에 순환 참조를 넣고 생성기를 그 경로로 실행하는 테스트 `cyclic_reference_is_reported`. Expected: 토큰 이름이 들어간 오류. Evidence: `.issueops/evidence/task-1-seed-error.txt`
  - Commit: YES; Message: `feat(ui): vendor SEED rootage tokens and generate the Rust token module`; Files: `design/seed/**`, `scripts/sync-seed.sh`, `src/ui/seed/{mod.rs,gen.rs,tokens.rs}`, `Cargo.toml`, `Cargo.lock`, `NOTICE`

- [ ] 2. 테마와 SEED 위젯
  - What to do: `src/ui/seed/theme.rs`(모드·깊이 결정, `color()`), 의존 `terminal-colorsaurus = "1.0"`. `src/ui/seed/widgets/{badge,chip,callout,list_item,tablist,progress,snackbar,divider,skeleton,key_hint}.rs` 를 위 표대로. 각 위젯 단위 테스트(`TestBackend` 작은 영역): 톤별 글자색·배경이 토큰과 같고, 글리프·단어가 렌더되는지. 대비 테스트 `body_text_contrast_meets_wcag`(라이트·다크, 트루컬러·256).
  - Must NOT do: 위젯 밖 색 계산, 테마를 전역 static 으로 두기(App 이 소유).
  - Recommended Agent: visual-engineering; Reason: 위젯 시각 규칙.
  - Parallelization: YES; Wave 2; Blocks T5~T8; Blocked By T1
  - References: SEED `components/{badge,chip,callout,list-item,tablist,progress-circle,snackbar,divider,skeleton}.yaml`(variants·tones·slots); `terminal-colorsaurus` 1.0.3 docs(https://docs.rs/terminal-colorsaurus).
  - Acceptance Criteria: `cargo test --locked ui::seed` 전부 통과; `AG_THEME=light` 와 `dark` 에서 같은 위젯의 색이 각 테마 토큰과 일치(테스트); `COLORTERM` 미설정 시 `Color::Indexed` 로 렌더(테스트 `ansi256_fallback`).
  - QA Scenarios:
    - Channel: shell. Steps: 위 테스트. Expected: 통과. Evidence: `.issueops/evidence/task-2-widgets.txt`
    - Channel: shell(실패 경로). Steps: 하위 프로세스로 `TERM=dumb target/release/agents-graph …` 를 띄워(테스트 프로세스 환경은 바꾸지 않음) 첫 화면까지 시간 측정, 그리고 `decide()` 순수 함수 단위 테스트. Expected: 질의 없이 즉시 다크로 결정(+100 ms 이하). Evidence: `.issueops/evidence/task-2-theme-timeout.txt`
  - Commit: YES; Message: `feat(ui): add a SEED theme and terminal widgets`; Files: `src/ui/seed/**`, `Cargo.toml`, `Cargo.lock`

- [ ] 3. fact 에 도구 의도와 비용
  - What to do: `ToolStart.intent`, `Tokens.cost_usd` 추가. provider 채움(위 "데이터 변경"). `ToolCallInfo.intent`, `AgentInfo.cost_usd` 합계(`dedup` 규칙 동일). omp 는 같은 intent 값을 `intent` 와 기존 `summary` 에 함께 채운다(summary 동작 불변). `state/render.rs` 는 바꾸지 않는다.
  - Must NOT do: 비용 추정, 골든 변경, `inspect` 출력 변경.
  - Recommended Agent: deep; Reason: 세 provider 와 골든.
  - Parallelization: YES; Wave 1; Blocks T4, T5; Blocked By —
  - References: `src/fact.rs:89-93`, `src/provider/omp/{mod.rs:296-346,434-441, wire.rs:182-190,265-269}`, `src/provider/claude/mod.rs:201-209,425-458`, `src/provider/codex/mod.rs:222-231,508-513`, `src/state/session.rs:160,191,366,414`, `src/state/render.rs`.
  - Acceptance Criteria: `git diff --exit-code assets/` (골든 무변경); `tool_call_pairs_with_its_result_and_uses_intent_as_summary` 유지; 새 테스트 `omp_tool_intent_and_cost`(인라인 JSON: 객체 `usage.cost.total` 이 `AgentInfo.cost_usd` 로, `intent` 가 `ToolCallInfo.intent` 로; 스칼라 `cost:0.01` 도 받음), `omp_object_cost_keeps_the_rest_of_the_line`(cost 형태가 예상과 달라도 같은 줄의 도구 호출·토큰은 남음), `claude_agent_description_is_intent`, `codex_spawn_task_name_is_intent`, `cost_is_none_without_records` 통과.
  - QA Scenarios:
    - Channel: shell. Steps: 위 테스트와 `cargo test --locked`. Expected: 통과, 골든 diff 없음. Evidence: `.issueops/evidence/task-3-facts.txt`
    - Channel: shell(실패 경로). Steps: `usage.cost` 가 문자열·배열·`total` 없는 객체인 인라인 omp 줄. Expected: 패닉 없이 cost None, 도구 호출·토큰은 집계. Evidence: `.issueops/evidence/task-3-facts-error.txt`
    - Channel: shell(실세션). Steps: 이 머신 omp 실세션으로 `agents-graph inspect`(도구 호출 수가 변경 전과 같음)와 단위 테스트 헬퍼로 `cost_usd` 합계 출력. Expected: 도구 호출 수 동일, 비용 > 0. Evidence: `.issueops/evidence/task-3-facts-real.txt`
  - Commit: YES; Message: `feat(provider): carry tool-call intent and recorded cost`; Files: `src/fact.rs`, `src/provider/**`, `src/state/session.rs`, `src/state/render.rs`(필요 시)

- [ ] 4. 뷰 상태와 선택 모델, 상태→UI 결합 제거
  - What to do: `App` 에 `view: View{Now,Lanes,Graph}`, `selection: Selection{agent: Option<AgentId>, expanded: bool, detail_scroll}`, `snackbar: Option<Snack>`, `theme: Theme` 를 둔다. `state/graph.rs` 의 `ui::nodes`/`ui::edges` 의존을 끊는다: 그래프 노드·간선 콘텐츠 타입을 `ui/views/graph.rs` 로 옮기고 `state/graph.rs` 는 레이아웃 좌표와 id 만 다룬다(rataflow `Flow` 제네릭 인자를 `ui` 쪽 타입으로 두되 `state` 는 trait 경계만 앎) — 옮기기 어려우면 `AgentFlow` 를 `ui/views/graph.rs` 로 통째로 옮기고 `App` 은 `Option<GraphState>` 로 보관한다. `ChipTray` 와 `state/mod.rs:24` 재수출, `scrubber_tally`·`era_cache` 를 제거하고 필요한 값은 뷰가 계산한다. `ToolCallInfo` 가 시작 시각 순으로 쌓이는지 확인하고(아니면 정렬 보장 추가) 시간 레인 이분 탐색 전제를 테스트 `tool_calls_are_start_ordered` 로 고정.
  - Must NOT do: Timeline·SessionModel 의미 변경.
  - Recommended Agent: deep; Reason: 모듈 경계 재배치.
  - Parallelization: YES; Wave 2; Blocks T5~T8; Blocked By T3
  - References: `src/state/mod.rs:24,153-240,984`, `src/state/graph.rs:13-31,80-245`, `src/ui/chips.rs:146-330`, `src/ui/panel.rs:28`(EraCache).
  - Also: `benches/memory.rs:109`, `benches/timeline.rs:136` 이 `app.flow` 를 직접 쓰므로 새 구조에 맞게 고친다(`clippy --all-targets` 대상).
  - Acceptance Criteria: `grep -rn "use crate::ui" src/state | wc -l` 가 0; `cargo clippy --all-targets --locked -- -D warnings` 통과; `cargo test --locked` 통과(삭제된 chips 테스트 제외, 그 의도는 T5 테스트로 대체 — 목록을 PR 에 적음).
  - QA Scenarios:
    - Channel: shell. Steps: 위 grep 과 테스트. Expected: 0, 통과. Evidence: `.issueops/evidence/task-4-state.txt`
    - Channel: shell(실패 경로). Steps: 시작 시각이 역순인 도구 호출 두 개를 넣은 세션 단위 테스트. Expected: 정렬 보장 또는 명시적 정렬로 순서 유지. Evidence: `.issueops/evidence/task-4-order.txt`
  - Commit: YES; Message: `refactor(state): keep view state in App and drop state-to-ui imports`; Files: `src/state/**`, `src/ui/chips.rs`(삭제)

- [ ] 5. '지금' 뷰
  - What to do: `src/ui/views/now.rs`. 위 화면 설계 1) 그대로: callout(실패·막힘 최대 2), 트리 행(`ListItem`·`Badge`·의도·현재 도구 `Chip`·경과·토큰), 펼침(최근 도구 5), 폭 규칙(<60, 60~99, ≥100 + 상세). 의도 우선순위 함수 `intent_line(agent)` 단위 테스트. 경과 시간 문자열은 기존 `fmt_timing` 재사용.
  - Must NOT do: 색 리터럴, 상태를 바꾸는 렌더.
  - Recommended Agent: visual-engineering; Reason: 핵심 화면.
  - Parallelization: YES; Wave 3; Blocks T8, T10; Blocked By T2, T4
  - References: 위 화면 설계, `src/state/session.rs:121-160,191`, `src/ui/panel.rs:503`.
  - Acceptance Criteria: 단위 테스트 `intent_line_prefers_running_tool_intent`, `failures_rise_to_callout`, `narrow_width_hides_tokens` 통과; 스냅샷(T10) 고정.
  - QA Scenarios:
    - Channel: pty. Steps: `script -q /dev/null bash -c 'stty rows 40 cols 120; target/release/agents-graph <omp 실세션> --follow'` 를 3 s 띄우고 화면 캡처. Expected: 상단 요약에 비용, main 아래 서브에이전트 행과 의도 텍스트. Evidence: `.issueops/evidence/task-5-now.txt`
    - Channel: pty(실패 경로). Steps: 실패 도구가 있는 Codex fixture 로 같은 실행. Expected: 맨 위 critical callout 과 ✗ 배지. Evidence: `.issueops/evidence/task-5-now-error.txt`
  - Commit: YES; Message: `feat(ui): add the now view`; Files: `src/ui/views/now.rs`, `src/ui/text.rs`

- [ ] 6. 시간 레인 뷰
  - What to do: `src/ui/views/lanes.rs`. 균일 축 + 공백 접기(위 규칙, compress_gaps 미사용), 열당 시간, 보이는 구간 이분 탐색, 결과 톤 칠하기, 이름 표기 규칙, 커서 표시. 단위 테스트 `lane_columns_cover_session_span`, `running_call_reaches_now_edge`, `visible_slice_uses_binary_search`(호출 수 10,000 에서 비교 횟수 상한), `idle_gap_over_quarter_width_folds_to_three_columns`, `running_tool_span_is_never_folded`, `trailing_gap_is_never_folded`, `z_toggles_gap_folding`.
  - Must NOT do: Timeline cursor 의미 변경.
  - Recommended Agent: deep; Reason: 시간 축 계산.
  - Parallelization: YES; Wave 3; Blocks T8, T10; Blocked By T2, T4
  - References: `src/state/timeline.rs:43,106,135,341,430,485`.
  - Acceptance Criteria: 위 테스트 통과; 스냅샷 고정.
  - QA Scenarios:
    - Channel: pty. Steps: Claude 96 MB 세션 리플레이를 열고 `2` 키, `→` 10회. Expected: 레인과 커서 이동, 축 시각 변화. Evidence: `.issueops/evidence/task-6-lanes.txt`
    - Channel: pty(실패 경로). Steps: 도구 호출 0개 세션. Expected: Skeleton 대신 "기록된 도구 호출이 없습니다" 빈 상태. Evidence: `.issueops/evidence/task-6-lanes-empty.txt`
  - Commit: YES; Message: `feat(ui): add the time-lane view`; Files: `src/ui/views/lanes.rs`

- [ ] 7. 그래프 뷰와 상세 패널 재작성
  - What to do: `src/ui/views/graph.rs`(노드·간선을 위젯과 토큰으로, rataflow `Palette` 를 SEED 토큰으로 채움, 카메라 이동 d6), `src/ui/views/detail.rs`(헤더·프롬프트·도구 목록). 기존 `ui/nodes.rs`, `ui/edges.rs`, `ui/panel.rs` 삭제. 기존 nodes/panel 테스트 중 의미 있는 것(fmt_tokens, resolve_scroll, tool_line)은 새 위치로 옮긴다.
  - Must NOT do: Sugiyama 레이아웃 파라미터 변경.
  - Recommended Agent: visual-engineering; Reason: 렌더 재작성.
  - Parallelization: YES; Wave 3; Blocks T8, T10; Blocked By T2, T4
  - References: `src/ui/nodes.rs:19-275`, `src/ui/edges.rs:18-37`, `src/ui/panel.rs:22-611`, `src/state/graph.rs:80-245`.
  - Acceptance Criteria: 옮긴 테스트 통과; 스냅샷 고정.
  - QA Scenarios:
    - Channel: pty. Steps: omp 실세션, `3` 키. Expected: 그래프와 토큰 색, 실행 중 간선 informative. Evidence: `.issueops/evidence/task-7-graph.txt`
    - Channel: pty(실패 경로). Steps: 서브에이전트 50개 합성 세션. Expected: 패닝 가능, 패닉 없음. Evidence: `.issueops/evidence/task-7-graph-large.txt`
  - Commit: YES; Message: `feat(ui): redraw the graph and detail panel with SEED widgets`; Files: `src/ui/views/{graph,detail}.rs`, 삭제 파일

- [ ] 8. 화면 셸, 키맵, 그리기 억제
  - What to do: `src/ui/mod.rs` 를 새 `draw` 로(상단 바·탭·본문·하단 바·도움말 오버레이·snackbar), `src/ui/chrome.rs`. `src/handler.rs` 키맵 표대로, 마우스(행·탭·축 클릭). `src/state/frame.rs` 술어를 "성능 영향" 의 목록으로 교체하고 테스트: `draw_skipped_when_clean`(유지), `running_tool_elapsed_redraws_once_per_second`(선택하지 않은 자식 행의 도구로 검증; 루트가 Idle 이고 자식 `task` 가 긴 도구를 실행하는 경우 포함), `pending_tool_count_redraws_at_30s`, `lane_live_edge_redraws_per_column`, `snackbar_expiry_redraws_once`, `view_switch_marks_dirty`. `src/tui.rs` 는 `Theme::detect()` 를 원시 모드 진입 전에 호출해 App 에 넣는다.
  - Must NOT do: 기존 동작(재생·스크럽·라이브·따라가기) 제거.
  - Recommended Agent: deep; Reason: 이벤트·그리기 루프.
  - Parallelization: NO; Wave 3; Blocks T9, T10; Blocked By T5, T6, T7
  - References: `src/tui.rs:24-145`, `src/handler.rs:20-253`, `src/state/frame.rs:22-216`, `src/ui/mod.rs:34,569,702`.
  - Acceptance Criteria: 위 테스트 통과; 키맵 테스트 `keymap_matches_table`(표의 각 키 → 동작); `grep -rnE 'Color::' src/ui src/state | grep -v src/ui/seed` 0줄.
  - QA Scenarios:
    - Channel: pty. Steps: omp 실세션에서 `Tab`, `1`,`2`,`3`, `j`,`k`, `Enter`, `Esc`, `?`, `q`. Expected: 각 화면 전환과 종료(rc 0). Evidence: `.issueops/evidence/task-8-keys.txt`
    - Channel: pty(실패 경로). Steps: 40x12 의 아주 작은 창. Expected: "창을 키워 주세요" 안내 한 줄, 패닉 없음. Evidence: `.issueops/evidence/task-8-tiny.txt`
  - Commit: YES; Message: `feat(ui): new shell, keymap and redraw predicates`; Files: `src/ui/{mod,chrome}.rs`, `src/handler.rs`, `src/state/frame.rs`, `src/tui.rs`

- [ ] 9. 성능 재측정
  - What to do: G4·G9~G12 재측정. `scripts/idle-cpu.sh`(PR #2 산출물) 재사용, Running 사본은 `scripts/make-running-demo.sh` 로 측정 직전 생성. `make-running-demo.sh` 에 `--pending-tool` 옵션(마지막에 결과 없는 `tool_use` 를 가진 assistant 줄 추가)을 더한다. 바이너리 크기 전후 비교.
  - Must NOT do: 기준 완화.
  - Recommended Agent: quick; Reason: 측정.
  - Parallelization: NO; Blocks T10; Blocked By T8
  - References: PR #2 의 `scripts/idle-cpu.sh`, `scripts/make-running-demo.sh`, `.issueops/issues/1/gates.md`(이전 사이클 게이트 정의).
  - Acceptance Criteria: G9~G12 EXPECT 충족.
  - QA Scenarios:
    - Channel: shell(pty). Steps: G9~G12 명령. Expected: 기준 이하. Evidence: `.issueops/evidence/task-9-perf.txt`
    - Channel: shell(실패 경로). Steps: `AG_THEME` 미설정·배경 질의 무응답 터미널에서 시작 시간. Expected: 응답하는 터미널(herdr 패널)에서 +100 ms 이하, 무응답 pty 에서 +1.1 s 이하, `AG_THEME=dark` 면 질의 없음. Evidence: `.issueops/evidence/task-9-startup.txt`
  - Commit: NO(evidence 만)

- [ ] 10. 스냅샷 골든, 문서
  - What to do: `src/ui/snapshots.rs`(cfg(test))에 조합마다 `#[test]` 하나씩, 3 뷰 × {light, dark} × {80x24, 120x40} = 12 개를 두고 `assets/ui/*.txt` 로 고정(셀 문자 + 셀별 시맨틱 토큰 이름 요약을 텍스트로; 실제 RGB 가 아닌 토큰 이름이라 테마 값 변경에 흔들리지 않음). 입력은 `assets/omp` fixture 를 `Mode::Replay` 로 끝까지 적재한 상태(now_reference = 커서; `Mode::Live` 라이브 끝은 `Utc::now()` 를 써서 날짜마다 축이 달라지므로 쓰지 않음, `timeline.rs:430-436`). 모든 뷰는 시각을 `App` 의 표시 오프셋(`FixedOffset`, 기본값은 실행 시점 Local 오프셋)으로 포맷하고, 스냅샷 테스트는 UTC 를 주입한다(지금은 `chrono::Local` 로 직접 포맷, `ui/mod.rs:434,545`, `ui/panel.rs:472,515`; Windows 는 TZ 환경 변수를 따르지 않으므로 환경 변수로 고정하지 않는다). 셀 색을 시맨틱 토큰 이름으로 역매핑하지 못하면(토큰 밖 색) 스냅샷 테스트가 실패한다. 역매핑·칠하지 않은 셀 판정 도우미는 `src/ui/seed/snapshot.rs` 에 두어 `src/ui/snapshots.rs` 에 `Color::` 가 나오지 않게 한다(G3). `UPDATE_GOLDEN=1` 관례. README: 새 화면 스크린샷 대신 텍스트 예시, 키 표, `AG_THEME/AG_COLOR/AG_BG`, SEED 귀속과 "당근마켓과 무관" 문구, 키맵 변경 안내. CHANGELOG 0.2.0. NOTICE.
  - Must NOT do: 스냅샷에 실제 세션 내용(개인 정보) 사용.
  - Recommended Agent: quick; Reason: 테스트·문서.
  - Parallelization: NO; Blocked By T8, T9
  - References: `src/provider/harness.rs:89-100`, `assets/omp/*`.
  - Acceptance Criteria: G6 충족; `grep -c "/Users/" README.md` 0; `cargo test --doc` 통과.
  - QA Scenarios:
    - Channel: shell. Steps: `cargo test --locked --lib ui::snapshots::`. Expected: `12 passed; 0 failed`. Evidence: `.issueops/evidence/task-10-snapshots.txt`
    - Channel: shell(실패 경로). Steps: 위젯 패딩을 1 바꾼 뒤 실행. Expected: 스냅샷 diff 로 실패(그다음 되돌림). Evidence: `.issueops/evidence/task-10-snapshots-detect.txt`
  - Commit: YES; Message: `test(ui): snapshot the three views in both themes and document the new UI`; Files: `assets/ui/**`, 테스트, `README.md`, `CHANGELOG.md`, `NOTICE`

## 게이트

| 게이트 | 결과 | CHECK | EXPECT |
|---|---|---|---|
| G1 | 전체 테스트 | `cargo test --locked > /tmp/g1.log 2>&1; echo rc=$?; grep -E '^test result' /tmp/g1.log \| grep -vc ' 0 failed'` | `rc=0` 과 `0` |
| G2 | 토큰 생성 일치 | `UPDATE_SEED=1 cargo test --locked seed_tokens >/dev/null 2>&1; git diff --exit-code design src/ui/seed/tokens.rs; echo rc=$?` | `rc=0` |
| G3 | 색 리터럴 0 | `grep -rnE 'Color::' src/ui src/state \| grep -v '^src/ui/seed/' \| wc -l` | `0` |
| G4 | 테마·256 대체 | `cargo test --locked ui::seed::theme 2>&1 \| grep 'test result'` | `0 failed` 와 passed ≥ 3 |
| G5 | 의도·비용 데이터 | `cargo test --locked --lib -- omp_tool_intent_and_cost omp_object_cost_keeps_the_rest_of_the_line claude_agent_description_is_intent codex_spawn_task_name_is_intent 2>&1 \| grep 'test result'` | `4 passed; 0 failed` |
| G6 | 뷰 스냅샷 12 | `cargo test --locked --lib ui::snapshots:: 2>&1 \| grep 'test result'` | `12 passed; 0 failed` |
| G7 | 상태 글리프+단어 | `cargo test --locked badge_renders_glyph_and_word 2>&1 \| grep 'test result'` | `1 passed` |
| G8 | 키맵 | `cargo test --locked keymap_matches_table 2>&1 \| grep 'test result'` | `1 passed` |
| G9 | RSS | `/usr/bin/time -l target/release/agents-graph inspect --provider codex <521MB> 2>&1 \| grep 'maximum resident'` | ≤ 104857600 |
| G10 | 유휴 CPU | `scripts/idle-cpu.sh assets/claude/demo.jsonl; scripts/make-running-demo.sh /tmp/demo-running.jsonl && scripts/idle-cpu.sh /tmp/demo-running.jsonl; scripts/make-running-demo.sh --pending-tool /tmp/demo-pending.jsonl && scripts/idle-cpu.sh /tmp/demo-pending.jsonl` | 세 값 모두 ≤ 0.5 |
| G11 | 실파일 전수 | `AG_REAL_SESSIONS=1 cargo test --release --test real_sessions -- --nocapture 2>&1 \| grep -E '^real_sessions:'` | `failed=0` |
| G12 | 골든 무변경 | `git diff --exit-code 1c744d2 -- assets/claude assets/codex assets/omp; echo rc=$?` | `rc=0` |
| G13 | CI | PR 의 `gh pr checks` | 전 job pass |
| G14 | 실설치 QA | (준비 세션) `gh release download v0.2.0 -p 'agents-graph-aarch64-apple-darwin.tar.gz' -O - \| tar tz \| grep -cE 'LICENSE-APACHE-SEED\|NOTICE-SEED'` 그리고 `herdr plugin install …` 후 omp 패널에서 토글 2회, `pane read` | `2`; 새 상단 바·'지금' 트리가 보이고 닫힘 |

## Final Verification Wave

- F1 Plan Compliance Audit: T1~T10 evidence 대조.
- F2 Code Quality Review: `cargo clippy --all-targets -- -D warnings`, 죽은 코드(삭제한 chips/panel/nodes 잔재) 없음.
- F3 Real Manual QA: pty 로 세 뷰·두 테마 화면 캡처.
- F4 Scope Fidelity Check: Must NOT Have 위반 없음(carrot 매핑·로고 0, 골든 무변경).

## Commit Strategy

TODO 마다 Conventional Commit + Lore 본문. 저장소 `committed.toml` 규칙(build/ci 허용).

## Success Criteria

이슈 #6 완료 기준 전부(대응표)와 G1~G12 EVIDENCE. G13·G14 는 PR 이후 준비 세션이 확인한다.
