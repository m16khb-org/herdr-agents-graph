# 요청자 의도 계약

- lifecycle: io-4b513b64dc69
- issue: https://github.com/m16khb-org/herdr-agents-graph/issues/1
- intent_class: standard

## 원문 요청
첨부한 docs/research/task.md 는 이 저장소(herdr-agents-graph)의 첫 태스크입니다. 문서의 조사 질문, 지킬 것, 산출물, 완료 기준을 그대로 따라 자료조사를 하고 docs/research/report.md 를 커밋해 주세요(push 하지 않음). 구현 코드는 쓰지 않습니다.
[중간 지시] 경쟁 플러그인들을 조사하고 내가 만든 플러그인의 품질과 최적화가 최고여야해
[후속 지시] 다 정리 완료되면 자료조사된 내용을 기반으로 이슈를 만들고 로 진행
(task.md 목표: herdr 에서 포커스된 에이전트 패널의 세션을 실시간 흐름 그래프(메인 에이전트, 서브에이전트, 도구 호출)로 보여 주는 herdr 플러그인 herdr-agents-graph 를 만든다. 지원 에이전트는 Claude Code, Codex, omp 세 가지다.)

## 해석
docs/research/report.md 의 권장안대로 herdr-agents-graph MVP 를 구현한다. (1) furkankly/zoetrope@b1f31dd(MIT) 소스를 이 저장소에 가져오고 업스트림 PR #26 의 omp provider·provider_of 오분류 수정을 적용해 Claude Code·Codex·omp 세 에이전트를 한 바이너리로 읽는다. (2) herdr 브리지(herdr-plugin.toml, resolve, toggle)가 agent_session.kind=path(omp) 와 kind=id(claude/codex) 를 모두 받고, 같은 키 토글을 패널 id 기록으로 처리하며 jq 없이 동작한다. (3) 경쟁 도구 실측 기준선(zoetrope: 파일 전체 적재로 RSS≈파일 크기, 200ms 고정 폴링, omp 오분류)을 넘기 위해 초기 적재 스트리밍, 유휴 폴링 backoff, omp rename 재작성 감지, token_usage_record/token_count 집계를 넣는다. (4) 릴리스 바이너리를 체크섬 검증으로 내려받는 [[build]] 설치 스크립트로 런타임 의존 0 설치를 제공한다. 비목표: pane.focused 구독 자동 전환, omp 분기 트리 표시, 노드별 비용 표시, 업스트림 머지.

## 성공 기준
- - 이 머신의 omp 패널(herdr pane get 의 agent_session.kind=path)에서 플러그인 키를 누르면 메인 에이전트·task 서브에이전트·도구 호출이 그래프로 보이고 같은 키로 닫힌다.
- zoe inspect --provider omp <실제 omp 세션> 이 서브에이전트 파일 <ts>_<uuid>/<Name>.jsonl 을 부모 task 호출에 조인해 agent 수와 tool call 수를 출력하고, provider 자동 감지가 omp 파일을 claude 로 오분류하지 않는다(fixture 테스트 고정).
- Claude Code·Codex 기존 골든 테스트와 cargo test --locked 가 모두 통과한다.
- 이 머신의 Claude Code·Codex·omp 실제 세션 파일 전수를 inspect 로 파싱해 패닉 0, 비정상 종료 0.
- 521 MB Codex rollout 을 inspect 할 때 최대 RSS 가 100 MB 이하(기준선 561 MB), 96 MB Claude 세션 첫 결과까지 0.3 s 이내(기준선 0.19 s 와 동등 수준).
- 유휴 상태(파일 변화 없음 30 s)에서 폴링 간격이 늘어나 프로세스 CPU 가 0.5% 이하.
- omp /new 로 세션 파일이 rename 교체된 뒤에도 그래프가 새 파일을 따라간다(inode 변화 테스트).
- herdr plugin install <owner>/herdr-agents-graph/herdr-plugin 이 jq·cargo·brew 없이 성공하고 herdr plugin list 에 나타난다.

## 비목표
- (없음)

## 제약
- (없음)

## 모호함
- (없음)

## 읽는 규칙
이 문서는 요청자 의도 계약이다. 원격 이슈 본문은 구현 계약이다. 두 문서가 충돌하면 구현을 시작하지 말고 충돌한 줄을 인용해 blocker로 보고한다.
