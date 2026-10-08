# herdr-agents-graph 자료조사 보고서

조사일: 2026-10-08. 환경: macOS arm64, herdr 0.9.3 (API protocol 22), omp 18.8.4, zoe 0.2.0, Claude Code 2.1.291~2.1.292, Codex CLI 0.159.3~0.160.0.
표기: `[추정]` 은 직접 확인하지 못한 추론이다. 경로의 사용자 홈은 `~` 로, 세션 식별자와 프로젝트 슬러그는 `<id>`, `<slug>` 로 바꿔 적었다. 세션 파일은 필드 이름과 레코드 종류만 읽었고 내용은 옮기지 않았다.

## 1. 요약: 결론과 권장 방향

- **세 에이전트 모두 "herdr 패널 → 세션 파일 → 서브에이전트·도구 호출" 경로가 파일 기반으로 닫힌다.** 셋 다 JSONL 덧붙이기 쓰기이고, 서브에이전트는 별도 파일(Claude Code: `<sessionId>/subagents/agent-<agentId>.jsonl`, omp: `<ts>_<uuid>/<AgentName>.jsonl`) 또는 같은 디렉터리 트리의 형제 파일(Codex: `rollout-*-<childThreadId>.jsonl`)로 남는다. 실시간 레지스트리나 소켓 없이 파일 꼬리 읽기만으로 그래프를 만들 수 있다.
- **herdr 가 패널에 붙여 주는 세션 참조는 에이전트마다 kind 가 다르다.** Claude Code·Codex 는 `kind: "id"` (네이티브 세션/스레드 id), omp 는 `kind: "path"` (전사 파일 절대 경로). herdr 0.9.3 소스 `src/agent_resume.rs:126-132` 가 이 규칙을 고정한다. zoetrope 의 `resolve.sh` 는 `kind == id` 와 `claude|codex` 만 받아서 omp 가 막힌다.
- **권장: 새로 만들지 말고 zoetrope 를 포크해 omp provider 와 브리지의 `kind: path` 처리를 더한 뒤, 그 결과물을 `herdr-agents-graph` 플러그인으로 배포한다.** 근거는 네 가지다. (a) zoetrope 는 MIT 이고 provider → fact → state → ui 경계가 이미 있어 omp 는 provider 한 디렉터리로 끝난다. (b) 같은 내용의 업스트림 PR #26(bl-dev0, 2026-09-13) 이 이미 열려 있고 omp·pi 공유 모듈, 서브에이전트 조인, `resolve.sh` 의 path 허용까지 포함한다. (c) 업스트림 메인테이너는 2026-09-11 이후 커밋이 없고, 외부 PR #25·#26·#29·#31 과 본인 릴리스 PR #22 가 모두 리뷰 없이 열려 있어 업스트림 머지만 기다리는 선택은 일정이 없다. (d) 처음부터 만들면 zoetrope 의 provider·fact·state 약 4,500줄과 rataflow 레이아웃을 다시 쓰게 된다.
- MVP 범위: 포크한 zoe 에 omp provider 적용 → `herdr-agents-graph` 저장소에 브리지(manifest, `pane.sh`, `resolve.sh`, 설치 스크립트) 와 릴리스 바이너리 배포 → 포커스된 패널이 omp 든 Claude Code 든 Codex 든 같은 키로 그래프를 열고 닫는다. 업스트림에는 PR #26 리뷰 코멘트 또는 분리된 작은 PR 로 기여를 시도하되, 머지를 전제로 하지 않는다.
- **품질·최적화 목표는 4.6 절의 측정 기준으로 고정한다.** 경쟁 도구 중 herdr 안에서 세션 그래프를 그리는 것은 zoetrope 뿐이고, 실측 결과 파일 전체를 메모리에 올리며(521 MB rollout → RSS 561 MB), omp 파일을 Claude 로 오분류하고, 200 ms 고정 폴링으로 유휴에도 CPU 를 쓴다. herdr-agents-graph 는 스트리밍 파서, 유휴 backoff, 포커스·세션 전환 자동 추종, 런타임 의존 0 설치, 세 에이전트 실파일 전수 defensive 파싱을 기준으로 삼아 이 빈자리를 메운다.

## 2. herdr 플러그인 표면 정리 (0.9.3)

### 2.1 `herdr-plugin.toml` 스키마

근거: herdr 0.9.3 문서 `docs/next/website/src/content/docs/plugins.mdx` (raw.githubusercontent.com, 2026-10-08), 설치된 매니페스트 6개, `herdr plugin --help`, `herdr plugin pane open --help`.

| 키 | 필수 | 내용 | 근거 |
|---|---|---|---|
| `id`, `name`, `version`, `min_herdr_version` | 필수 | 플러그인 id 는 영문·숫자·`.`·`:`·`_`·`-`; 액션 정식 id 는 `<plugin.id>.<action.id>` | plugins.mdx; zoetrope `herdr-plugin/herdr-plugin.toml:1-23` |
| `description`, `platforms` | 선택 | `platforms` 는 최상위 또는 항목별(항목별이 우선). 값 예: `["macos","linux"]` | zoetrope 매니페스트 17행; herdr-file-viewer 매니페스트 |
| `[[build]]` `command`, `platforms` | 선택 | `plugin install` 때만 실행, 실패하면 설치 중단. `plugin link` 는 실행하지 않음. 소켓 env 없음 | plugins.mdx; zoetrope 매니페스트 25-28행 |
| `[[startup]]` `command` | 선택 | 서버 시작 뒤 활성 플러그인마다 한 번. env `HERDR_PLUGIN_EVENT=startup` | plugins.mdx; herdr-cwd 매니페스트 |
| `[[actions]]` `id`, `title`, `description`, `contexts`, `command`, `platforms` | 선택 | `contexts` 예: `["pane","workspace"]` | zoetrope 매니페스트 39-76행 |
| `[[events]]` `on`, `command` | 선택 | `on` 은 link 시점에 검사, 모르는 이름은 경고만. 실제 훅 대상은 `pane.created/updated/moved/closed/focused/output_changed/exited/agent_detected/agent_status_changed`, `layout.updated` 등 | herdr 소스 `src/app/api/plugins/context.rs:130-180`; herdr-cwd 매니페스트 12-60행 |
| `[[panes]]` `id`, `title`, `placement`, `command`, `width`, `height`, `platforms` | 선택 | `placement` 기본 `overlay`. 패널 프로세스는 `HERDR_PLUGIN_ENTRYPOINT_ID` 를 받음 | plugins.mdx; zoetrope 매니페스트 33-37행 |
| `[[link_handlers]]` `id`, `title`, `pattern`, `action` | 선택 | Ctrl+클릭 URL 을 액션으로 라우팅 | plugins.mdx |
| `[[keys.command]]` `key`, `type="plugin_action"`, `command`, `description` | 사용자 config | herdr 는 플러그인 매니페스트에서 키를 바인딩하지 않는다. zoetrope 는 `setup-keys` 액션으로 사용자 config 에 블록을 써 넣는다 | plugins.mdx; zoetrope 매니페스트 62-76행 |

`command` 는 argv 배열이고 셸을 거치지 않는다. 실행 cwd 는 플러그인 디렉터리다.

### 2.2 패널 배치와 같은 키 토글

