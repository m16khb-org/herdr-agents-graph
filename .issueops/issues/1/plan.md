# herdr-agents-graph MVP: zoetrope 를 기반으로 더 개선·최적화한 세 에이전트 세션 그래프 플러그인

- lifecycle ID: io-4b513b64dc69
- 이슈: https://github.com/m16khb-org/herdr-agents-graph/issues/1
- 브랜치: `1-agents-graph-mvp` (base `main`, 봉인 SHA 는 record 에 있음)
- 사용자 요청 범위: 자료조사 보고서(docs/research/report.md)에 근거해 이슈를 만들고 IssueOps 흐름으로 진행한다. 사용자 추가 지시: "zoetrope 를 참고하되 더욱 개선되고 최적화된 품질로 만들어야 하고, zoetrope 는 추후 깔끔하게 정리한다". 승인된 종료점은 draft PR 발행과 `execution complete` 다. merge 와 머지 후 정리, 이 머신의 zoetrope 제거는 별도 승인이다.
- 세션 인계: 이 계획을 스테이징한 뒤 `execution prepare --mode direct` 로 워크트리를 만들고, 준비 세션이 Herdr 안에서 실행 중이므로 Herdr 로 새 세션을 열어 인계한다. 인계 뒤 이 세션은 구현하지 않는다.
- 계획 개정 이력: 1차 적대 리뷰(revise, 지적 11건), 2차 delta 리뷰(revise, 새 지적 6건: TUI 애니메이션 술어, Codex 토큰 권위 소스, 저장소 비공개 전제, G1 rc, G8 순서, G5 pty 크기), 3차 delta 리뷰(revise, 새 지적 3건: Running 구간 60 fps 재그리기, auto-pan dirty 누락, `token_usage_record` 보류분 방출 훅 부재), 4차 delta 리뷰(revise, 새 지적 2건: pending chip 경과 시간·afterglow fade 정지, G5 Running 사본의 시간 경과 전제)를 반영했다. 바뀐 점은 각 TODO 에 "리뷰 반영" 으로 표시했다. 저장소는 2차 리뷰 뒤 사용자 승인으로 공개 전환했다.

## TL;DR

- Summary: `furkankly/zoetrope@b1f31dd` 를 이 저장소 루트로 가져와 정체성을 `herdr-agents-graph` 로 바꾸고(zoetrope 전용 잔재 제거), 업스트림 PR #26 을 얹어 omp provider 와 네 형식 판별을 넣는다. 대용량 적재 스트리밍, TUI 유휴 재그리기 억제와 폴링 backoff, herdr 브리지의 `herdr` 서브커맨드(jq 제거, `kind: path`), 체크섬 검증 릴리스 설치를 더한다.
- Deliverables: `agents-graph` 바이너리(Claude Code·Codex·omp), `herdr-plugin/` 브리지, 태그 트리거 릴리스 workflow 와 `SHA256SUMS`, 벤치 비교표가 있는 README, NOTICE.
- Effort: Large
- Parallel: YES — 3 waves
- Critical Path: T1(포크·정체성) → T2(omp provider) → T6(실파일 전수) → T9(벤치표) / T7·T8 → T10(실 패널 QA)

## Context

### Original Request

사용자: "첨부한 docs/research/task.md 는 이 저장소의 첫 태스크입니다. … 자료조사를 하고 docs/research/report.md 를 커밋해 주세요." → "경쟁 플러그인들을 조사하고 내가 만든 플러그인의 품질과 최적화가 최고여야해" → "다 정리 완료되면 자료조사된 내용을 기반으로 이슈를 만들고 로 진행" → "zoetrope를 참고하되 더욱 개선되고 최적화된 품질로 만들어야해 그리고 zoetrope는 추후 깔끔하게 정리해줘".

### Interview Summary

인터뷰 없이 보고서와 이슈 본문으로 범위를 확정했다. 사용자 결정 두 건: (1) origin 이 비어 있어 base SHA 를 봉인할 수 없으니 main 을 push 할지 → 승인. (2) zoetrope 는 참고 기반이고 결과물은 그보다 나아야 하며 zoetrope 정리는 후속 → record 의 scope 결정으로 기록.

### Gap Analysis

- PR #26 은 2026-09-13 기준이라 그 뒤 omp 가 추가한 레코드(`credential_pin`, `title_change`, `tool_execution_start` custom)와 자식 완료 결과 파일(`<Name>.md`, `<Name>.json`)을 모른다 [미확인 가정: PR #26 diff 를 아직 읽지 않았다]. T2 에서 diff 를 읽고 이 머신의 실파일로 보완한다.
- 서브에이전트 종료 신호: omp 는 종료 레코드가 없다. 부모 `toolResult(toolName:"task")` 도착, 자식 옆 `<Name>.md` 생성, `.tombstone` 셋을 `Ended` 로 매핑한다. 기준은 "부모 toolResult 가 가장 확실, 나머지는 보조".
- 유휴 CPU 의 실제 원인(리뷰 실측): `src/tui.rs:22` `TICK = 16 ms` 와 `:100` 의 무조건 `terminal.draw` 로 약 60 fps 재그리기를 해 완료된 세션을 띄워 둔 30 s 동안 CPU 약 4 % 를 썼다. 200 ms 폴링의 stat 은 그 일부에 불과하다. 그래서 T4 는 TUI 와 tailer 를 함께 다룬다.
- 대용량 전체 적재의 실제 경로(리뷰 확인): `src/tailer/replay.rs:141-157 parse_file_into` 의 `std::fs::read` + `from_utf8_lossy`(herdr 패널의 `--follow <file>` 이 `main.rs:243-251` 에서 replay=true 로 이 경로를 탄다), `src/main.rs:184` 의 Tail 분기, live 첫 폴의 `src/tailer/bytes.rs:80-90` `vec![0u8; to_read]`. `live.rs:430` 과 `replay.rs:115` 는 `ReadMode::Whole` sidecar(Claude `agent-*.meta.json`) 읽기이며 `serde_json::from_str(text)` 로 문서 전체를 파싱하므로 줄 단위로 바꾸면 안 된다.
- Codex 토큰(리뷰 확인): `codex/wire.rs:327-329` 와 `codex/mod.rs:239-265` 가 `token_count` 의 total 증가분을 이미 `Tokens{dedup: None}` 으로 낸다. `harness::conform` 은 `model.txt` 와 `timeline.txt` 를 함께 비교하므로(`harness.rs:33,85`) Tokens 의 발생 시각이 바뀌면 골든이 바뀐다. rollout 한 파일은 한 스레드(`codex/mod.rs:3-4`)라 `token_usage_record` 의 `thread_id` 로 얻을 추가 귀속 정보는 없다. 2차 리뷰 실측: 521 MB rollout 의 `token_usage_record.usage.output_tokens` 합 262262 와 `token_count` 최종 total 246129 는 다르다(응답 3건에서 total 이 늘지 않음). 결정: 집계의 권위 소스는 종전대로 `token_count` 하나다. `token_usage_record` 는 wire 타입으로 파싱해 `token_count` 가 한 번도 나오지 않은 파일에서만 대체 소스로 쓴다.
- `[[build]]` 는 `HERDR_PLUGIN_*` 를 받지 않는다(herdr v0.9.3 `src/cli/plugin.rs:1532-1550` 이 `HERDR_PLUGIN_` 접두 변수를 모두 지움; plugins.mdx 228-229행). 설치 위치는 체크아웃 상대 경로여야 한다.
- `herdr plugin pane open --placement` 는 `overlay|split|tab|zoomed` 만 받고 popup 은 pane id 가 없다. popup 액션은 만들지 않는다.

## 적용되는 결정과 주의사항

대조했으나 없음. 이 저장소에는 `.issueops/` 운영 문서가 없다(`ls -a` 결과 `.git`, `docs` 뿐). `issueops docs --json` 은 설치본 저장소(issueops_root)의 문서를 가리키므로 이 저장소의 제약이 아니다. 대신 record 의 결정 3건이 제약이다.

