# 계획 검토 기록

## 1차: 수정 요청

- 1차 리뷰(독립 서브에이전트, reviewer, 모델 설정 anthropic/claude-opus-5-5 high). 필수 결함 11건: provider_of 의 Claude 규칙이 uuid 단독이면 기존 테스트와 실세션 177개가 깨짐(sessionId 또는 uuid 로 수정); 유휴 CPU 의 실제 원인은 tui.rs 16ms 무조건 draw(실측 약 4%)이므로 T4 범위에 tui.rs 추가; T3 가 sidecar Whole 읽기를 잘못 지목했고 실제 대용량 경로는 replay.rs parse_file_into 와 main.rs Tail, bytes.rs 첫 폴 할당; T5 는 token_count 를 이미 세고 있어 이중 집계가 되므로 token_usage_record 가 보이면 대체하도록 변경; [[build]] 는 HERDR_PLUGIN_* 를 받지 않아 설치 위치를 체크아웃 상대 경로로 고정; 패키지 이름 변경 시 lib 이름·lock·workflow 선택자 동반 수정; 이슈 기준 4개(rename 추종, 0.3s, plugin install, Claude·Codex 패널)에 TODO·게이트 추가; G2/G5/G6 명령 수정; popup 액션 제거; jq 대체 수단을 서브커맨드로 통합; cd.yml 의 GPG·homebrew·release-plz 의존 제거. 전부 계획 2판에 반영했다.

## 2차: 수정 요청

- 2차 delta 리뷰(독립 서브에이전트 reviewer, anthropic/claude-opus-5-5 high). 1차 지적 11건은 모두 해소로 판정됐다. 새 필수 결함 6건: (N1) TUI dirty 조건이 카메라 glide·Running 간선 맥동·LIVE_FRESH 만료·chip afterglow 를 빠뜨려 애니메이션이 멈춤 → 애니메이션 진행 중 술어와 단위 테스트 3개 추가; (N2) token_usage_record 우선 설계가 timeline 골든과 521MB 총합(262262≠246129)을 바꿈 → token_count 를 권위 소스로 유지하고 token_usage_record 는 token_count 가 없는 파일의 대체 소스로만 사용; (N3) 저장소가 비공개라 herdr 의 익명 https clone 이 실패 → 사용자 승인으로 공개 전환(isPrivate=false 확인), T10 에 전제 명시; (N4) G1 에 rc 확인 추가; (N5) G8 전에 bin 정리; (N6) idle-cpu.sh 에 stty 200x60 과 측정 pid 명시. 미확인 가정 3건(README grep 범위를 [로컬 경로 생략] 로 축소, invoke 전 pane focus, 업스트림 비교는 /tmp clone 에서)도 반영했다.

## 3차: 수정 요청

- 3차 delta 리뷰(독립 서브에이전트 reviewer, anthropic/claude-opus-5-5 xhigh 상향). 2차 지적 N1~N6 은 모두 해소로 판정됐다. 새 필수 결함 3건: (A) Running 노드가 있으면 매 틱 그리는 술어 때문에 interactive main 이 Running 으로 남는 30~120 s 무변화 구간(INTERACTIVE_IDLE_SECS=120)에서 60 fps 재그리기가 계속돼 CPU 약 4 % → 맥동 phase 전환(약 480 ms)·ants phase 전환(120 ms, 마지막 fact 뒤 30 s 안)에만 그리도록 바꾸고 G5 에 'main 이 Running 인 demo 사본' 시나리오 추가; (B) rataflow auto-pan 은 마우스가 멈춰도 계속 panning 하는데 tui.rs:73 이 반환값을 버려 화면이 멈춤 → tick_auto_pan 반환 Event 또는 is_dragging 이면 dirty, 단위 테스트 추가; (C) 보류한 token_usage_record 의 방출 훅이 없고 라이브 tail 에는 EOF 가 없어 이중 집계 또는 죽은 코드가 됨 → provider::Stream::finish 를 정의해 전체 읽기 경로에서만 호출, 방출 합을 output_tokens 에 더해 뒤이은 token_count 가 초과분만 내게 하고 T5 를 T3 뒤 같은 담당으로 배치. 판별 요청 2건(G8 에 바이너리 미생성 관측 추가, idle-cpu.sh 의 /bin/ps·</dev/null 명시)도 반영했다.