- 배치 값: 소켓 스키마 `PluginPanePlacement = overlay | popup | split | tab | zoomed` (`herdr api schema --json`). CLI `herdr plugin pane open --placement` 의 가능 값은 `overlay, split, tab, zoomed` 로 **popup 이 빠져 있다** (`herdr plugin pane open --help` 실행 결과). popup 을 쓰려면 소켓 요청이나 매니페스트 `[[panes]] placement = "popup"` 을 써야 한다 [추정: CLI 가 의도적으로 뺀 것인지 누락인지는 확인하지 못함].
- `overlay` 는 임시 zoomed 패널이며 닫으면 이전 포커스와 zoom 을 복원한다. `popup` 은 세션 모달로 pane id 가 없고 `HERDR_PANE_ID` 도 받지 않으며 pane 생명주기 이벤트를 내지 않는다. `split/tab/zoomed/overlay` 는 연 뒤 일반 패널이라 `pane.move/swap/resize/zoom` 을 쓸 수 있다 (plugins.mdx "Panes" 절).
- 같은 키 토글 레시피 (zoetrope `herdr-plugin/herdr/pane.sh:25-40`): `HERDR_PLUGIN_CONTEXT_JSON.focused_pane_id` 로 `herdr pane get` 을 호출해 `.result.pane.label` 이 매니페스트 `title` 과 같고 `.agent` 가 비어 있으면 `herdr plugin pane close <id>`, 아니면 `herdr plugin pane open --plugin <id> --entrypoint graph --placement <p>`. reviewr 는 같은 목적을 `--action toggle` 인자 하나로 처리한다 (reviewr `herdr-plugin.toml` `[[actions]] toggle`). 라벨 비교 방식이라 사용자가 패널 이름을 바꾸면 토글이 깨진다 [추정].

### 2.3 액션이 받는 컨텍스트와 환경 변수

근거: plugins.mdx 255-275행, herdr 소스 `src/api/schema/plugins.rs:355-395` (`PluginInvocationContext`), `src/app/api/plugins/runtime.rs:59-62`.

- 공통 env: `HERDR_SOCKET_PATH`, `HERDR_BIN_PATH`, `HERDR_ENV=1`, `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ROOT`, `HERDR_PLUGIN_CONFIG_DIR`, `HERDR_PLUGIN_STATE_DIR`, `HERDR_PLUGIN_CONTEXT_JSON`; 있을 때 `HERDR_WORKSPACE_ID`, `HERDR_TAB_ID`, `HERDR_PANE_ID`.
- 액션: `HERDR_PLUGIN_ACTION_ID`. 이벤트 훅: `HERDR_PLUGIN_EVENT`, `HERDR_PLUGIN_EVENT_JSON`. 패널: `HERDR_PLUGIN_ENTRYPOINT_ID`. 링크 핸들러: `HERDR_PLUGIN_CLICKED_URL`, `HERDR_PLUGIN_LINK_HANDLER_ID`.
- `HERDR_PLUGIN_CONTEXT_JSON` 필드(소스 기준, 모두 선택): `workspace_id`, `workspace_label`, `workspace_cwd`, `worktree`, `tab_id`, `tab_label`, `focused_pane_id`, `focused_pane_cwd`, `focused_pane_agent`, `focused_pane_status`, `selected_text`, `invocation_source`, `correlation_id`, `clicked_url`, `link_handler_id`. **`agent_session` 은 컨텍스트에 없다.** 패널 명령 안에서 `HERDR_PANE_ID` 는 플러그인 자신의 새 패널이므로, 대상 패널은 반드시 `focused_pane_id` 로 잡아야 한다 (zoetrope `resolve.sh:5-9` 주석, 실제로 그렇게 동작).
- herdr 관리 변수는 `--env` 로 덮어쓸 수 없다 (cli-reference.mdx env 절).

### 2.4 패널의 `agent`·`agent_session` 얻기와 이벤트

- CLI: `herdr pane get <pane_id>`, `herdr pane list`, `herdr agent get/list`. 응답은 `{"id":..., "result":{"type":"pane_info","pane":{...}}}` 이고 레코드는 `.result.pane` 아래에 있다 (직접 실행, 2026-10-08). `PaneInfo` 필드: `agent`, `agent_session`, `agent_status`, `cwd`, `display_agent`, `focused`, `foreground_cwd`, `label`, `pane_id`, `restore_error`, `revision`, `scroll`, `state_labels`, `tab_id`, `terminal_id`, `terminal_title`, `terminal_title_stripped`, `title`, `tokens`, `workspace_id` (`herdr api schema --json`).
- `agent_session` 스키마: `{source, agent, kind, value}`, `kind` 는 `AgentSessionRefKind = "id" | "path"` (schema `$defs/AgentSessionRefKind`). 실측: omp 패널은 `{"source":"herdr:omp","agent":"omp","kind":"path","value":"~/.omp/agent/sessions/<slug>/<ts>_<uuid>.jsonl"}` (`herdr pane list`, `herdr pane get w5:p1`). omp 연동 설치 전에 시작한 패널은 `agent: "omp"` 이지만 `agent_session: null` 이다(실측 2개 패널).
- kind 결정 규칙 (herdr 소스 `src/agent_resume.rs:116-133` `session_ref_from_report`): `source` 가 `herdr:<agent>` 공식 소스일 때만 저장한다(`is_official_agent_source`, 328행). `agent` 가 `pi`·`omp` 면 `agent_session_path` 를 우선 쓰고 없으면 id, 그 밖의 에이전트는 **`agent_session_id` 만 쓰고 `agent_session_path` 는 버린다.** 따라서 Claude Code 훅이 `transcript_path` 를 보내도(`~/.claude/hooks/herdr-agent-state.sh:63-77`) 패널에는 `kind: "id"` 만 남는다. 커스텀 소스(`custom:*`)는 `agent_session` 을 저장하지 못한다.
- 보고 측 RPC: `pane.report_agent_session {pane_id, source, agent, agent_session_id?, agent_session_path?, seq?, session_start_source?, resume_argv?}` (schema `PaneReportAgentSessionParams`).
- 이벤트: 소켓 `events.subscribe {"subscriptions":[{"type":"pane.focused"}, {"type":"pane.agent_status_changed","pane_id":...}]}`. 구독 가능 타입 27개 중 그래프 플러그인에 필요한 것은 `pane.focused` (payload `{pane_id, workspace_id}`), `pane.updated` (payload `pane: PaneInfo`, **`agent_session` 포함**), `pane.agent_status_changed` (`agent`, `agent_status`, `display_agent`, `state_labels`, `title`), `pane.agent_detected`, `pane.closed`. `events.wait` 로 단발 대기도 된다 (schema; herdr 소스 `src/api/server.rs:1566`). 구독 연결은 열린 채 유지되고 일반 RPC 는 응답마다 서버가 연결을 닫으므로 연결을 분리해야 한다 (devswha.herdr-web-ui `AGENTS.md:20` [scout 보고, 직접 재확인하지 않음]).
- `[[events]]` 훅은 위 소켓 이벤트 중 `src/app/api/plugins/context.rs:130-180` 에 매핑된 것만 호출하며, 이벤트 훅으로 "포커스가 바뀌면 그래프를 다른 세션으로 바꾸기"는 가능하다. 다만 이미 열린 패널 프로세스에 전달하려면 플러그인이 자체 IPC(파일·소켓)를 둬야 한다 [추정: herdr 에 플러그인 패널로 메시지를 보내는 API 는 없음, plugins.mdx 에 storage/messaging API 없음 명시].

