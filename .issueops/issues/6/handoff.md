# 인계 자료 (io-4b513b64dc69, 이슈 #6)

- 작성 시각: 2026-10-09T01:01:26.802211+00:00
- 목적: 이슈 #6(https://github.com/m16khb-org/herdr-agents-graph/issues/6). SEED Design rootage 토큰(3.0.2, daangn/seed-design@22b68ce)을 vendoring·생성하고, 테마·위젯 층 위에 UI 층을 새로 설계·구현한다('지금' 기본 뷰, 시간 레인, 그래프, 상세, 새 키맵, RedrawGate 술어). fact 에 도구 intent 와 cost 를 더한다.
- 비목표: 당근 로고·상호·캐릭터, carrot 강조색, SEED 전체 컴포넌트, 웹 UI, herdr pane.focused 자동 전환, provider 파싱 규칙·inspect 출력·골든 변경.
- 승인된 종료점(구현 세션): draft PR 발행 + execution complete. 머지·CI 대응·v0.2.0 릴리스·실설치 QA·정리는 준비 세션(pane wK:p1)이 이어서 한다.
- source root: /Users/m16khb/Workspace/herdr-agents-graph
- canonical worktree: /Users/m16khb/Workspace/herdr-agents-graph.worktrees/6-seed-ui-redesign (branch 6-seed-ui-redesign)
- base head = full head: 1c744d24931a6634f8dea129c218f1823775f1c0 (diff 없음)
- 계획: /Users/m16khb/Workspace/herdr-agents-graph.worktrees/6-seed-ui-redesign/.issueops/issues/6/artifact/plan.md (sha256 deebc0757beb8c5a4d4332ab313075a7010fbe39c8d9ea950b3e70ff86844482). 적대 리뷰 3라운드(revise×2 → pass), 판정은 이 digest 에 묶임.
- 완료한 조사: SEED 저장소 구조(rootage YAML·컴포넌트 104·라이선스·NOTICE 브랜드 조항), crate 선택(terminal-colorsaurus 1.0.3 런타임, yaml-rust2 0.13 dev), 실제 omp usage.cost 는 객체(cost.total). 코드 지도: 계획 '재사용하는 기존 구현' 표.
- 3차 리뷰 비차단 메모(구현 시 반영):
  1. 스냅샷에서 스피너 위상·상단 바 경과 시간은 now_reference(또는 App 주입 시계)에서 계산하고, 스냅샷 준비 단계에서 tick_timeline(ZERO) 로 상태를 정착시킨다. src/ui 에 Utc::now/Instant::now/Local::now 를 직접 쓰지 않는다.
  2. T9 무응답 터미널 측정은 TERM=xterm-256color, env -u COLORFGBG -u AG_THEME 로 실행한다(이 환경의 기본 TERM=dumb 는 질의를 건너뜀).
  3. Cargo include 검증: cargo package --list --locked | grep -c '^design/seed/' 가 2.
  4. 입력 비우기 단계(ratatui::init 뒤, EventStream 생성 전)를 T8 구현에도 반영. handler 의 ']' 는 수정키를 보지 않으므로 늦은 OSC 응답(ESC ])이 seek 로 읽힐 수 있음에 유의.
  5. handler.rs:368 테스트는 Tab→j 교체 외에 그래프 뷰 설정과 노드 타입 교체가 필요.
  6. herdr 패널의 OSC 11 응답 여부·지연을 T2 에서 실측해 README 에 기록.
- 주의(caution): .issueops/cautions 의 두 건(AGENTS_GRAPH_PANE 전달 유지, Codex hook 신뢰는 사용자 결정).
- 현재 lifecycle 상태: stage implement.enter 전(plan.handoff 완료), execution direct generation 1, 이 자료 작성 뒤 holder 가 release 한다.
- 새 세션 첫 단계: issueops next --id io-4b513b64dc69 --json → released 이면 replace preview 부터 next_command 체인으로 자기 actor 로 claim → link-plan → compatibility review → gates init(G1~G12, 비차단 메모 포함) → implement.
- 읽기 전용 확인: issueops status --id io-4b513b64dc69 --json, issueops execution status --id io-4b513b64dc69 --json, git -C /Users/m16khb/Workspace/herdr-agents-graph.worktrees/6-seed-ui-redesign rev-parse HEAD, shasum -a 256 /Users/m16khb/Workspace/herdr-agents-graph.worktrees/6-seed-ui-redesign/.issueops/issues/6/artifact/plan.md