## 4차: 수정 요청

- 4차 delta 리뷰(독립 서브에이전트 reviewer, anthropic/claude-opus-5-5 xhigh). 3차 지적 A·B·C 는 모두 해소로 판정됐다(finish 는 Stream enum match 하나로 들어가며 parse_file_into·하네스·inspect 경로가 모두 stream 을 소유하고, codex/mod.rs:254-255 의 saturating_sub 증가분 계산으로 finish 합을 output_tokens 에 더하면 뒤이은 token_count 가 초과분만 낸다는 점을 확인). 새 필수 결함 2건: (1) 노드 맥동은 0.2.0 에서 phase 가 {0,1,2} 뿐이라 실제로 그려지지 않으므로 거짓 전제였고, pending 도구 chip 의 경과 시간(fmt_dur)과 afterglow fade 가 술어에 없어 멈춤 → 술어를 pending chip 문자열 변화(상한 1 s)·afterglow 0.45/0.75/1.0×TTL 경계로 바꾸고 맥동 규칙 삭제, 단위 테스트 pending_chip_duration_redraws 추가; (2) G5 의 'main 이 Running 인 demo 사본' 은 생성 80 s 뒤 Idle 이 되어 재실행 시 전제가 깨짐 → scripts/make-running-demo.sh 로 측정 직전 생성하고 main_active=1 을 EXPECT 에 추가. 판별 요청 3건(omp arm 도 finish None, Target::Here·Bundle::load 미호출 범위 수용, finish Statement at 은 마지막 보류 레코드 timestamp)도 반영했다.

## 5차: 통과

- 5차 delta 리뷰(독립 서브에이전트 reviewer, anthropic/claude-opus-5-5 xhigh) 통과. 4차 지적 2건과 판별 요청 3건이 모두 해소됐다: fmt_dur 단위(chips.rs:328-341)와 afterglow 경계(chips.rs:419-438)가 계획의 주장과 일치하고, 시간에 따라 바뀌는 텍스트는 단일 chip 하나뿐이며 group chip 의 fade 는 규칙 (4) 가 덮는다. make-running-demo.sh 의 date -u -v-40S 가 macOS 에서 동작하고, Claude provider 는 type:assistant 와 timestamp 만으로 main 활동을 세므로 사본에 추가 필드가 필요 없다.
- 공격 1: fact 가 파일 변화 없이 들어와 ants 정지 타이머를 되살리는 경로를 찾았으나, Batch 는 새 줄이나 새 파일에서만 나가므로(live.rs:327-334) 마지막 fact 시각은 마지막 파일 변화 시각보다 늦을 수 없다. 30 s 정지 규칙은 이슈 기준과 일관된다.
- 공격 2: G5 산술. 사본 생성 T0 기준 main 은 T0+80 s 까지 Running(session.rs:741-748), 측정은 T0+5~35 s, inspect 는 약 T0+36 s 이므로 여유 43 s 다. 측정 구간의 Running main 시나리오에서 술어 (3)~(6) 이 모두 거짓이라 draw 0회로 측정된다(adopt_baseline 이 완료 도구를 숨겨 afterglow 없음, ReplayLoaded 는 last_batch_at 을 세우지 않아 transport 가 Idle 유지).
- 공격 3: T5 finish 단락을 Stream 생성 지점 전부와 대조. main.rs:188, replay.rs:107, 하네스 codex/mod.rs:559-561 은 finish 호출 경로와 같고, live.rs:95·149 는 poll 경로라 호출하지 않으며, item.rs:283 Bundle 은 수용 범위다. 이 머신 rollout 2,822개 중 token_usage_record 만 있는 파일은 0개다.
- 비차단 메모(구현 시 반영): T4 의 Files 에 src/ui/chips.rs 를 추가해 ChipTray·fmt_dur·ttl 에 pub(crate) 접근자를 둔다; pending chip 이 k개면 재그리기 상한은 초당 k회다; 의존 매트릭스 T3 행 Blocks 에 T5 를 포함해 읽는다.