### 2.5 배포

- `herdr plugin install owner/repo[/subdir] [--ref ..] [--yes]`: git clone 후 `[[build]]` 실행. `herdr plugin list` 는 `github:<owner>/<repo>[/<subdir>]@<commit>` 로 고정 커밋을 보여 준다(실측: zoetrope 는 `github:furkankly/zoetrope/herdr-plugin@b1f31dd…`). `plugin update` 는 없고 재설치로 갱신한다 (cli-reference.mdx).
- `herdr plugin link <path>`: 로컬 개발용, build 없음.
- 마켓플레이스: GitHub topic `herdr-plugin`, 30분 주기 인덱싱, 검수 없음 (marketplace.mdx).
- 바이너리 조달 패턴: zoetrope 는 `[[build]]` 에서 `ensure-zoe.sh` 가 `brew install furkankly/tap/zoetrope` 또는 `cargo install zoetrope` 를 실행하고(`herdr/ensure-zoe.sh:67-74`), 패널 스크립트는 PATH 의 `zoe` 를 호출한다(`herdr/open.sh`). herdr-file-viewer·web-ui 는 플랫폼별 `[[build]]` 로 소스 빌드/다운로드를 한다 (매니페스트 scout 확인).

## 3. 에이전트별 세션 데이터 비교표와 세부

### 3.1 비교표

| 항목 | Claude Code | Codex CLI | omp |
|---|---|---|---|
| herdr 가 받는 값 | `kind: "id"`, `value` = 세션 UUID. SessionStart 훅 `~/.claude/hooks/herdr-agent-state.sh:53-84` 가 `agent_session_id`(+`agent_session_path`, herdr 가 버림) 보고. matcher `^(startup\|resume\|clear\|compact\|fork)$` (`~/.claude/settings.json`) | `kind: "id"`, `value` = 스레드 UUID. `~/.codex/herdr-agent-state.sh:51-82`, SessionStart 에서만. `CODEX_THREAD_ID` 가 있고 다르면 보고 안 함(63행) | `kind: "path"`, `value` = 루트 전사 절대 경로. 확장 `~/.omp/agent/extensions/herdr-omp-agent-state.ts:101` 이 `ctx.sessionManager.getSessionFile()` 을 읽어 `agent_session_path` 로 보고(117행). `ctx.hasUI === true` 일 때만(331행) → 서브에이전트는 보고하지 않음. `session_start`·`session_switch`·`agent_start` 마다 재보고(375-402행) |
| 세션 파일 위치 | `~/.claude/projects/<slug>/<sessionId>.jsonl`; `<slug>` = cwd 의 `[^A-Za-z0-9]` 를 전부 `-` 로 1:1 치환 (zoetrope `src/provider/claude/discovery.rs:13-18`; 실제 파일로 검증) | `~/.codex/sessions/YYYY/MM/DD/rollout-<ts>-<threadId>.jsonl`; 파일명 끝 36자 = `session_meta.payload.id` (zoetrope `src/provider/codex/discovery.rs` 주석; 실제 파일로 검증). `CODEX_HOME` 으로 루트 변경 | 경로가 그대로 옴. 디렉터리 규칙은 참고용: 홈 안이면 `-` + 홈 상대경로의 `/`→`-`, `$TMPDIR` 안이면 `-tmp-…`, 그 밖은 `--<abs>--` (`@oh-my-pi/pi-coding-agent/src/session/session-paths.ts:44-107`) |
| 형식 | JSONL. 공통 봉투: `type, uuid, parentUuid, sessionId, version, cwd, gitBranch, timestamp, isSidechain, agentId?, requestId?` | JSONL. 봉투 `{timestamp, ordinal, type, payload}` | JSONL. 1행 `type:"title"` 고정폭 256바이트 슬롯(`v`, `title`, `updatedAt`, `pad`), 2행 `type:"session"` 헤더(`version`, `id`, `timestamp`, `cwd`, `parentSession?`), 이후 레코드는 `{type, id, parentId, timestamp, ...}` 트리 |
| 사용자/어시스턴트 | `type:"user"`, `type:"assistant"`; `message.content[]` 블록 `text`/`thinking`/`tool_use`/`tool_result`; assistant 한 응답이 블록마다 여러 줄로 나뉘고 `requestId` 가 같음. `message.model`, `message.usage{input_tokens, output_tokens, cache_*}` | `response_item.payload.type`: `message`(role), `reasoning`, `function_call`/`function_call_output`(`call_id`), `custom_tool_call`/`custom_tool_call_output`(`exec`), `tool_search_call`; `event_msg.payload.type`: `task_started`, `task_complete`, `token_count`, `item_completed`, `turn_aborted`, `thread_settings_applied`; `turn_context`; `compacted`; `token_usage_record` | `type:"message"`, `message.role` ∈ `user`/`assistant`/`toolResult`; assistant `content[]` 블록 `text`/`thinking`/`toolCall{id,name,arguments,intent}`; `toolResult` 는 `toolCallId`, `toolName`, `isError`, `details`; assistant 에 `model`, `provider`, `usage`, `stopReason`, `ttft`, `duration` |
| 도구 호출 ↔ 결과 | `tool_use.id` ↔ `tool_result.tool_use_id` (user 레코드), `toolUseResult` 객체 | `function_call.call_id` ↔ `function_call_output.call_id`; `item_completed.item.type` 로 `CommandExecution`, `McpToolCall`, `FileChange`, `Extension(kind)`, `ImageView` 등 요약 | `toolCall.id` ↔ `toolResult.toolCallId`; 실행 시작은 `custom{customType:"tool_execution_start", data:{toolCallId, toolName, startedAt, args, intent}}` |
| 서브에이전트 | 메인 `tool_use name:"Agent"`(zoetrope 는 `Task`·`Workflow` 도 spawn 으로 봄) → user `tool_result` 의 `toolUseResult{agentId, status:"async_launched", isAsync, outputFile, resolvedModel}` → 파일 `<sessionId>/subagents/agent-<agentId>.jsonl` (레코드 `isSidechain:true`, `agentId`, 같은 `sessionId`) + `agent-<agentId>.meta.json{agentType, description, toolUseId, spawnDepth, requestShape, requestNonInteractive, model}`. 실측 39쌍 | 부모 `function_call name:"spawn_agent"`(namespace `collaboration`) → `item_completed{item:{type:"SubAgentActivity", kind:"started", id:<call_id>, agent_thread_id, agent_path}}`; `kind:"interacted"`(send_message), `kind:"completed"`. 자식 파일 1행 `session_meta.payload{parent_thread_id, session_id(=루트 id), thread_source:"subagent", source:{subagent:{thread_spawn:{parent_thread_id, depth, agent_path, agent_nickname, agent_role}}}}`. 실측 10월 파일 57개 중 자식 16개 | 부모 `toolCall name:"task"`, `arguments.tasks[].name` → 자식 파일 `<ts>_<uuid>/<Name>.jsonl` (세션 파일 옆 같은 이름 디렉터리). 자식 1행 `title`, 2행 `session{parentSession:<부모 절대경로>}`, 그 뒤 `session_init{task, tools, agent, modelRole, resolvedModel, ...}` (`session-entries.ts:236-272`). 부모 `toolResult.details.progress[]{id, agent, status, toolCount, tokens, cost, durationMs}`, `details.async{state, jobId}`. 부모 찾기: `dirname(child) + ".jsonl"` (`session-manager.ts:373-377`). 실행 중 `.<Name>.jsonl.lock.os` 가 생기고, 강제 종료 시 `<Name>.jsonl.tombstone` (`registry/agent-tombstone.ts:4-6`). 완료 후 `<Name>.md`, `<Name>.json` 결과 파일이 옆에 생김(실측) |
| 실시간 추적 | 덧붙이기. zoetrope 는 200ms 폴링, 바이트 오프셋, 개행 기준 부분행 버퍼, `(dev, ino)` 변화·축소 시 리셋 (`src/tailer/live.rs:27-38`, `src/tailer/bytes.rs:49-143`). 파일 알림 API 미사용 | 덧붙이기. 동일 tailer | 덧붙이기 `O_WRONLY\|O_CREAT\|O_APPEND`, fsync 없음 (`session-storage.ts:23-24, 257`). 단 제목 변경·세션 전환·분기·newSession 때 **전체 재작성 + rename** (`session-manager.ts:1677 #rewriteAtomically`, 호출부 2251, 2301, 2412, 2614, 2633). 1행 title 슬롯은 고정폭이라 제자리 덮어쓰기 [추정: `SESSION_TITLE_SLOT_BYTES=256` 에서 유추]. inode 교체를 반드시 감지해야 함 |
| 버전 표시 | 레코드마다 `version` (실측 `2.1.291`, `2.1.292` 가 한 세션에 섞임) | `session_meta.payload.cli_version` (실측 `0.159.3`, `0.160.0`). 0.101 → 0.120 사이에 `source` 가 문자열에서 객체로, `parent_thread_id` 위치가 바뀜 (scout 실측 + zoetrope `wire.rs` 주석 0.149.1/0.150.0-alpha.8/0.153.4) | `session.version` (현재 `CURRENT_SESSION_VERSION = 3`, `session-entries.ts:15`); v1 은 필드 없음 |

