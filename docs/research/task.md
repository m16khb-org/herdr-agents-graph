# 자료조사 태스크: herdr-agents-graph

## 목표

herdr 에서 포커스된 에이전트 패널의 세션을 실시간 흐름 그래프(메인 에이전트, 서브에이전트, 도구 호출)로 보여 주는
herdr 플러그인 `herdr-agents-graph` 를 만든다. 지원 에이전트는 **Claude Code, Codex, omp** 세 가지다.
기존 플러그인 zoetrope 는 Claude Code·Codex 만 읽어서 omp 패널에서는
`agent 'omp' in pane … is not one zoe reads` 로 끝난다. 이 빈자리를 채우는 것이 출발점이다.

이번 태스크는 **구현 전 자료조사**다. 코드는 쓰지 않는다. 결과는 아래 "산출물" 의 보고서 하나다.

## 환경 (조사 시점에 다시 확인할 것)

- macOS arm64. herdr 0.9.3, omp 18.8.4, zoe 0.2.0(zoetrope CLI) 설치됨.
- herdr 내장 연동 상태: `herdr integration status` → claude v10, codex v8, omp v10 이 설치돼 있다.
- `herdr pane list` 의 `agent_session`: omp 패널은 `{source: "herdr:omp", agent: "omp", kind: "path"}` 를 보고한다.
  omp 연동을 설치하기 전에 시작한 세션은 `agent_session` 이 없다.
  Claude Code·Codex 는 `kind: "id"` 로 보고한다(zoetrope `herdr-plugin/herdr/resolve.sh` 주석 기준, 직접 확인할 것).
- 세션 파일 위치 후보: `~/.claude/projects/<프로젝트>/<id>.jsonl`, `~/.codex/sessions/`, `~/.omp/agent/sessions/`.
- 설치된 플러그인 소스: `~/.config/herdr/plugins/github/` (zoetrope, reviewr, herdr-file-viewer, herdr-cwd, web-ui, plugin-manager).
- omp 의 herdr 연동 확장: `~/.omp/agent/extensions/herdr-omp-agent-state.ts` (`pane.report_agent_session` 호출부).

## 조사 질문

### 1. herdr 플러그인 표면 (0.9.3)

- `herdr-plugin.toml` 매니페스트의 전체 스키마: `platforms`, `[[actions]]`, `[[panes]]`(entrypoint), `[[events]]`, `[[startup]]`, `[[build]]`, 링크 핸들러.
- 플러그인 패널 배치(popup, overlay, split, tab, zoomed)와 같은 키로 열고 닫는 토글을 만드는 방법(zoetrope 의 `open`/`open-split`/`open-tab` 구현 참고).
- 액션이 받는 컨텍스트(포커스된 패널 id, cwd, agent, worktree 등)와 환경 변수(`HERDR_BIN_PATH`, `HERDR_PLUGIN_ROOT` 등).
- 소켓 API·CLI 로 패널의 `agent`, `agent_session` 을 얻는 방법과, 포커스가 바뀔 때 받을 수 있는 이벤트.
- 근거: `herdr --help`, `herdr plugin --help`, `herdr --skill`, https://herdr.dev/llms.txt, herdr 저장소 문서(herdrdev/herdr), 설치된 플러그인 매니페스트.

### 2. 에이전트별 세션 데이터

에이전트마다 아래를 표로 정리한다.

- 세션 ID·경로를 herdr 에서 얻는 방법과, 그 값으로 세션 파일을 찾는 규칙.
- 파일 형식(JSONL 등)과 레코드 종류: 사용자 메시지, 어시스턴트 메시지, 도구 호출과 결과, 서브에이전트 시작·종료, 모델, 토큰.
- 서브에이전트 기록 방식: Claude Code 의 sidechain·subagents 파일, Codex 의 하위 세션, omp 의 task·subagent 세션과 실시간 에이전트 레지스트리.
- 실시간으로 따라갈 수 있는가: 덧붙이기 쓰기인지, 파일을 교체하는지, 버퍼링 지연이 있는지.
- 버전에 따라 형식이 바뀐 흔적과 버전 표시 필드.
- 근거: 각 에이전트의 소스·문서(omp: can1357/oh-my-pi), zoetrope 의 `src/provider/claude`, `src/provider/codex`.

### 3. 이미 있는 것 (재사용 또는 참고)

- **furkankly/zoetrope**: 아키텍처(provider → fact → state → ui), herdr 연동 스크립트, 라이선스.
  zoe 에 omp provider 를 더하는 쪽이 나은지, 새로 만드는 쪽이 나은지 판단 근거를 낸다(업스트림 기여 가능성 포함).
- **hanbong5938/herdr-omp-subagents**: omp 브리지 확장이 실시간 에이전트 레지스트리를 읽는 방식.
- **edxeth/herdr-pi-tree**, **aemrebarut/herdr-dagr**: 트리·DAG 표시 방식.
- GitHub `topic:herdr-plugin` 에서 세션 시각화·omp 관련 플러그인을 더 찾는다.

### 4. 구현 선택지

- 언어와 TUI 라이브러리: Rust + ratatui(zoetrope, file-viewer), Bun/Node, 그 밖. 배포는 릴리스 바이너리, 소스 빌드, 설치 스크립트 중 무엇으로 할지.
- 실시간 갱신 방식(파일 감시, 폴링, herdr 이벤트)과 대용량 세션 성능.
- 세 에이전트를 하나의 내부 모델(노드: 에이전트·서브에이전트·도구 호출, 엣지: 호출·생성)로 맞추는 방법.

## 지킬 것

- 코드를 쓰지 않는다. 구현 계획도 보고서의 "제안" 절까지만 쓴다.
- 이 머신의 실제 세션 파일은 구조(필드 이름, 레코드 종류) 확인에만 쓴다. 대화 내용, 경로의 개인 정보, 토큰을 보고서에 옮기지 않는다.
  예시가 필요하면 필드 값을 지어낸 값으로 바꾼다.
- herdr 설정·플러그인·다른 저장소를 바꾸지 않는다. `herdr server stop` 이나 서버 재시작을 하지 않는다.
- 다른 패널에 키나 글자를 보내지 않는다.
- 커밋은 하되 push 하지 않는다.
- 확인한 것과 추정을 구분한다. 추정에는 `[추정]` 을 붙인다. 모든 주장에 근거(파일 경로:줄, URL, 실행한 명령)를 단다.

## 산출물

`docs/research/report.md` (한국어). 아래 순서로 쓴다.

1. 요약: 결론과 권장 방향
2. herdr 플러그인 표면 정리
3. 에이전트별 세션 데이터 비교표와 세부
4. 기존 플러그인 분석과 재사용 판단
5. 구현 선택지 비교와 제안(언어, 배포, 내부 모델, 갱신 방식, MVP 범위)
6. 위험과 아직 모르는 것
7. 출처 목록

## 완료 기준

- 위 조사 질문에 모두 답했거나, 답하지 못한 이유와 다음에 확인할 방법을 적었다.
- 세 에이전트 각각에 대해 "herdr 패널 → 세션 파일 → 서브에이전트·도구 호출" 로 이어지는 경로를 실제 파일로 한 번씩 확인했다.
- 보고서를 커밋했다(push 하지 않음).
