# 요청자 의도 계약

- lifecycle: io-4b513b64dc69
- issue: https://github.com/m16khb-org/herdr-agents-graph/issues/6
- intent_class: standard

## 원문 요청
근데 ui가 너무 zoetrope를 배꼈어 더 직관적이고 독창적인 ui/ux로 개선이 가능할까?
→ (제안: '지금'이 먼저 보이는 화면 — 상단 요약, 에이전트 트리+의도, 시간 레인, 주의 우선, 좁은 패널 우선, 키맵 단순화)
당근 디자인시스템을 확인하고 가져와서 세팅하고 그걸 기반으로 ui 층을 새로 설계해줘
[결정] 강조색: SEED 구조 + 강조색만 다른 팔레트(blue 또는 purple) / 진행 범위: 지난번처럼 전체 사이클

## 해석
당근 SEED Design(daangn/seed-design, Apache-2.0) 의 rootage 토큰(@seed-design/rootage-artifacts 3.0.2, git 22b68ce)을 이 저장소에 고정 버전으로 가져와 Rust 토큰 모듈로 생성하고(LICENSE·NOTICE 귀속 유지, 로고·상호 등 브랜드 리소스 미사용), 그 토큰과 SEED 컴포넌트 의미(badge, chip, callout, list-item, tablist, progress, snackbar, divider)를 TUI 위젯으로 옮긴 디자인 시스템 층을 만든다. 그 위에 zoetrope 에서 가져온 UI 층(src/ui/*, 키맵)을 새로 설계한다: 기본 화면은 '지금' 뷰(상단 요약 · 에이전트 트리 + 도구 호출의 의도 · 주의 항목 우선), 보조로 시간 레인 뷰와 기존 그래프 뷰(토큰으로 재스타일), 좁은 패널 우선 반응형, 단순하고 화면에 항상 보이는 키맵. brand 역할은 SEED purple 팔레트로 매핑한다(blue 는 informative 톤이 쓰므로). 라이트/다크 테마를 터미널 배경으로 감지한다. provider·fact·state·tailer 층과 성능 최적화(스트리밍 적재, 유휴 재그리기 억제, backoff)는 유지하고, 의도(intent)와 비용(cost)을 fact 로 추가한다. 전체 사이클: 이슈→계획 리뷰→구현→PR→CI→머지→v0.2.0 릴리스→실설치 QA→정리.

## 성공 기준
- - SEED rootage 토큰이 고정 버전(rootage-artifacts 3.0.2)으로 저장소에 들어 있고, LICENSE·NOTICE 귀속이 포함되며, 생성된 Rust 토큰이 vendored YAML 과 일치함을 테스트가 확인한다(UPDATE 시 재생성).
- src/ui 에 하드코딩된 Color::/Style 색 리터럴이 0개이고 모든 색이 SEED 시맨틱 토큰(fg/bg/stroke)에서 온다.
- 라이트·다크 두 테마에서 기본 '지금' 뷰, 시간 레인 뷰, 그래프 뷰가 80x24 와 120x40 에서 렌더되고 버퍼 스냅샷 테스트로 고정된다.
- '지금' 뷰가 상단 한 줄 요약(상태·경과·토큰·omp 는 비용), 에이전트 트리(상태 배지·의도·현재 도구와 경과 시간), 실패/막힘 callout 을 보여 준다. omp 도구 호출 intent, Claude Agent description 이 표시된다.
- 상태는 색만으로 구분하지 않고 글리프+텍스트를 함께 쓴다.
- 키맵: Tab/1·2·3 뷰 전환, j/k·화살표 선택, Enter 펼치기, Esc 뒤로, ? 도움말, q 종료; 하단 힌트 바가 항상 보인다.
- 기존 성능 기준 유지: 521MB Codex RSS ≤100MB, 끝난 세션 유휴 CPU ≤0.5%, 세 에이전트 실파일 전수 파싱 패닉 0, inspect 골든 무변경.
- CI 전 job 통과 후 머지, v0.2.0 릴리스, herdr plugin install 로 실설치해 omp 패널에서 새 UI 로 열고 닫힘을 확인한다.

## 비목표
- (없음)

## 제약
- (없음)

## 모호함
- (없음)

## 읽는 규칙
이 문서는 요청자 의도 계약이다. 원격 이슈 본문은 구현 계약이다. 두 문서가 충돌하면 구현을 시작하지 말고 충돌한 줄을 인용해 blocker로 보고한다.