### 3.2 Claude Code 세부

- 실측 레코드 `type` (최근 3개 파일 합산): `attachment`, `assistant`, `user`, `mode`, `atis-latch`, `last-prompt`, `permission-mode`, `ai-title`, `frame-link`, `pr-link`, `system`, `queue-operation`, `file-history-snapshot`, `artifact-autoreact-ledger`, `file-history-delta`, `artifact-comment-monitor`. zoetrope `src/provider/claude/wire.rs` 의 `Entry` 디스패치에 없는 타입(`atis-latch`, `frame-link`, `file-history-delta` 등)은 unknown 으로 버려지므로 파서는 **모르는 타입을 무시하는 전진 호환**이 필수다.
- `system` 레코드에 `subtype`, `toolUseID`, `pendingBackgroundAgentCount`, `compactMetadata` 가 있어 컴팩션과 백그라운드 에이전트 수를 보조 신호로 쓸 수 있다 [추정: 값 의미는 확인하지 않음].
- 세션 디렉터리 `<sessionId>/` 에는 `subagents/` 와 `tool-results/` 가 있고 `tasks/` 는 없었다. `toolUseResult.outputFile` 이 가리키는 `tasks/<agentId>.output` 은 디스크에 없었다(scout 확인).
- 서브에이전트 종료는 별도 레코드가 없고, zoetrope 는 메인 파일의 task-notification 문자열 user 레코드로 처리한다 (`src/provider/claude/mod.rs`).
- 토큰은 `requestId` 로 중복 제거해야 한다(같은 응답이 블록마다 여러 줄).

### 3.3 Codex 세부

- 실측 빈도(2026-10 파일 57개): `event_msg/item_completed` 3,463, `token_count` 1,536, `token_usage_record` 1,506, `custom_tool_call`(+output) 1,233, `reasoning` 1,043, `message` 879, `function_call`(+output) 236, `turn_context` 152, `task_started` 149, `task_complete` 135, `inter_agent_communication_metadata` 99, `agent_message` 99, `world_state` 90, `session_meta` 62, `SubAgentActivity` started 16 / interacted 59 / completed 25, `compacted` 5, `ContextCompaction` 3.
- `token_usage_record` 와 `inter_agent_communication_metadata`, `world_state` 는 zoetrope `wire.rs` 가 `Other` 로 버린다. 최신 버전 토큰 집계는 `token_count` 이벤트로도 가능하다 [추정].
- `~/.codex/config.toml` 에는 `notify`, `plugin_hooks`, `multi_agent`, `hooks` 와 `hooks.state` 의 `subagent_start/subagent_stop` 항목이 있다(키만 확인). 파일 외 보조 신호로 쓸 수 있지만 사용자 설정 의존이라 MVP 에서는 제외한다.
- `~/.codex/archived_sessions` 는 없었다. `state_5.sqlite` 등 sqlite 인덱스는 이름만 확인하고 스키마는 보지 않았다.

### 3.4 omp 세부

- 실측 레코드 `type` (현재 세션): `message`, `custom`(`tool_execution_start`), `title_change`, `title`, `session`, `model_change`, `thinking_level_change`, `credential_pin`; 자식 파일에는 `session_init` 이 추가된다. 소스가 정의하는 그 밖의 타입(`compaction`, `branch_summary`, `label`, `mode_change`, `tool_result_update` 등)은 `session-entries.ts` 에 있으나 이번 파일에는 없었다.
- 모든 레코드가 `id`/`parentId` 로 트리를 이루므로 분기(branch) 가 있는 세션은 `parentId` 체인을 따라야 현재 가지가 나온다. zoetrope PR #26 설명도 "the DAG is explicit in the format" 이라고 적었다.
- 실시간 레지스트리: omp 의 `AgentRegistry` (`src/registry/agent-registry.ts:115 static global()`, `335 isRunning`, `348 onChange`) 는 **프로세스 내 메모리**이고 디스크 파일·소켓으로 노출되지 않는다. 디스크에는 `~/.omp/run/daemons/<hash>/{broker.token, clients/, scope.json}`, `~/.omp/run/session-owners/<id>.lock`, `~/.omp/agent/terminal-sessions/<tty>` (cwd, 세션 파일 경로, `cwdstat` 세 줄 텍스트) 가 있다. 외부 프로세스가 레지스트리를 읽으려면 hanbong5938 방식(omp 확장으로 `AgentRegistry.global()` 을 폴링해 herdr 로 보고)이 필요하다. 그래프 플러그인은 전사 파일만으로 충분하다 [추정: 서브에이전트 "실행 중" 판정은 자식 파일 mtime/lock 파일/부모 toolResult 도착으로 대체].
- 재작성 위험: `/new`, `/resume`, 분기, 제목 변경 때 파일이 rename 으로 교체되며, `session_switch` 가 나면 herdr 패널의 `agent_session.value` 도 새 경로로 바뀐다(실측: 같은 패널 `w5:p1` 이 조사 중 다른 경로를 보고함). 플러그인은 `pane.updated` 이벤트를 구독해 경로 변경을 따라가야 한다.