- 결정(scope) "no split": 한 PR 로 검토한다. 하위 이슈를 만들지 않는다.
- 결정(architecture) "zoetrope 포크를 이 저장소에 가져오고 herdr-plugin/ 하위 디렉터리로 브리지를 둔다": 설치 경로는 `<owner>/herdr-agents-graph/herdr-plugin`. 원본 LICENSE(MIT) 와 저작권을 유지한다.
- 결정(scope) "zoetrope 는 참고 기반이며 결과물은 더 개선·최적화한다; zoetrope 정리는 후속": 성공 기준은 zoetrope 동등이 아니라 초과(RSS, 유휴 CPU, 오분류 0, 런타임 의존 0). T1 에서 zoetrope 전용 잔재를 제거해 저장소 정체성을 바꾼다. 바이너리는 플러그인 체크아웃 안(`herdr-plugin/bin/`)에만 두고 PATH 에 올리지 않아 기존 brew `zoe` 와 충돌하지 않는다. 이 머신의 `furkankly.zoetrope` 플러그인과 brew `zoetrope` 제거는 MVP 설치 검증 뒤 별도 승인으로 한다(아래 "후속").
- 보고서 §6 의 위험: omp 파일은 제목 변경·세션 전환·분기 때 rename 교체된다(`@oh-my-pi/pi-coding-agent/src/session/session-manager.ts:1677 #rewriteAtomically`, 호출부 2251·2301·2412·2614·2633). T3 의 테스트 `tail_reattaches_after_rename` 과 G10 으로 고정한다.

## 재사용하는 기존 구현

| 재사용 | 위치 | 방식 |
|---|---|---|
| Provider enum 과 프리미티브 7개 | zoetrope `src/provider/mod.rs` (`all_paths`, `session_file`, `related_paths`, `project_key`, `session_file_from`, `stream_for`, `sidecar`) | `Provider::Omp` arm 추가. trait 로 바꾸지 않는다(파일 헤더 주석이 enum 을 의도적으로 택함) |
| `provider_of` | `src/provider/mod.rs:135-144` | `type` 이 있으면 Claude 로 폴백하는 143행을 네 형식 판별로 교체. Claude 판별은 PR #26 과 같이 `type` + (`sessionId` 또는 `uuid`)(리뷰 반영: 기존 테스트 `mod.rs:576-579` 와 이 머신의 Claude 1행 177개는 `uuid` 없이 `sessionId` 만 있음) |
| Fact 어휘 | `src/fact.rs:53-108` `FactKind` 14종, `AgentKind`, `AgentStatus` | 변경 없음. omp 를 이 어휘로 매핑 |
| 골든 테스트 하네스 | `src/provider/harness.rs:37 conform()`, `UPDATE_GOLDEN=1` | `assets/omp/` fixture 와 `demo.model.txt`·`demo.timeline.txt` 골든 추가 |
| tailer | `src/tailer/bytes.rs` TailState(offset, partial, identity dev/ino, `:49-143` Reset), `src/tailer/live.rs:27` `POLL_INTERVAL=200ms`, `:33` `SWITCH_SCAN_EVERY=10`, `:39` `SWITCH_IDLE_TICKS=150`, `:244-342` 폴 루프 | 폴링 루프는 유지하고 간격만 유휴 backoff 로 바꾼다. 첫 폴의 전체 할당(`bytes.rs:80-90`)은 청크 읽기로 바꾼다 |
| TUI 루프 | `src/tui.rs:22` `TICK=16ms`, `:57` interval, `:100` `terminal.draw` | 변화 없을 때 draw 를 건너뛰는 dirty 플래그 추가 |
| Codex wire | `src/provider/codex/wire.rs:327-329` TokenCount, `codex/mod.rs:239-265` 증가분 집계, `codex::Stream`(`mod.rs:31-44`, 파일별 상태) | `token_usage_record` 를 wire 타입으로 추가해 읽되, 집계는 `token_count` 가 그 파일에 없을 때만 대체한다(2차 리뷰 반영: 골든·총합 무변경) |
| 릴리스 workflow | `.github/workflows/cd.yml:24-80` matrix 5 target·cross | matrix 와 빌드 단계만 재사용. 트리거·서명·업로드·homebrew 단계는 교체(리뷰 반영: `:6-8` release-plz 트리거, `:103-108` GPG, `:130` GH_TOKEN, `:136-145` homebrew tap 은 이 저장소에서 동작하지 않음) |
| 브리지 스크립트 | `herdr-plugin/herdr/{pane.sh, resolve.sh, open.sh, ensure-zoe.sh}` | 구조만 참고. 판단 로직은 전부 `agents-graph herdr …` 서브커맨드로 옮기고 셸은 호출 한 줄만 남긴다 |
| 업스트림 PR #26 | https://github.com/furkankly/zoetrope/pull/26 (head `71e8345`, 파일: `src/provider/pi/*`, `assets/omp/*`, `assets/pi/*`, `herdr-plugin/herdr/resolve.sh`) | cherry-pick 후 이 머신의 omp 18.8.4 실파일로 보완. pi arm 은 지원 대상이 아니므로 제거한다(모듈 이름 `omp`) |

새로 만드는 것: `agents-graph herdr {resolve,toggle}` 서브커맨드, 스트리밍 적재 함수, backoff 간격 계산, TUI dirty 플래그, `scripts/idle-cpu.sh`, `tests/real_sessions.rs`. 모두 기존 모듈에 함수 단위로 들어가며 새 크레이트나 추상 계층을 만들지 않는다.

## 성능 영향

- hot path 는 (a) 초기 적재, (b) TUI 재그리기, (c) 폴링 루프다.
- (a) 현재 `replay.rs:141-157` 이 파일 전체를 `std::fs::read` 하고 `main.rs:184` 가 `read_to_string` 한다. 실측 RSS: 96 MB Claude 세션 110 MB, 521 MB Codex rollout 561 MB. 변경: `BufReader::read_until(b'\n')` 로 줄 바이트를 세며 `Stream::push(&str)` 에 넘기고 원문은 버린다. 마지막 개행까지 소비한 바이트를 TailState offset 으로 넘겨 follow 가 이어받는다(리뷰 반영: `BufReader::lines()` 는 개행을 버려 offset 을 못 센다). 첫 폴의 `vec![0u8; to_read]` 는 1 MiB 청크 반복으로 바꾼다. 복잡도 O(bytes) 그대로, 메모리 O(facts). 목표: 521 MB 파일 RSS ≤ 100 MB, 96 MB 파일 첫 결과 ≤ 0.30 s.
- (b) `tui.rs` 는 16 ms 마다 무조건 그린다(`tui.rs:6` 주석 "Draws every iteration so animation never freezes"). 변경: 매 틱 draw 를 "dirty 또는 보이는 프레임이 바뀜" 일 때만 한다. dirty 는 tailer 이벤트 수신, 키·마우스 입력, resize, 그리고 `tick_auto_pan` 이 `EventResponse::Event(_)` 를 돌려주거나 `flow.is_dragging()` 일 때 세운다(3차 리뷰 반영: rataflow `state/auto_pan.rs:72-140` 은 viewport 를 실제로 옮길 때만 `Event` 를 돌려주고 `tui.rs:73` 은 그 반환값을 버린다). 보이는 프레임 변화 술어(2·3·4차 리뷰 반영)는 (1) `camera_glide.is_some()`(`state/mod.rs:81 GLIDE_SECS=0.5`, `:924-939 tick_camera`) 이면 매 틱, (2) marching ants(`state/graph.rs:181` animated 간선)는 rataflow 애니메이션 phase(120 ms, `ANIMATION_PATTERN_LENGTH=3`·`animation_speed_ms` pub) 가 바뀔 때만, 그것도 마지막 fact 수신 뒤 30 s 안일 때만(30 s 가 지나면 ants 는 정지), phase 는 `tick_animation` 에 넘기는 같은 `elapsed.as_millis()` 를 App 이 누적한 mirror 로 계산한다(Instant 벽시계로 따로 재면 절삭 drift 가 생김), (3) 보이는 pending 단일 도구 chip 의 `fmt_dur(duration(now))` 문자열이 바뀌는 시각(`ui/chips.rs:392-399`, `ui/mod.rs:687`; 1 s 미만 ms·10 s 미만 0.1 s·그 이상 1 s 단위이므로 간격 상한 1 s), (4) afterglow chip 이 0.45·0.75·1.0×TTL 경계를 넘는 시각(`ui/chips.rs:419-438` text→subtle→muted fade, TTL 2.5 s/4 s), (5) `transport()` 값이 직전 프레임과 다를 때(`state/mod.rs:276-278` Live→Idle, `LIVE_FRESH=10 s`), (6) status tick 의 상태 변화, (7) autopilot 활성, (8) Playing(`mod.rs:283`) 이면 매 틱. 노드 맥동(`ui/nodes.rs:122` `(phase/4)%2`)은 0.2.0 에서 phase 가 {0,1,2} 뿐이라 한 번도 그려지지 않으므로(4차 리뷰 확인) 술어에 넣지 않는다. interactive main 은 마지막 활동 뒤 `INTERACTIVE_IDLE_SECS=120`(`state/session.rs:22,741-748`) 동안 Running 으로 남지만 Running 자체는 draw 사유가 아니다. 30~120 s 무변화 구간에서 그리는 횟수는 pending chip 이 있으면 최대 1 회/초, 없으면 0 이다(4차 리뷰 추정 약 0.07 %, 실측으로 확인). tick 간격은 유지한다(입력은 `select!` 의 event_rx 로 즉시 깨어남).
- (c) 폴링: 마지막 변화 뒤 30 s 가 지나면 간격을 200 → 500 → 1000 → 2000 ms 로 늘리고(상한 2 s), 변화를 읽으면 200 ms 로 복귀. `SWITCH_IDLE_TICKS`(150틱) 는 시간 기준 30 s 로 바꿔 간격 변화와 무관하게 한다. 목표 유휴 CPU ≤ 0.5 %.
- 측정 계획: `/usr/bin/time -l target/release/agents-graph inspect …` 로 RSS 와 wall. 유휴 CPU 는 `scripts/idle-cpu.sh <file>` 이 `script -q /dev/null` pty 안에서 `--follow` 를 띄우고 30 s 간 `ps -o time=` 차분으로 % 를 계산한다(리뷰 반영: tty 없는 셸에서는 terminal 초기화가 실패한다). 기준선과 같은 파일(96 MB Claude, 521 MB Codex)로 README 표에 기록. `benches/memory.rs` 를 omp fixture 에도 돌린다.