### 3.5 완료 기준 확인: 패널 → 파일 → 서브에이전트·도구 호출

| 에이전트 | 패널 단계 | 파일 단계 | 서브에이전트·도구 호출 단계 |
|---|---|---|---|
| omp | `herdr pane get wK:p1` → `kind: path` (실측) | 그 경로가 존재, 옆에 같은 이름 디렉터리 존재 (실측) | 부모 `toolCall name:"task"` 1건, 자식 `<Name>.jsonl` 5개, 자식 2행 `parentSession` = 부모 경로 (실측) |
| Claude Code | 라이브 패널 없음. 훅 스크립트 53-84행으로 `agent_session_id` 보고를 확인, herdr 소스 132행으로 `kind: id` 확정 | 파일명 stem == `sessionId`, 디렉터리 slug == cwd 치환 (실측 True/True) | `tool_use name:"Agent"` 12건, `subagents/agent-*.jsonl` 39개 + meta.json 39개, 자식 1행 `isSidechain:true` (실측) |
| Codex | 라이브 패널 없음. 훅 스크립트 51-82행으로 `agent_session_id` 보고 확인, herdr 소스 132행으로 `kind: id` | 파일명 꼬리 == `session_meta.id`, `id == session_id`, `thread_source:"user"` (실측 True) | `spawn_agent` + `SubAgentActivity started/completed`, 자식 `parent_thread_id == session_id`, `source.subagent.thread_spawn{...}` (실측, cli 0.160.0) |

Claude Code·Codex 의 패널 단계는 이번 머신에 열린 패널이 없어 실제 `pane get` 출력으로 보지 못했다. 다음 확인 방법: herdr 패널에서 `claude` 또는 `codex` 를 새로 시작한 뒤 `herdr pane get <id>` 를 실행한다.

## 4. 기존 플러그인 분석과 재사용 판단

### 4.1 furkankly/zoetrope

- Rust, MIT, ★1,015, 포크 69, 마지막 커밋 2026-09-11 (`b1f31dd`), 릴리스 0.2.0 (2026-09-10). 설치된 브리지는 그 커밋 고정.
- 의존: `ratatui 0.30`, `rataflow 0.1`(sugiyama 레이아웃), `crossterm`, `tokio`, `imbl`(영속 자료구조), `serde_json`, `chrono`. WASM 빌드로 웹 앱도 같은 엔진을 쓴다.
- 파이프라인: `herdr-plugin/herdr/pane.sh` → `open.sh` → `resolve.sh` → `zoe --provider <agent> --follow <id>`. 코어는 `src/provider/<name>/{wire.rs, discovery.rs, mod.rs}` 가 파일을 찾고 줄을 `Statement{at, facts}` 로 바꾸고, `src/state/*` 가 `SessionModel` 을 만들고, `src/ui/*` 가 그래프를 그린다. 전체 provider+fact 약 4,550줄(`wc -l`).
- provider 는 trait 이 아니라 **enum** 이다(`src/provider/mod.rs` 헤더 주석: "providers arrive by pull request, and an exhaustive match makes the compiler list every site"). 프리미티브: `all_paths(scope)`, `session_file(path)`, `related_paths(file)`, `project_key(cwd)`, `session_file_from(path, head)`, `stream_for(file)`, `sidecar(file, text)`; `Stream::push(&str) -> Option<Statement>`.
- 공통 어휘 `FactKind` (`src/fact.rs:53-108`): `Agent, Activity, Label, Model, Tokens{output, dedup}, Prompt, Reasoning, ToolStart, ToolEnd{call, outcome}, Spawn{call}, Ended(status), Session, Tally, Title`; `AgentKind {Main, Subagent, Group}`; `AgentStatus {Running, Idle, Done, Failed, Stopped}`. 이것이 곧 "세 에이전트를 맞출 내부 모델" 이다.
- omp 거부 지점: `resolve.sh:37-41` (`claude | codex` 만), `:43-57` (`kind == id` 만). `docs/HERDR-PLUGIN.md` 도 "pi and omp store a path, and zoetrope does not read those agents" 라고 적는다.
- 업스트림 상태(api.github.com, 2026-10-08): 열린 PR `#22 chore: release v0.2.1`(메인테이너 본인, 09-10), `#25 feat(provider): read pi sessions`(09-13), `#26 provider: read omp and pi sessions, subagent graph included`(09-13, 코멘트 0, 리뷰는 Copilot 봇뿐), `#29 IBM Bob`(09-17), `#31 Skill call 요약`(10-01), dependabot 3건. 이슈 `#15 support hermes`. 즉 9월 11일 이후 사람 리뷰·머지가 없다.
- PR #26 내용(본문 기준, diff 전체는 검토하지 않음): `provider_of` 가 `type` 이 있는 아무 줄이나 Claude 로 오분류하던 버그 수정, `src/provider/pi/` 한 디렉터리로 omp·pi 두 arm, omp 자식 `<Name>.jsonl` 을 `task` 호출의 `tasks[].name` 과 `session_init` 으로 조인, `resolve.sh` 에 `kind: path` 허용, 실파일 567개 파싱, `cargo test` 226 통과(자가 보고). 픽스처 `assets/omp/<ts>_<uuid>/{Helper.jsonl, Reviewer.jsonl, Reviewer.md, 3.bash.log, __sidecar.jsonl}` 가 이 머신의 실제 레이아웃과 일치한다.

### 4.2 hanbong5938/herdr-omp-subagents

- TypeScript omp 확장, LICENSE 파일 없음(API `license: null`), ★3, 마지막 push 2026-09-30.
- 데이터: omp 프로세스 안에서 `exports.AgentRegistry.global()` 을 능력 탐지로 얻어 `list/get/isRunning/onChange` 를 쓰고(`extension.ts:101-118`), `REFRESH_MS` 주기 타이머로 폴링(192행)한 뒤 `herdr pane report-metadata` 로 사이드바에 "역할:모델" 행을 올린다. 그래프가 아니고 전사도 읽지 않는다.
- 참고 가치: omp 내부 레지스트리에 접근하는 유일한 공개 예시. 라이선스가 없어 코드 재사용은 불가하고 방식만 참고한다.

### 4.3 edxeth/herdr-pi-tree, aemrebarut/herdr-dagr

- herdr-pi-tree: JavaScript, MIT, pi 전용 사이드바 트리(spawn 관계, worktree, git stat). omp 미지원, 그래프 아님.
- herdr-dagr: Rust, 라이선스 표기 불일치(API Apache-2.0, README "Apache-2.0 OR MIT"), ★93. producer 가 쓰는 `run.json`(CONTRACT v3) 을 DAG 로 렌더하며 세션 전사를 읽지 않는다. trace 문법과 인스펙터 UI 는 시각 참고.

### 4.4 topic:herdr-plugin 의 다른 항목

zenbu-labs/terminal-browser, persiyanov/herdr-reviewr, devswha/herdr-web-ui, smarzban/herdr-file-viewer, alexarthurs/herdr-sidebar, AltanS/collie(모바일 클라이언트) 등. 세션 시각화 플러그인은 zoetrope 뿐이다. topic 전체 목록은 첫 페이지만 확인했다.

### 4.5 재사용 판단

| 선택 | 장점 | 단점·위험 |
|---|---|---|
| A. zoetrope 포크 + omp provider(PR #26 기반) + 자체 브리지 배포 | provider 경계와 UI 가 이미 있음. PR #26 이 레이아웃·조인 규칙을 검증함. MIT. 세 에이전트가 같은 `Fact` 모델로 수렴 | 포크 유지 비용. 업스트림과 갈라질 때 rebase 부담. PR #26 diff 를 직접 검토해야 함 |
| B. 업스트림 PR 머지 대기 후 브리지만 작성 | 유지 비용 최소 | 메인테이너 활동 중단(4주). 일정 통제 불가 |
| C. 처음부터 작성(Rust 또는 Bun) | 설계 자유, omp 특화(분기 트리, 비용 표시) | provider·state·레이아웃 재구현. 검증된 Claude/Codex 파서를 버림 |

**A 를 권장한다.** 업스트림 기여는 PR #26 에 리뷰 코멘트와 테스트 결과를 남기고, 이 저장소의 포크는 PR #26 브랜치를 main 에 rebase 한 것에서 출발한다. 메인테이너가 복귀하면 포크 패치를 줄이고 브리지만 남긴다.

### 4.6 경쟁 도구 품질 비교와 넘어야 할 기준

목표는 herdr 안에서 세션 그래프를 보여 주는 도구 중 품질과 성능이 가장 좋은 플러그인이다. 비교 대상은 herdr 플러그인 네 개와 herdr 밖의 세션 시각화 도구 세 개다.

| 도구 | 범위 | 데이터 | 실시간 | 렌더 | 설치 의존 | 라이선스 / ★ / 마지막 push |
|---|---|---|---|---|---|---|
| furkankly/zoetrope | Claude Code, Codex | 전사 JSONL | 200ms 폴링 | ratatui 흐름 그래프, 타임라인 스크럽, 웹(WASM) | `zoe` 바이너리(brew/cargo), 브리지에 `jq` | MIT / 1,015 / 2026-09-11 |
| hanbong5938/herdr-omp-subagents | omp | 프로세스 내 `AgentRegistry` | 1초 폴링 | 사이드바 텍스트 행 | omp 확장(bun) | 없음 / 3 / 2026-09-30 |
| edxeth/herdr-pi-tree | pi | pi 세션 | — | 사이드바 트리 | node | MIT / 4 / 2026-09-30 |
| aemrebarut/herdr-dagr | producer 가 쓰는 `run.json` | 자체 계약 | 파일 감시 | DAG TUI | Rust 빌드 | Apache-2.0(표기 불일치) / 93 / 2026-08-23 |
| patoles/agent-flow (herdr 밖) | Claude Code, Codex | Claude Code 훅 HTTP 서버 + Codex rollout tail | 훅 기반 즉시 | 브라우저 노드 그래프, 타임라인, 파일 히트맵 | `npx agent-flow-app` | Apache-2.0 / 1,683 / 2026-07-11 |
| jayparikh/agentviz (herdr 밖) | Claude Code, Codex, Copilot, ATIF | 전사 파일 | 파일 tail | 브라우저 그래프·트랙·워터폴·비용 | `npx agentviz` | MIT / 95 / 2026-10-01 |
| everettjf/coding-agent-visualizer (herdr 밖) | Claude Code, Codex | 전사 파일 | 없음(사후 분석) | 브라우저 실행 그래프, 스팬 워터폴, 토큰 flame, 파일 히트맵 | 웹 | MIT / 18 / 2026-06-23 |

근거: 각 저장소 README(raw.githubusercontent.com, 2026-10-08)와 api.github.com 메타데이터. 어느 도구도 처리 가능한 파일 크기, 지연, 메모리 같은 성능 수치를 공개하지 않는다.

zoetrope 의 실측 기준선 (`/usr/bin/time -l zoe inspect --provider <p> <file>`, 이 머신, 2026-10-08):

| 입력 | 파일 크기 | 벽시계 | 최대 RSS | 결과 |
|---|---|---|---|---|
| Claude Code 메인 세션 | 96.3 MB | 0.19 s | 110 MB | 5 agents, 1,041 tool calls |
| Codex rollout | 521.0 MB | 0.49 s | 561 MB | 1 agent, 813 tool calls |
| omp 루트 세션, `--provider omp` | 0.8 MB | 0.01 s | — | `unknown provider "omp"; known: claude, codex` |
| omp 루트 세션, provider 자동 감지 | 0.8 MB | — | — | Claude 로 오분류되어 `1 agent(s), 0 tool call(s)` 를 조용히 출력 |

RSS 가 파일 크기와 같은 이유는 `std::fs::read_to_string` 으로 파일 전체를 올리기 때문이다 (`src/tailer/live.rs:430`, `src/tailer/replay.rs:115`, `src/main.rs:184`). 자동 감지 오분류는 PR #26 본문이 지적한 `provider_of` 폴백 버그이며 실제로 재현된다.

zoetrope 이슈 이력(api.github.com, 전체 4건): #10 Codex 지원 요청(closed, 0.2.0 에서 해결), #16 thinking 블록이 표시되지 않음(closed), #15 hermes 지원(open), #24 목록 등재(closed). 사용자 요구는 "에이전트 추가" 와 "표시 누락" 두 축이다.

경쟁 도구가 남긴 빈자리와 herdr-agents-graph 가 넘어야 할 기준:

| 축 | 경쟁 도구 현황 | 목표 기준 (측정 가능) |
|---|---|---|
| 에이전트 범위 | zoetrope main 2종, PR 로 omp/pi 대기. 다른 도구는 omp 없음 | Claude Code, Codex, omp 3종을 한 바이너리에서. 자동 감지 오분류 0건(형식별 fixture 로 고정) |
| 메모리 | zoetrope RSS ≈ 파일 크기(521 MB 파일 → 561 MB) | 스트리밍 파서로 RSS 를 파일 크기와 무관하게 유지. 목표: 500 MB 세션에서 RSS 100 MB 이하 [추정: 설계 목표, 실측 전] |
| 초기 로드 | zoetrope 0.19 s / 96 MB (inspect 기준) | 같은 파일에서 동등 이하. 그래프 첫 프레임까지 200 ms 이내 |
| 실시간 지연 | zoetrope 200 ms 고정 폴링, 유휴에도 CPU 사용. agent-flow 는 훅으로 즉시 | 폴링 유지하되 유휴 시 간격을 늘리고(backoff), 유휴 CPU 0.1% 이하. omp 재작성(rename) 감지 누락 0건 |
| 형식 전진 호환 | zoetrope 는 모르는 레코드 타입을 버림(실측 Claude Code 신규 타입 6종, Codex `token_usage_record` 무시) | 모르는 타입은 무시하되 집계는 유지. 토큰은 `token_usage_record`/`token_count` 까지 읽음 |
| 포커스 추종 | zoetrope 는 열 때 한 번 해석. agentviz 는 최근 파일 하나 | `pane.focused`/`pane.updated` 구독으로 패널 전환·세션 전환(`/new`, `/resume`) 자동 추종 |
| 설치 | zoetrope 는 brew/cargo 와 `jq` 필요. 웹 도구는 node/npx | `[[build]]` 가 릴리스 바이너리를 체크섬과 함께 내려받고, 런타임 의존 0(jq 없이 자체 JSON 처리) |
| 토글·배치 | zoetrope 는 라벨 비교 토글, overlay/split/tab | 같은 키 토글을 `HERDR_PLUGIN_STATE_DIR` 의 패널 id 기록으로 라벨과 무관하게 처리. popup 포함 5종 배치 |
| 비용·토큰 | 웹 도구(agentviz, coding-agent-visualizer)는 비용 flame·캐시 분석 제공. zoetrope 는 출력 토큰 합계 | 노드별 입력/출력/캐시 토큰과 omp `usage.cost` 표시 (2차 범위) |
| 서브에이전트 상태 | zoetrope: 종료 레코드 없는 버전에서 Running 고착 가능(Codex `completed` 누락, Claude task-notification 의존) | 파일 mtime·lock/tombstone·부모 toolResult 세 신호를 합쳐 상태 판정, 고착 0건을 fixture 로 고정 |
| 테스트 | zoetrope: provider 별 골든 테스트, 메모리·타임라인 벤치 | 같은 수준 유지 + 세 에이전트 실파일 수백 개 defensive 파싱(패닉 0) + 벤치를 CI 에서 회귀 검사 |

## 5. 구현 선택지 비교와 제안

### 5.1 언어·TUI

| 선택 | 근거 |
|---|---|
| Rust + ratatui (권장) | zoetrope 포크이므로 자동 결정. file-viewer·reviewr·dagr 도 Rust. 세션 파일이 수 MB(이 머신에서 자식 파일 1 MB, 메인 0.5 MB 이상)라 파싱 비용이 낮아야 함 |
| Bun/TypeScript | omp 확장과 같은 언어라 레지스트리 연동은 쉬움. 그러나 그래프 레이아웃·TUI 를 새로 써야 하고 설치 시 bun 의존이 생김 |

### 5.2 배포

- 저장소 `herdr-agents-graph`: 루트에 `herdr-plugin.toml`, `herdr/{pane.sh, resolve.sh, open.sh, ensure-bin.sh}`. `[[build]]` 가 GitHub Release 에서 플랫폼별 바이너리를 내려받고 체크섬을 확인한 뒤 `HERDR_PLUGIN_STATE_DIR` 아래에 둔다(설치 스크립트 방식). PATH 에 전역 설치를 요구하지 않는다. brew/cargo 는 대안 경로로만 둔다.
- 바이너리 소스: 포크한 zoetrope 저장소의 release workflow. 플러그인 버전과 바이너리 버전을 매니페스트에 함께 적는다(zoetrope 매니페스트 주석은 반대 입장이지만, 설치 스크립트가 URL 을 조립하려면 필요).

### 5.3 내부 모델

zoetrope `FactKind` 를 그대로 쓴다. 에이전트별 매핑:

| Fact | Claude Code | Codex | omp |
|---|---|---|---|
| `Agent{Main}` | 메인 파일 첫 레코드 | `session_meta` (`thread_source != subagent`) | `session` 헤더(`parentSession` 없음 또는 부모 파일 아님) |
| `Spawn{call}` / `Agent{Subagent}` | `tool_use name:Agent` + `toolUseResult.agentId` + `subagents/agent-<id>.jsonl` | `function_call spawn_agent` + `SubAgentActivity started.agent_thread_id` + 자식 `session_meta.parent_thread_id` | `toolCall task.arguments.tasks[].name` + `<Name>.jsonl` + `session_init.agent` |
| `ToolStart/ToolEnd` | `tool_use.id` ↔ `tool_result.tool_use_id` | `function_call.call_id` ↔ `function_call_output.call_id`, `custom_tool_call` | `toolCall.id` ↔ `toolResult.toolCallId` (+`custom tool_execution_start`) |
| `Model`, `Tokens` | `message.model`, `message.usage` (`requestId` 로 dedup) | `turn_context`, `token_count` | `model_change`, `assistant.usage` |
| `Ended` | task-notification user 레코드 | `SubAgentActivity completed` (일부 버전 없음) | 부모 `toolResult(task)` 도착, `.tombstone`, `<Name>.md` 생성 [추정] |
| `Title` | `ai-title` | — | 1행 `title` 슬롯, `title_change` |

### 5.4 실시간 갱신

- 파일 감시 대신 **200ms 폴링 + 바이트 오프셋 + (dev, ino) 식별** 을 유지한다(zoetrope 검증됨). macOS FSEvents 는 지연과 합쳐짐이 있고, omp 의 rename 재작성은 inode 변화 감지로 잡힌다.
- 디렉터리 스캔: Claude Code `subagents/`, omp `<ts>_<uuid>/`, Codex 날짜 디렉터리를 주기적으로 다시 훑어 새 자식 파일을 붙인다(zoetrope `SWITCH_SCAN_EVERY` 패턴).
- 패널 전환: 브리지 패널 프로세스가 `events.subscribe {pane.focused, pane.updated}` 를 별도 연결로 열어, 포커스된 패널의 `agent_session.value` 가 바뀌면 `zoe` 를 새 대상으로 재시작하거나 `--follow` 대상을 바꾼다. MVP 는 재시작으로 충분하다.
- 대용량: 자식 파일을 처음 열 때는 `ReadMode::Whole`, 이후 tail. 메인 파일이 수십 MB 면 스냅샷 사다리(zoetrope #17) 가 seek 비용을 줄인다. 단 zoetrope 의 `read_to_string` 전체 적재는 4.6 절 메모리 기준을 넘지 못하므로, 포크에서는 초기 적재를 `BufReader` 줄 단위 스트리밍으로 바꾸고 원문 줄을 보관하지 않는다(사실만 남김). 이 변경은 `src/tailer/{live.rs, replay.rs}` 와 `src/main.rs` 세 지점에 국한된다.

### 5.5 MVP 범위 제안

1. 포크: `furkankly/zoetrope@b1f31dd` + PR #26 rebase, `cargo test` 와 이 머신의 실제 omp 파일로 `zoe inspect` 검증. 자동 감지 오분류 수정(`provider_of`) 포함.
2. 브리지: `resolve.sh` 가 `kind: path` 를 받아 `zoe --provider omp --follow <path>`; `claude|codex` 는 기존대로 id. 같은 키 토글은 패널 id 를 `HERDR_PLUGIN_STATE_DIR` 에 기록해 라벨 비교 없이 처리. `jq` 의존 제거.
3. 최적화: 초기 적재 스트리밍(메모리 기준), 유휴 폴링 backoff(CPU 기준), omp rename 재작성 감지 fixture, `token_usage_record`/`token_count` 집계.
4. 배포: 릴리스 바이너리 + 체크섬 검증 `[[build]]` 다운로드 스크립트, `herdr plugin install <owner>/herdr-agents-graph`. 런타임 의존 0.
5. 검증: 세 에이전트 실파일 전수 파싱(패닉 0), 4.6 절 기준선과 같은 파일로 벤치 비교표를 README 에 기록.
6. 다음 단계(비 MVP): `pane.focused`/`pane.updated` 구독으로 자동 전환, omp 분기 트리 표시, 노드별 비용 집계.

## 6. 위험과 아직 모르는 것

- **업스트림 정체.** zoetrope 메인테이너가 2026-09-11 이후 활동이 없다. 포크로 가되 PR #26 저자와 중복 작업이 될 수 있다.
- **PR #26 diff 미검토.** 본문과 파일 목록만 확인했다. 분기(`parentId`) 처리, `session_switch` 재작성, `.lock.os`/`.tombstone` 취급이 구현돼 있는지 코드로 확인해야 한다.
- **omp 파일 재작성.** 제목 변경·전환·분기에서 rename 교체가 일어난다. tailer 의 inode 리셋이 이를 받는지, 그리고 재작성 중 부분 읽기가 생기지 않는지 실측이 필요하다(`#rewriteAtomically` 는 publish lock 으로 append 를 막는다고 주석에 적혀 있으나 외부 리더는 보호 대상이 아니다).
- **Claude Code·Codex 패널 실측 부재.** 두 에이전트의 `pane get` 출력은 소스와 훅 스크립트로만 확정했다. 새 패널에서 재확인한다.
- **형식 드리프트.** Claude Code 는 레코드 타입이 버전마다 늘고(이번 실측에 zoetrope 미지원 타입 6종), Codex 는 0.101→0.120 사이에 `source` 구조가 바뀌었다. 모르는 타입은 무시하고 알려진 필드만 읽는 파서 정책이 필요하다.
- **popup 배치.** CLI `--placement` 에 popup 이 없다. 매니페스트나 소켓으로만 가능한지, 의도인지 확인한다.
- **플러그인 패널로의 메시징 부재.** 포커스 전환을 열린 패널에 전달할 공식 API 가 없다. 패널 프로세스가 직접 소켓 구독을 열어야 한다.
- **라이선스.** zoetrope MIT 는 문제없다. hanbong5938 확장은 라이선스가 없어 코드 재사용 불가. dagr 는 표기가 엇갈린다.
- **omp 레지스트리 비노출.** 실행 중/대기 중 같은 실시간 상태는 전사에 늦게 나타난다. 필요하면 hanbong 방식의 보조 확장을 별도 선택 기능으로 둔다.
- 확인하지 못한 것: Codex `state_5.sqlite` 스키마와 `hooks.state.subagent_*` 명령 내용, Claude Code `tasks/<agentId>.output` 의 생성 조건, `HERDR_PLUGIN_CONTEXT_JSON` 실제 샘플(소스 구조체로만 확인), zoetrope 열린 이슈 10건 중 #15 외 목록.

## 7. 출처 목록

실행한 명령(2026-10-08, 이 머신): `herdr --version`, `herdr integration status`, `herdr pane list`, `herdr pane get w5:p1`, `herdr pane get wK:p1`, `herdr --help`, `herdr plugin --help`, `herdr plugin pane open --help`, `herdr plugin list`, `herdr api schema`, `herdr api schema --json`, `herdr --skill`, `omp --version`, `zoe --version`; 세션 파일 레코드 타입·키 빈도 집계(Python, 필드 이름만).

로컬 파일:
- `~/.claude/hooks/herdr-agent-state.sh:53-84`, `~/.claude/settings.json` (hooks 절)
- `~/.codex/herdr-agent-state.sh:51-82`, `~/.codex/config.toml` (키 이름만), `~/.codex/version.json`
- `~/.omp/agent/extensions/herdr-omp-agent-state.ts:101-162, 330-402`
- `~/.bun/install/global/node_modules/@oh-my-pi/pi-coding-agent/src/session/{session-entries.ts:15-56,236-272, session-paths.ts:44-107, session-storage.ts:23-24,257, session-manager.ts:373-377,1677-1699,2251-2633}`, `src/registry/{agent-registry.ts:115,335,348, agent-tombstone.ts:4-6, agent-lifecycle.ts:464-502, persisted-agents.ts:288-343}`
- `~/.config/herdr/plugins/github/furkankly.zoetrope-452e176858b2/{herdr-plugin/herdr-plugin.toml, herdr-plugin/herdr/pane.sh, herdr-plugin/herdr/resolve.sh, herdr-plugin/herdr/open.sh, herdr-plugin/herdr/ensure-zoe.sh, Cargo.toml, src/fact.rs, src/provider/mod.rs, src/provider/claude/{discovery.rs,wire.rs,mod.rs}, src/provider/codex/{discovery.rs,wire.rs,mod.rs}, src/tailer/{live.rs,bytes.rs}, docs/HERDR-PLUGIN.md, docs/DISCOVERY.md}`
- `~/.config/herdr/plugins/github/{persiyanov.reviewr-*, ray.plugin-manager-*, herdr-file-viewer-*, herdr-cwd-*, devswha.herdr-web-ui-*}/herdr-plugin.toml`
- 세션 파일(구조만): `~/.claude/projects/<slug>/<id>.jsonl`, `<id>/subagents/agent-*.jsonl`, `agent-*.meta.json`; `~/.codex/sessions/2026/10/*/rollout-*.jsonl`; `~/.omp/agent/sessions/<slug>/<ts>_<uuid>.jsonl`, `<ts>_<uuid>/<Name>.jsonl`; `~/.omp/run/`, `~/.omp/agent/terminal-sessions/`
- herdr 소스 v0.9.3 (`git clone --depth 1 --branch v0.9.3 --sparse` 로 `/tmp` 에 받아 읽음): `src/agent_resume.rs:98-133,178-197,328`, `src/api/schema/plugins.rs:355-395`, `src/api/schema/panes.rs:372-391,480`, `src/app/api/plugins/context.rs:130-180`, `src/app/api/plugins/runtime.rs:59-62`, `src/api/schema/events.rs:57,245`

웹(모두 2026-10-08 열람):
- https://herdr.dev/llms.txt
- https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/plugins.mdx
- https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/socket-api.mdx
- https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/cli-reference.mdx
- https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/agent-automation.mdx
- https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/add-herdr-support.mdx
- https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/marketplace.mdx
- https://github.com/herdrdev/herdr/blob/v0.9.3/src/api/schema/panes.rs
- https://api.github.com/repos/furkankly/zoetrope , /pulls?state=open , /pulls/26 , /pulls/26/files , /pulls/26/reviews , /issues/26/comments , /commits
- https://raw.githubusercontent.com/furkankly/zoetrope/main/CHANGELOG.md
- https://raw.githubusercontent.com/furkankly/zoetrope/main/docs/HERDR-PLUGIN.md
- https://github.com/furkankly/zoetrope/pull/26
- https://api.github.com/repos/hanbong5938/herdr-omp-subagents , https://raw.githubusercontent.com/hanbong5938/herdr-omp-subagents/main/extension.ts
- https://api.github.com/repos/edxeth/herdr-pi-tree
- https://api.github.com/repos/aemrebarut/herdr-dagr
- https://github.com/topics/herdr-plugin
- https://api.github.com/repos/furkankly/zoetrope/issues?state=all (이슈 4건)
- https://api.github.com/repos/{patoles/agent-flow, jayparikh/agentviz, everettjf/coding-agent-visualizer, alexarthurs/herdr-sidebar} 와 각 README (raw.githubusercontent.com)
- 실측 벤치: `/usr/bin/time -l zoe inspect --provider claude|codex|omp <파일>` 및 `zoe inspect <omp 파일>` (provider 자동 감지), `zoe --help`; zoetrope `src/tailer/live.rs:430`, `src/tailer/replay.rs:115`, `src/main.rs:184`, `benches/{memory.rs, timeline.rs}`