## 하위 호환성과 side effect

- CLI: 바이너리 이름을 `agents-graph` 로 바꾼다(zoetrope 정체성 제거, brew `zoe` 와 구분). 플래그 `--provider`, `--follow`, `--speed`, `inspect` 는 유지하고 `--provider omp` 와 `herdr resolve|toggle` 서브커맨드가 추가된다. 이 저장소에는 기존 호출자가 없다.
- 크레이트: `[package] name = "herdr-agents-graph"`, `[lib] name = "agents_graph"`, `[[bin]] name = "agents-graph"`. `use zoetrope::` 참조 4곳(`src/main.rs:21-25`, `benches/memory.rs:18-19`, `benches/timeline.rs:21-24`, `benches/common/mod.rs:178-183`)을 바꾸고 `Cargo.lock` 루트 항목을 갱신해 커밋한다(리뷰 반영).
- 골든: `assets/claude/*`, `assets/codex/*` 골든은 바꾸지 않는다. T5 의 수용 기준이 "codex 골든 무변경" 이다.
- 온디스크: 플러그인은 세션 파일을 읽기만 한다. 쓰는 곳은 `HERDR_PLUGIN_STATE_DIR/open-pane`(토글용 패널 id 한 줄)과 `herdr-plugin/bin/agents-graph`(설치 바이너리, `.gitignore` 등록) 둘뿐이다. 실패 시 남는 상태: 다운로드 중단이면 임시 디렉터리만 남고 다음 설치가 덮어쓴다; 토글 파일이 가리키는 패널이 사라졌으면 `pane get` 실패를 "닫힘"으로 보고 지운다.
- herdr 계약: 0.9.3 의 `agent_session{kind:id|path}`, `plugin.pane.open/close`, `HERDR_PLUGIN_CONTEXT_JSON.focused_pane_id`, `[[build]]` 는 `HERDR_PLUGIN_*` 없이 실행. `min_herdr_version = "0.9.3"` [미확인 가정: 0.8.2 의 kind 규칙은 확인하지 않았고 0.9.3 소스 `src/agent_resume.rs:126-132` 만 확인함].
- 라이선스: MIT 원본 LICENSE 유지, `NOTICE` 에 zoetrope 저작권·포크 출발 커밋·PR #26 저자 표기.
- 롤백: `herdr plugin uninstall m16khb.herdr-agents-graph`. 업스트림 zoetrope 플러그인과 공존 가능(플러그인 id 와 바이너리 이름이 다르고 바이너리는 체크아웃 안에 있음).
- CI: `web/` 제거에 따라 `ci.yml:74-88` wasm job, `website.yml`, `release-plz.yml`, `dependabot.yml:24` 의 web 항목을 지우고 `ci.yml:126` msrv 선택자를 새 패키지 이름으로 고친다.
- 데이터베이스·마이그레이션 없음.

## Work Objectives

### Core Objective

herdr 에서 omp·Claude Code·Codex 패널 어느 쪽이든 같은 키로 세션 흐름 그래프를 열고 닫는다. 측정 기준(RSS, 첫 결과 시간, 유휴 CPU, 오분류 0, 런타임 의존 0)을 README 에 증명한다.

### Deliverables

1. 크레이트 루트: 포크 + `src/provider/omp` + 스트리밍 적재 + TUI dirty 플래그·backoff + Codex 스레드별 토큰 + `herdr` 서브커맨드.
2. `herdr-plugin/`: `herdr-plugin.toml`, `herdr/{pane.sh, open.sh, install.sh}`, `bin/`(ignored).
3. `.github/workflows/{ci.yml, cd.yml}` (태그 트리거, 5 target, `SHA256SUMS` 집계 job).
4. `README.md` 벤치 비교표, `NOTICE`, `scripts/idle-cpu.sh`.

### Definition of Done (verifiable conditions with commands)

G1..G12 전부 EVIDENCE 충족.

### Must Have

- omp 자식 파일을 부모 `task` 호출에 `tasks[].name` ↔ `<Name>.jsonl` 로 조인. 이름이 중복되면 순서대로 두 번째 호출에 붙인다(PR #26 규칙).
- 모르는 레코드 타입 무시, 패닉 0.
- 이슈 완료 기준 10개 각각에 담당 TODO 와 게이트가 있다(아래 대응표).

### Must NOT Have

- `pane.focused` 구독 자동 전환, omp 분기 트리, 노드별 비용 표시(후속 이슈).
- herdr 설정·다른 플러그인 변경, 세션 파일 쓰기, 다른 사람의 패널에 키·텍스트 전송.
- trait 기반 provider 추상화, 새 워크스페이스 멤버, popup 액션.
- 이 머신의 zoetrope 플러그인·brew 제거(후속, 별도 승인).

### 이슈 완료 기준 ↔ TODO·게이트 대응

| 이슈 기준 | TODO | 게이트 |
|---|---|---|
| omp·Claude·Codex 패널에서 같은 키로 열고 닫힘 | T7, T10 | G9 |
| omp inspect 조인·자동 감지 무오분류 | T2 | G2, G3 |
| `cargo test --locked` 통과 | T1~T5 | G1 |
| 실파일 전수 패닉 0 | T6 | G6 |
| 521 MB RSS ≤ 100 MB, 96 MB 첫 결과 ≤ 0.3 s | T3 | G4, G11 |
| 유휴 CPU ≤ 0.5 % | T4 | G5 |
| rename 교체 추종 | T3 | G10 |
| Codex `token_usage_record`·`token_count` 읽기 | T5 | G12 |
| `herdr plugin install` 이 jq·cargo·brew 없이 성공 | T8, T10 | G8, G9 |
| README 비교표 | T9 | F3 |

## Verification Strategy

- Test decision: TDD(RED→GREEN) for provider·tailer·tui 변경; 기존 `cargo test --locked` 유지. 프레임워크는 Rust 내장 테스트 + `harness::conform` 골든.
- QA: 에이전트가 실행하는 시나리오만. herdr 패널 QA 는 이 세션의 herdr 서버에서 자기 패널만 만들고 닫는다(`herdr pane split` 으로 만든 자기 패널에서 `claude`/`codex` 를 실행; 다른 패널에는 입력하지 않음).
- Evidence: `.issueops/evidence/task-{N}-{slug}.{ext}` (워크트리의 ignored 영역).

## Execution Strategy

### Parallel Execution Waves

- Wave 1: T1 포크·정체성 (단독, 나머지의 전제).
- Wave 2: T2 omp provider·네 형식 판별, T3 스트리밍 적재·rename 추종 → T5 Codex 토큰 대체 소스(T3 뒤, 같은 담당), T4 TUI dirty·backoff, T7 브리지 서브커맨드, T8 릴리스 workflow·설치 스크립트.
- Wave 3: T6 실파일 전수 검증, T9 벤치 비교표·README·NOTICE, T10 herdr 실제 패널 QA(세 에이전트 + `plugin install --ref`).

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|---|---|---|---|
| T1 | — | T2..T10 | — |
| T2 | T1 | T6, T7, T9, T10 | T3, T4, T5, T8 |
| T3 | T1 | T6, T9 | T2, T4, T5, T7, T8 |
| T4 | T1 | T9 | T2, T3, T5, T7, T8 |
| T5 | T1, T3 | T6 | T2, T4, T7, T8 |
| T6 | T2, T3, T5 | T9 | T4, T7, T8 |
| T7 | T1, T2 | T10 | T3, T4, T5, T8 |
| T8 | T1 | T10 | T2..T7 |
| T9 | T3, T4, T6 | — | T10 |
| T10 | T7, T8 | — | T9 |

## TODOs

- [ ] 1. zoetrope 포크 가져오기와 저장소 정체성 전환
  - What to do: `git fetch https://github.com/furkankly/zoetrope b1f31dd26bd4e9e513885e39edb78d0850a5d1fe` 후 `git merge --allow-unrelated-histories` 로 루트에 가져온다(히스토리 보존). zoetrope 전용 잔재 제거: `web/`(사이트·WASM), `assets/*.tape`·`*.gif`·`*.mp4`·`og*.png`·`social-preview.png`·`apple-touch-icon.png`·`favicon*`·`icon.svg`·`mark.svg`·`fonts/`, `project-spotlight.md`, `CLAUDE.md`, `release-plz.toml`, `.github/workflows/{website.yml,release-plz.yml}`, `ci.yml` 의 wasm job(74-88행), `dependabot.yml` 의 web 항목. `Cargo.toml`: `exclude = ["web/wasm"]` 삭제, `[package] name = "herdr-agents-graph"`, `version = "0.1.0"`, `[lib] name = "agents_graph"`, `[[bin]] name = "agents-graph"`; `use zoetrope::` 4곳을 `use agents_graph::` 로; `ci.yml:126` msrv 선택자를 `.name == "herdr-agents-graph"` 로; `Cargo.lock` 갱신(`cargo update -p zoetrope` 가 아니라 `cargo build` 가 만든 lock 을 커밋). `lib.rs:13` 의 `include_str!("../README.md")` 가 있으므로 README 는 T9 전까지 짧은 임시 본문으로 둔다. `docs/research/` 유지. `NOTICE` 에 출처·커밋·라이선스 기록.
  - Must NOT do: 이름 변경과 잔재 제거 이외의 소스 수정, `cliff.toml`·`committed.toml`·`typos.toml` 삭제(커밋 규칙·오타 검사는 재사용).
  - Recommended Agent: quick; Reason: 기계적 작업.
  - Parallelization: NO; Wave 1; Blocks T2..T10; Blocked By —
  - References: 업스트림 `Cargo.toml:1-14`(workspace exclude 주석), `src/main.rs:21-25`, `benches/{memory.rs:18-19,timeline.rs:21-24,common/mod.rs:178-183}`, `.github/workflows/ci.yml:74-88,126`. (리뷰 반영: 패키지 이름 변경 시 lib 이름·lock·workflow 선택자 동반 수정)
  - Acceptance Criteria: `cargo build --locked` 성공; `cargo test --locked 2>&1 | grep -E "^test result"` 의 passed 합이 업스트림 b1f31dd 를 `/tmp` 의 별도 clone 에서 같은 명령으로 돌린 값과 같음(2차 리뷰 반영: herdr 관리 체크아웃 안에서 cargo 를 돌리지 않는다); `grep -rn "zoetrope" src benches Cargo.toml` 이 NOTICE·주석의 출처 표기 외 0건(inline 경로 `main.rs:168,171,173,219,310,324,475`, `benches/timeline.rs:29` 포함); `ls web 2>&1` 이 "No such file".
  - QA Scenarios:
    - Channel: shell. Steps: `cargo run -- inspect --provider claude assets/claude/demo.jsonl`. Expected: 종료 코드 0, 출력에 `agent(s)` 포함. Evidence: `.issueops/evidence/task-1-fork.txt`
    - Channel: shell(실패 경로). Steps: `cargo run -- inspect --provider omp assets/claude/demo.jsonl`. Expected: `unknown provider "omp"` (T2 전 상태 확인). Evidence: `.issueops/evidence/task-1-fork-error.txt`
  - Commit: YES(2건); Message: `chore(fork): import furkankly/zoetrope@b1f31dd` 와 `chore(identity): rename crate to herdr-agents-graph and drop zoetrope-only assets`; Files: 전체 트리, NOTICE, Cargo.lock

- [ ] 2. omp provider 와 네 형식 판별 (PR #26 적용 + 보완)
  - What to do: PR #26 head `71e8345` 를 `git fetch https://github.com/bl-dev0/zoetrope feat/omp-pi-provider` 후 cherry-pick(충돌 시 수동 적용). 모듈은 `src/provider/omp/`(PR 의 `pi/` 를 이름 변경, pi arm 과 `assets/pi/` 는 제거). `provider_of`: Codex = `session_meta` 또는 `payload`; omp = 1행 `type:"title"` 또는 `type:"session"`+`version`; Claude = top-level `type` + (`sessionId` 또는 `uuid`)(리뷰 반영, PR #26 규칙과 동일); 그 밖은 None. 테스트 `provider_of_pins_all_four_formats` 에 `sessionId` 만 있는 Claude 1행(`agent-setting`, `mode`)을 포함. omp 18.8.4 실파일로 보완: `custom{customType:"tool_execution_start"}` 는 `ToolStart` 보조 신호(toolCall 이 주), `title_change` 는 `Title`, `credential_pin`·`thinking_level_change` 는 무시, `model_change` 는 `Model`, assistant `usage` 는 `Tokens{dedup: responseId}`. 자식 종료: 부모 `toolResult(toolName:"task")` 의 `details.progress[].status` → `Ended(Done|Failed)`; 보조로 `<Name>.jsonl.tombstone` → `Ended(Stopped)`. `related_paths` 는 `<stem>/` 의 `*.jsonl` 만(`.lock.os`, `*.log`, `*.md`, `*.json` 제외). `project_key` 는 PR #26 의 세 분기(home/tmp/legacy)를 `session-paths.ts:44-107` 과 대조.
  - Must NOT do: Claude/Codex 골든 변경, Fact 어휘 변경, pi 지원 유지.
  - Recommended Agent: deep; Reason: 형식 조인 규칙과 종료 판정.
  - Parallelization: YES; Wave 2; Blocks T6, T7, T9, T10; Blocked By T1
  - References: PR #26 본문·파일 목록; `src/provider/mod.rs:135-144`, `:576-579`(기존 provider_of 테스트); `src/provider/claude/mod.rs`(spawn/ended 패턴); omp 소스 `session-entries.ts:236-272`(session_init), `session-manager.ts:373-377`(부모 파일 규칙); 보고서 §3.4·§5.3.
  - Acceptance Criteria: `provider_of_pins_all_four_formats` 와 `provider::tests` 전부 통과; `assets/omp/` fixture 로 `harness::conform("omp", …)` 골든 통과; `cargo run -- inspect --provider omp <이 머신 omp 루트 세션> | grep 'agent(s)'` 가 `agent(s)` ≥ 2, `tool call(s)` ≥ 1; 자동 감지(`inspect <같은 파일>`)가 동일 출력.
  - QA Scenarios:
    - Channel: shell. Steps: 현재 세션 파일(`agents-graph herdr resolve` 로 얻음)에 `inspect --provider omp`. Expected: 서브에이전트 수가 `<Name>.jsonl` 수와 같고 각 subagent 행이 `done`. Evidence: `.issueops/evidence/task-2-omp-inspect.txt`
    - Channel: shell(실패 경로). Steps: 빈 파일과 1행만 있는 파일에 `inspect`. Expected: 종료 코드 ≠0 과 사람이 읽는 오류 한 줄, 패닉 없음. Evidence: `.issueops/evidence/task-2-omp-inspect-error.txt`
  - Commit: YES; Message: `feat(provider): read omp sessions and pin provider detection`; Files: `src/provider/{mod.rs,omp/*}`, `assets/omp/*`, `src/main.rs`

- [ ] 3. 대용량 적재 스트리밍과 rename 교체 추종
  - What to do: (리뷰 반영) 대상은 `src/tailer/replay.rs:141-157 parse_file_into`(`std::fs::read` + `from_utf8_lossy`) 와 `src/main.rs:184` 의 Tail 분기, 그리고 live 첫 폴의 `src/tailer/bytes.rs:80-90` 전체 할당이다. `ReadMode::Whole` sidecar 경로(`live.rs:430`, `replay.rs:115`)는 그대로 둔다. `BufReader::read_until(b'\n')` 로 줄을 읽어 바이트를 세고 `Stream::push` 에 넘기며 원문은 버린다. 잘못된 UTF-8 줄은 건너뛴다. 마지막 개행까지 소비한 바이트를 seed offset 으로 넘긴다(`replay.rs:137-140` 계약). 첫 폴은 1 MiB 청크 반복으로 바꾼다. rename 교체 테스트 `tail_reattaches_after_rename`: 임시 파일에 N 줄 → tail → 같은 경로에 새 파일을 `rename` 으로 교체(새 inode, 더 긴 내용) → 다음 폴에서 `Reset` 이 나오고 새 내용이 처음부터 다시 읽히는지 확인.
  - Must NOT do: replay 의 timestamp 병합 순서 변경, sidecar 파싱 변경.
  - Recommended Agent: deep; Reason: 적재와 tail 의 offset 접합.
  - Parallelization: YES; Wave 2; Blocks T6, T9; Blocked By T1
  - References: `src/tailer/replay.rs:1-30`(assembly 설계), `:137-157`(seed offset·parse_file_into), `src/tailer/bytes.rs:49-143`(TailState·Reset), `:80-90`(첫 폴 할당), `src/main.rs:243-251`(--follow 가 replay 경로), `benches/memory.rs:1-20`.
  - Acceptance Criteria: `/usr/bin/time -l target/release/agents-graph inspect --provider codex <521 MB rollout>` 의 `maximum resident set size` ≤ 104857600; 같은 명령의 agent/tool 수가 변경 전과 동일; `/usr/bin/time -l … inspect --provider claude <96 MB>` 의 real ≤ 0.30; `tail_reattaches_after_rename` 통과; `replay_dates_metas_to_first_subagent_entry`(기존, `replay.rs:166`) 통과.
  - QA Scenarios:
    - Channel: shell. Steps: 위 두 명령. Expected: RSS ≤ 104857600, real ≤ 0.30, 출력 `1 agent(s), 813 tool call(s)`. Evidence: `.issueops/evidence/task-3-streaming.txt`
    - Channel: shell(실패 경로). Steps: 마지막 줄이 잘린 파일(`head -c -50`)로 `--follow` 를 pty 에서 띄운 뒤 잘린 줄의 나머지를 append. Expected: 잘린 줄이 완성되면 한 번만 반영(중복·누락 없음, 디버그 로그로 확인). Evidence: `.issueops/evidence/task-3-streaming-error.txt`
  - Commit: YES; Message: `perf(tailer): stream initial load and reattach after rename`; Files: `src/tailer/{replay.rs,bytes.rs}`, `src/main.rs`

- [ ] 4. TUI 유휴 재그리기 억제와 폴링 backoff
  - What to do: (1·2·3·4차 리뷰 반영) `src/tui.rs`: 성능 영향 (b) 의 규칙대로 "dirty 또는 보이는 프레임이 바뀜" 일 때만 `terminal.draw`. dirty 는 tailer 이벤트·키·마우스·resize·auto-pan 진행(`tick_auto_pan` 반환이 `Event(_)` 이거나 `is_dragging()`)에서 세우고 draw 뒤 내린다. 프레임 변화 술어는 `App` 에 `fn next_frame_due(&self, now, prev_transport) -> bool` 로 두고 glide(매 틱)·ants phase 전환(120 ms mirror, 마지막 fact 뒤 30 s 안에서만)·pending chip `fmt_dur` 문자열 변화(상한 1 s)·afterglow 0.45/0.75/1.0×TTL 경계·transport 변화·status tick 변화·autopilot·Playing 을 묶는다. 마지막 fact 수신 시각은 tailer 이벤트에서 기록한다. `TICK` 은 유지. `src/tailer/live.rs`: 폴 루프(`:256`)를 가변 간격으로 바꾼다. 마지막 변화 뒤 30 s 경과 시 200→500→1000→2000 ms, 변화 읽으면 200 ms 복귀. `SWITCH_IDLE_TICKS` 를 시간 기준 30 s 로 변경. `scripts/idle-cpu.sh <file>`: `script -q /dev/null bash -c 'stty rows 60 cols 200; exec target/release/agents-graph <file> --follow' </dev/null &` 로 띄우고(2·3차 리뷰 반영: pty 크기 0x0 이면 측정이 왜곡되고, stdin 리다이렉트가 없으면 job control 셸에서 시작되지 않음), `pgrep -n -f 'agents-graph .*--follow'` 로 측정 대상 pid 를 잡아 5 s 뒤와 35 s 뒤 `/bin/ps -o time= -p <pid>`(셸 builtin `ps` 가 아닌 `/bin/ps` 명시) 차분을 30 으로 나눠 `idle_cpu_percent=<값>` 출력, 종료 시 프로세스 정리. `scripts/make-running-demo.sh <out>`: `assets/claude/demo.jsonl` 사본 끝에 `date -u -v-40S` 시각의 main assistant 줄 하나를 붙여 main 이 Running 인 파일을 **측정 직전에** 만든다(4차 리뷰 반영: 생성 80 s 뒤 main 이 Idle 로 바뀌므로 고정 파일을 재사용하면 전제가 깨짐).
  - Must NOT do: 변화가 있을 때 지연 증가, 자동 전환 스캔 제거, 입력 응답 지연.
  - Recommended Agent: deep; Reason: 타이밍 상수와 UI 루프 상호작용.
  - Parallelization: YES; Wave 2; Blocks T9; Blocked By T1
  - References: `src/tui.rs:22,57,100`, `src/tailer/live.rs:27-45,244-342`; 리뷰 실측(zoetrope 0.2.0, 완료 세션 30 s 유휴 CPU 약 4 %).
  - Acceptance Criteria: 단위 테스트 `backoff_schedule` 이 (0 s→200, 31 s→500, 32 s→1000, 34 s→2000, 변화 후→200) 고정; 단위 테스트 `draw_skipped_when_clean`(완료 세션, glide 없음, dirty 없음 → draw 0회), `draw_every_tick_while_gliding`(glide 중 0.5 s 동안 매 틱 draw), `draw_every_tick_while_auto_panning`(3차 리뷰 반영), `draw_once_when_live_expires`(`LIVE_FRESH` 경과 시 transport 변화로 1회 draw), `pending_chip_duration_redraws`(main 에 pending 도구 하나, 60 s 무변화, 1 s 동안 draw ≥ 1; 4차 리뷰 반영), `running_without_pending_draws_at_most_once_per_second`(Running main, pending 없음, ants 정지 뒤 1 s 동안 draw ≤ 1) 통과; `scripts/idle-cpu.sh assets/claude/demo.jsonl` ≤ 0.5 와 `scripts/make-running-demo.sh /tmp/demo-running.jsonl && scripts/idle-cpu.sh /tmp/demo-running.jsonl` ≤ 0.5 둘 다, 후자는 측정 뒤 `agents-graph inspect /tmp/demo-running.jsonl | grep -c '\[main\].*(active)'` 가 1.
  - QA Scenarios:
    - Channel: shell(pty). Steps: `scripts/idle-cpu.sh assets/claude/demo.jsonl`; 이어서 `scripts/make-running-demo.sh /tmp/demo-running.jsonl && scripts/idle-cpu.sh /tmp/demo-running.jsonl && agents-graph inspect /tmp/demo-running.jsonl | grep -c '\[main\].*(active)'`. Expected: 두 `idle_cpu_percent=` 값 모두 ≤ 0.5, 마지막 grep 이 `1`. Evidence: `.issueops/evidence/task-4-idle.txt`
    - Channel: shell(변화 경로). Steps: 유휴 40 s 뒤 파일에 한 줄 append, 디버그 로그의 수신 시각과 append 시각 차. Expected: ≤ 2.2 s. Evidence: `.issueops/evidence/task-4-idle-resume.txt`
  - Commit: YES; Message: `perf(ui): redraw only when dirty and back off idle polling`; Files: `src/tui.rs`, `src/state/mod.rs`(next_frame_due), `src/tailer/live.rs`, `scripts/idle-cpu.sh`, `scripts/make-running-demo.sh`

- [ ] 5. Codex `token_usage_record` 읽기(대체 소스)
  - What to do: (1·2·3차 리뷰 반영) 집계의 권위 소스는 종전대로 `token_count` 다. `src/provider/codex/wire.rs` 에 `token_usage_record{thread_id, turn_id, response_id, usage{output_tokens, …}}` 를 역직렬화하는 타입을 추가해 `Other` 로 버리지 않는다. `codex::Stream` 의 파일별 상태에 `saw_token_count` 와 보류 목록(`response_id` 로 dedup 한 `output_tokens`)을 둔다. `push` 는 `token_usage_record` 를 보류만 하고 fact 를 내지 않으며, 첫 `token_count` 가 나오면 보류분을 버린다. 방출 훅은 `provider::Stream::finish(&mut self) -> Option<Statement>` 로 새로 정의한다(Claude 와 omp arm 은 `None`; T2 가 추가하는 omp arm 에도 명시, 4차 리뷰 반영). 방출 Statement 의 `at` 은 마지막 보류 레코드의 timestamp 를 쓴다(`at` 이 None 이면 `item.rs` 의 Timing 규칙으로 날짜가 다른 위치에 붙음). `Target::Here`(poll_live backfill)와 `Bundle::load`(benches) 는 finish 를 부르지 않으므로 `token_usage_record` 만 있는 세션은 그 두 경로에서 tokens 0 으로 남는다. 이 머신의 실파일 621개와 fixture 가 모두 `token_count` 를 가지므로 이 범위를 수용한다. `finish` 는 전체 읽기 경로에서만 호출한다: `main.rs` 의 inspect 전체 파싱, `tailer/replay.rs::parse_file_into` 끝, 하네스 클로저(`codex/mod.rs:561`), `tests/real_sessions.rs`. `poll_live` 에서는 호출하지 않는다. `finish` 가 방출한 합은 Stream 의 `output_tokens` 누적값에 더해서, 뒤에 라이브 tail 로 도착하는 `token_count` 가 초과분만 내게 한다(3차 리뷰 반영: 실파일 54/621 에서 첫 `token_usage_record` 뒤 1 s 이상 지나 첫 `token_count` 가 나오므로 이중 집계 방지가 필요). 이 머신의 cli 0.159.3/0.160.0 파일에는 둘 다 있으므로 총합과 timeline 골든은 변하지 않는다.
  - Must NOT do: Claude 토큰 경로 변경, codex 골든 변경, `token_count` 와 `token_usage_record` 동시 집계.
  - Recommended Agent: quick; Reason: wire 필드 추가와 전환 플래그.
  - Parallelization: YES; Wave 2(T3 뒤, 같은 담당); Blocks T6; Blocked By T1, T3
  - References: `src/provider/codex/wire.rs:327-329`, `src/provider/codex/mod.rs:3-4,31-44,239-265`, `src/provider/harness.rs:33,85`(timeline 골든 비교), `src/state/session.rs:474-480`(dedup None 은 매번 더함), `assets/codex/cli-0.153.4.{model,timeline}.txt`(`tokens: 1860`, 20행 `Tokens@main`); 보고서 §3.3.
  - Acceptance Criteria: `assets/codex/*` 골든 무변경(`git diff --exit-code assets/codex`); 이 머신 521 MB rollout 의 `tokens:` 가 변경 전과 같음(246129); 새 테스트 `token_usage_record_only_counts_without_token_count` 가 (가) 두 레코드가 모두 있는 fixture 에서 `token_count` 값만, (나) `token_usage_record` 만 있는 fixture 에서 `finish` 뒤 그 합을 집계함을 확인; 새 테스트 `finish_then_token_count_does_not_double_count` 가 `finish` 로 방출한 뒤 같은 응답을 포함한 `token_count` 가 와도 합이 늘지 않음을 확인; `inspect --provider codex <token_usage_record 만 있는 fixture>` 가 0 이 아닌 `tokens:` 를 출력.
  - QA Scenarios:
    - Channel: shell. Steps: `inspect --provider codex <rollout> | grep tokens`. Expected: `tokens: 246129`. Evidence: `.issueops/evidence/task-5-codex-tokens.txt`
    - Channel: shell(대체 경로). Steps: 위 테스트 (나) 실행. Expected: `1 passed`. Evidence: `.issueops/evidence/task-5-codex-tokens-fallback.txt`
  - Commit: YES; Message: `feat(provider): read Codex token_usage_record as a fallback token source`; Files: `src/provider/{mod.rs,codex/wire.rs,codex/mod.rs}`, `src/main.rs`, `src/tailer/replay.rs`, `tests/real_sessions.rs`(finish 호출), `assets/codex/*`(fixture 추가만)

- [ ] 6. 실파일 전수 defensive 파싱
  - What to do: `tests/real_sessions.rs`: 환경 변수 `AG_REAL_SESSIONS=1` 일 때만 실행. `~/.claude/projects/*/*.jsonl` 과 `*/subagents/*.jsonl`, `~/.codex/sessions/**/rollout-*.jsonl`, `~/.omp/agent/sessions/*/*.jsonl` 과 `*/<stem>/*.jsonl` 전부를 `open()` + `Stream::push` 로 읽는다. 마지막에 `real_sessions: total=<n> failed=<m>` 한 줄을 찍고 `failed` 가 0 이 아니면 실패한다. 실패 항목은 경로와 오류 종류만 출력.
  - Must NOT do: 파일 내용을 출력에 남기기.
  - Recommended Agent: quick; Reason: 반복 실행 루프.
  - Parallelization: NO; Wave 3; Blocks T9; Blocked By T2, T3, T5
  - References: PR #26 본문의 "567 real files, zero panics" 검증 방식.
  - Acceptance Criteria: `AG_REAL_SESSIONS=1 cargo test --release --test real_sessions -- --nocapture 2>&1 | grep -E '^real_sessions:'` 가 `failed=0`, `total` ≥ 150.
  - QA Scenarios:
    - Channel: shell. Steps: 위 명령. Expected: `real_sessions: total=<n> failed=0`. Evidence: `.issueops/evidence/task-6-real-sessions.txt`
    - Channel: shell(실패 경로). Steps: `AG_REAL_SESSIONS_ROOT=<임시 디렉터리>` 에 깨진 JSON 파일 하나를 두고 실행. Expected: `failed=1` 과 경로. Evidence: `.issueops/evidence/task-6-real-sessions-error.txt`
  - Commit: YES; Message: `test(provider): parse every local session file without panicking`; Files: `tests/real_sessions.rs`

- [ ] 7. herdr 브리지: `herdr resolve|toggle` 서브커맨드, kind=path, 패널 id 토글, jq 제거
  - What to do: (리뷰 반영: 셸에는 JSON 처리를 남기지 않는다) `src/main.rs` 에 `herdr` 서브커맨드 둘. `herdr resolve`: `HERDR_PLUGIN_CONTEXT_JSON` 에서 `focused_pane_id` 를 읽고 `HERDR_BIN_PATH pane get <id>` 를 실행해 `.result.pane.agent_session` 을 해석, `<provider> <value>` 출력. `claude|codex` 는 `kind==id`, `omp` 는 `kind==path`(파일 존재 확인), 그 밖은 사람이 읽는 오류(`is not one this plugin reads`, 또는 kind 없음 안내와 `herdr integration install <agent>` 힌트)와 종료 코드 2. `herdr toggle <placement>`: `HERDR_PLUGIN_STATE_DIR/open-pane` 을 읽어 id 가 있고 `pane get` 이 성공하면 `plugin pane close <id>` 후 파일 삭제; 실패하면 파일 삭제; 없으면 `plugin pane open --plugin $HERDR_PLUGIN_ID --entrypoint graph --placement <p>` 를 실행하고 응답 JSON 의 pane id 를 파일에 쓴다. `herdr-plugin/herdr/pane.sh` 는 `exec "$BIN" herdr toggle "${1:-overlay}"`, `open.sh` 는 `target=$("$BIN" herdr resolve) && exec "$BIN" --provider ${target%% *} --follow "${target#* }"` 와 실패 메시지 표시·enter 대기. `$BIN` 은 `$HERDR_PLUGIN_ROOT/bin/agents-graph`, 없으면 안내 후 종료(PATH 폴백 없음, 리뷰 반영). `herdr-plugin.toml`: `id = "m16khb.herdr-agents-graph"`, `name = "agents-graph"`, `min_herdr_version = "0.9.3"`, `[[panes]] graph`(overlay), 액션 `open`/`open-split`/`open-tab`, `[[build]] command = ["bash","herdr/install.sh"]`.
  - Must NOT do: `jq` 호출, 라벨 비교, `HERDR_PANE_ID` 로 대상 판정, popup 액션, PATH 의 `zoe` 사용.
  - Recommended Agent: deep; Reason: CLI·셸 경계와 실패 경로.
  - Parallelization: YES; Wave 2; Blocks T10; Blocked By T1, T2
  - References: 업스트림 `herdr-plugin/herdr/resolve.sh:5-9,30-35`, `pane.sh:25-40`; herdr 0.9.3 `src/api/schema/plugins.rs` PluginInvocationContext(`focused_pane_id`), `src/agent_resume.rs:126-132`(kind 규칙); `herdr plugin pane open --help`(placement 값); 보고서 §2.2·§2.3.
  - Acceptance Criteria: 실제 컨텍스트(`HERDR_PLUGIN_CONTEXT_JSON='{"focused_pane_id":"<omp pane>"}' HERDR_BIN_PATH=herdr agents-graph herdr resolve`)가 `omp <path>` 출력; `focused_pane_id` 가 에이전트 없는 패널이면 종료 코드 2 와 안내; `grep -c jq herdr-plugin/herdr/*.sh` 합계 0; `bash -n herdr-plugin/herdr/*.sh` 통과; 단위 테스트 `resolve_accepts_only_known_kinds` 통과.
  - QA Scenarios:
    - Channel: shell. Steps: 위 resolve 명령. Expected: `omp /…/.jsonl`. Evidence: `.issueops/evidence/task-7-resolve.txt`
    - Channel: shell(토글 실패 경로). Steps: `open-pane` 에 존재하지 않는 id 를 쓰고 `herdr toggle overlay`(이 세션의 herdr, 자기 패널만). Expected: 파일이 지워지고 새 패널이 열린 뒤 그 id 가 기록됨; 다시 실행하면 닫히고 파일 삭제. Evidence: `.issueops/evidence/task-7-toggle.txt`
  - Commit: YES; Message: `feat(herdr): resolve path sessions and toggle by pane id without jq`; Files: `src/main.rs`, `src/herdr.rs`, `herdr-plugin/**`

- [ ] 8. 릴리스 workflow 와 체크섬 검증 설치 스크립트
  - What to do: (리뷰 반영) `.github/workflows/cd.yml` 을 태그 push(`v*`) 트리거로 바꾸고 GPG 서명(103-108행)·homebrew job(136-145행)·`GH_TOKEN` 을 제거해 `GITHUB_TOKEN` 으로 업로드한다. matrix 5 target 의 빌드·cross 단계는 유지. 산출물 `agents-graph-<target>.tar.gz`(windows 는 zip). 모든 빌드 job 뒤 `needs` 로 묶인 `checksums` job 이 자산을 모아 `SHA256SUMS` 를 만들어 같은 릴리스에 올린다. `herdr-plugin/herdr/install.sh`: `HERDR_PLUGIN_*` 없이 실행된다는 전제로 설치 위치를 `$(cd "$(dirname "$0")/.." && pwd)/bin/agents-graph` 로 고정. `uname -s/-m` 으로 target 선택, 매니페스트 `version` 과 같은 태그의 `AG_RELEASE_BASE`(기본 GitHub releases URL)에서 아카이브와 `SHA256SUMS` 를 `curl -fsSL` 로 받아 `shasum -a 256 -c`(macOS)/`sha256sum -c`(Linux) 검증 후 설치. 같은 버전이 이미 있으면 건너뛴다. 실패 시 임시 디렉터리만 남긴다. `herdr-plugin/bin/` 을 `.gitignore` 에 등록.
  - Must NOT do: brew/cargo 호출, PATH 설치, 체크섬 없이 실행, `HERDR_PLUGIN_STATE_DIR` 의존.
  - Recommended Agent: quick; Reason: 스크립트와 CI 설정.
  - Parallelization: YES; Wave 2; Blocks T10; Blocked By T1
  - References: 업스트림 `.github/workflows/cd.yml:6-8,24-80,103-108,130,136-145`; herdr v0.9.3 `src/cli/plugin.rs:1353,1532-1550`(빌드 env scrub); plugins.mdx 222행(link 는 build 미실행), 228-229행; reviewr `herdr/install.sh`(체크아웃 상대 설치 방식).
  - Acceptance Criteria: `bash -n` 과 `shellcheck`(있으면) 통과; 로컬 `cargo build --release` 산출물로 가짜 릴리스 디렉터리(`file://`)를 만들어 `AG_RELEASE_BASE=file:///tmp/fake-release bash herdr-plugin/herdr/install.sh` 가 `installed agents-graph <version>` 출력하고 `herdr-plugin/bin/agents-graph --version` 성공; `SHA256SUMS` 변조 시 종료 코드 1, `checksum mismatch`, 바이너리 미생성; `grep -n 'GPG_RELEASE_KEY\|TAP_TOKEN\|release-plz' .github/workflows/cd.yml` 0건.
  - QA Scenarios:
    - Channel: shell. Steps: 위 설치 명령. Expected: 위 출력. Evidence: `.issueops/evidence/task-8-install.txt`
    - Channel: shell(실패 경로). Steps: 변조 후 같은 명령. Expected: `checksum mismatch`, rc=1. Evidence: `.issueops/evidence/task-8-install-error.txt`
  - Commit: YES; Message: `build(release): tag-triggered checksummed binaries and download-only installer`; Files: `.github/workflows/cd.yml`, `herdr-plugin/herdr/install.sh`, `.gitignore`

- [ ] 9. 벤치 비교표·README·NOTICE
  - What to do: README 를 이 플러그인 기준으로 쓴다(설치, 키 설정, 지원 에이전트, 측정표, zoetrope 와의 관계). 측정표는 기준선(zoetrope 0.2.0, 이 머신)과 변경 후를 같은 파일로 비교: 96 MB Claude(wall, RSS), 521 MB Codex(wall, RSS), 유휴 CPU(`scripts/idle-cpu.sh`), omp 자동 감지 결과. `NOTICE` 에 출처·라이선스·PR #26 저자. `lib.rs:13` 의 README include 가 doc-test 를 만드므로 README 의 코드 블록은 `text` 로 표시한다.
  - Must NOT do: 측정하지 않은 수치 기재, 세션 내용·개인 경로 기재.
  - Recommended Agent: quick; Reason: 문서.
  - Parallelization: YES; Wave 3; Blocks —; Blocked By T3, T4, T6
  - References: 보고서 §4.6 기준선 표.
  - Acceptance Criteria: README 표의 모든 수치에 대응하는 evidence 파일이 있음; `grep -c "/Users/" README.md` 가 0(2차 리뷰 반영: 액션 id 와 설치 원본에 "m16khb" 가 정당하게 들어가므로 개인 경로만 검사); `cargo test --doc` 통과.
  - QA Scenarios:
    - Channel: shell. Steps: 표의 명령을 재실행해 README 값과 ±10 % 비교. Expected: 범위 안. Evidence: `.issueops/evidence/task-9-bench.txt`
    - Channel: shell(실패 경로). Steps: 개인 경로 grep. Expected: 0. Evidence: `.issueops/evidence/task-9-bench-privacy.txt`
  - Commit: YES; Message: `docs: describe the plugin, install path, and measured baseline`; Files: `README.md`, `NOTICE`

- [ ] 10. 실제 herdr 설치·패널 QA (세 에이전트, install --ref)
  - What to do: (1·2차 리뷰 반영) 전제: 저장소가 공개 상태다(사용자 승인으로 2026-10-08 공개 전환, `gh repo view --json isPrivate` 가 false; herdr 는 `plugin.rs:783-785,891-892` 에서 익명 https 로 clone 한다). (a) 브랜치를 push 한 뒤 가짜 릴리스 디렉터리로 실제 설치 경로를 탄다: `AG_RELEASE_BASE=file:///tmp/fake-release herdr plugin install m16khb-org/herdr-agents-graph/herdr-plugin --ref 1-agents-graph-mvp --yes && herdr plugin list`(빌드는 CLI 프로세스에서 사용자 환경을 상속하고 `HERDR_PLUGIN_*`·herdr 관리 변수만 지워지므로 `AG_RELEASE_BASE` 는 전달된다, `plugin.rs:231→1328-1353,1532-1550`). (b) CLI `plugin action invoke` 는 `focused_pane_id` 를 비워 보내고 서버가 현재 포커스로 채우므로(`plugin.rs:477-487`, `context.rs:18`) 대상 패널을 `herdr pane focus <id>` 로 먼저 포커스하고 끝나면 원래 포커스로 되돌린다. 이 세션의 omp 패널을 대상으로 `herdr plugin action invoke m16khb.herdr-agents-graph.open` 두 번: 첫 번째에 label `agents-graph` 패널이 생기고 두 번째에 사라지는지 `herdr pane list` 로 확인, `herdr pane read` 로 그래프 텍스트에 `main` 과 subagent 이름이 있는지 확인. (c) Claude Code·Codex: `herdr pane split` 으로 자기 패널을 만들고 그 안에서 `claude`/`codex` 를 짧은 작업으로 실행해 `agent_session.kind == id` 가 보고되면 같은 invoke 를 반복(자기 패널만, 완료 후 닫음). (d) `agent_session` 이 없는 omp 패널(연동 설치 전 시작)을 대상으로 invoke 해 안내 문구를 확인. (e) `herdr plugin uninstall m16khb.herdr-agents-graph`.
  - Must NOT do: 다른 사람의 패널에 키·텍스트 전송, herdr 서버 재시작, 사용자 config 수정(`setup-keys` 액션 미실행), 플러그인을 설치한 채 두기.
  - Recommended Agent: deep; Reason: 실 환경 상호작용.
  - Parallelization: YES; Wave 3; Blocks —; Blocked By T7, T8
  - References: `herdr plugin install --help`(`--ref`), `plugin action invoke`, `plugin pane open/close`, `pane split/read/close`(보고서 §2.5); herdr-cwd 매니페스트.
  - Acceptance Criteria: `herdr plugin list` 에 `m16khb.herdr-agents-graph` 가 보임; omp·Claude·Codex 세 패널 각각에서 invoke 2회로 패널 생성·삭제 관측; `herdr plugin log` 에 오류 없음; uninstall 뒤 목록에서 사라짐.
  - QA Scenarios:
    - Channel: shell + herdr. Steps: 위 (a)~(c). Expected: 세 에이전트 모두 생성·삭제, `pane read` 에 `main` 포함. Evidence: `.issueops/evidence/task-10-herdr-qa.txt`
    - Channel: shell(실패 경로). Steps: (d). Expected: 패널 안에 `Herdr has no session path for this omp pane` 와 `herdr integration install omp` 힌트. Evidence: `.issueops/evidence/task-10-herdr-qa-error.txt`
  - Commit: NO(evidence 만)

## 게이트

| 게이트 | 결과 | CHECK | EXPECT |
|---|---|---|---|
| G1 | 전체 테스트 통과 | `cargo test --locked > /tmp/g1.log 2>&1; echo rc=$?; grep -E '^test result' /tmp/g1.log \| grep -vc ' 0 failed'` | `rc=0` 과 `0`(2차 리뷰 반영: rc 동시 확인) |
| G2 | omp 자동 감지·조인 | `target/release/agents-graph inspect <omp 루트 세션> \| grep 'agent(s)'` | `agent(s)` 값 ≥ 2 |
| G3 | 네 형식 판별 테스트 | `cargo test --locked provider_of_pins_all_four_formats 2>&1 \| grep 'test result'` | `1 passed` |
| G4 | RSS 기준 | `/usr/bin/time -l target/release/agents-graph inspect --provider codex <521MB> 2>&1 \| grep 'maximum resident'` | 값 ≤ 104857600 |
| G5 | 유휴 CPU | `scripts/idle-cpu.sh assets/claude/demo.jsonl; scripts/make-running-demo.sh /tmp/demo-running.jsonl && scripts/idle-cpu.sh /tmp/demo-running.jsonl; echo main_active=$(target/release/agents-graph inspect /tmp/demo-running.jsonl \| grep -c '\[main\].*(active)')`(pty 200x60, `/bin/ps`, 측정 pid 명시; 사본은 측정 직전 생성) | 두 `idle_cpu_percent=` 값 ≤ 0.5 와 `main_active=1`(4차 리뷰 반영) |
| G6 | 실파일 전수 | `AG_REAL_SESSIONS=1 cargo test --release --test real_sessions -- --nocapture 2>&1 \| grep -E '^real_sessions:'` | `failed=0` |
| G7 | jq 의존 0 | `grep -c jq herdr-plugin/herdr/*.sh \| awk -F: '{s+=$2} END {print s}'` | `0` |
| G8 | 설치 스크립트 체크섬 | `rm -rf herdr-plugin/bin; AG_RELEASE_BASE=file:///tmp/fake-release-bad bash herdr-plugin/herdr/install.sh; echo rc=$?; test -e herdr-plugin/bin/agents-graph; echo exists=$?` | `checksum mismatch`, `rc=1`, `exists=1`(2·3차 리뷰 반영) |
| G9 | 실 설치·토글 | `herdr plugin list \| grep -c m16khb.herdr-agents-graph` 그리고 invoke 2회 전후 `herdr pane list` | `1`; label `agents-graph` 패널 존재 후 부재 |
| G10 | rename 교체 추종 | `cargo test --locked tail_reattaches_after_rename 2>&1 \| grep 'test result'` | `1 passed` |
| G11 | 첫 결과 시간 | `/usr/bin/time -l target/release/agents-graph inspect --provider claude <96MB> 2>&1 \| grep real` | real ≤ 0.30 |
| G12 | Codex 토큰 무회귀 | `git diff --exit-code assets/codex/*.txt; target/release/agents-graph inspect --provider codex <521MB> \| grep tokens` | 기존 골든 diff 없음, `tokens: 246129` |

## Final Verification Wave

- F1 Plan Compliance Audit: T1~T10 이 명시대로 실행됐는지 evidence 파일로 대조.
- F2 Code Quality Review: AI slop·죽은 코드·pi arm 잔재·zoetrope 이름 잔재 확인, `cargo clippy --all-targets -- -D warnings`.
- F3 Real Manual QA: G2·G4·G5·G6·G9·G11 재실행, README 표와 대조.
- F4 Scope Fidelity Check: Must NOT Have 위반 없음, 비목표 기능 미포함.

## 후속 (이 사이클 밖, 별도 승인)

- 이 머신의 `furkankly.zoetrope` herdr 플러그인 제거(`herdr plugin uninstall furkankly.zoetrope`)와 brew `zoetrope` 제거. MVP 가 G9 를 통과한 뒤 대상과 fingerprint 를 preview 하고 사용자 승인을 받아 실행한다.
- `pane.focused`/`pane.updated` 구독 자동 전환, omp 분기 트리, 노드별 비용 표시.
- 업스트림 PR #26 에 이 작업에서 확인한 omp 18.8.4 차이를 리뷰 코멘트로 남긴다.

## Commit Strategy

TODO 마다 하나씩 Conventional Commit(Lore 본문). 포크 가져오기 커밋은 업스트림 히스토리를 포함하므로 PR 은 다중 커밋이며 그 사유를 PR 본문에 적는다.

## Success Criteria

이슈 #1 의 완료 기준 체크리스트 전부(대응표 참조)와 G1~G12 EVIDENCE.
