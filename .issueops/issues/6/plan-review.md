# 계획 검토 기록

## 1차: 수정 요청

- 1차 리뷰(독립 서브에이전트 reviewer, anthropic/claude-opus-5-5 high) 수정 요청, 필수 11건: 실제 omp usage.cost 는 객체({input,output,cacheRead,cacheWrite,total})라 스칼라 가정이면 줄 전체가 버려짐 → cost.total 출처·Value 기반 관대한 파싱; RedrawGate 가 선택 행만 다뤄 다른 실행 중 행의 경과 시간이 멈춤 → 보이는 모든 실행 중 도구·30 s 카운트·스피너 포함; G5 다중 필터·G6 테스트 이름 불일치 → --lib 경로 필터와 정확한 passed 수; brand 단계는 dark 에서 700 → 테마별 예시; omp summary 를 되돌리면 파싱 규칙 변경 → summary 불변; 그래프 바탕이 rataflow 다크 팔레트로 남음 → Palette 를 토큰으로; j/k 선택과 그래프 패닝 충돌 → J/K·h/l 패닝; 스냅샷이 로컬 시간대 의존 → 표시 오프셋 주입; G10 Running 기준을 1.0 % 로 완화 → 0.5 % 복원과 pending-tool 시나리오 추가; 바이너리 배포물에 Apache 사본 없음 → cd.yml·Cargo include 추가. 판별 요청(타임아웃·입력 비우기, 순수 함수 테마 결정, rustfmt::skip, benches app.flow, stroke.neutral-contrast, compress_gaps 는 재생 규칙)도 반영했다.

## 2차: 수정 요청

- 2차 delta 리뷰(독립 서브에이전트 reviewer, anthropic/claude-opus-5-5 high) 수정 요청. 1차 지적 11건은 모두 해소로 판정됐다(rataflow Palette 8개 필드와 Theme::Custom, omp Usage Value 파싱, pending-tool 사본이 Claude provider 에서 실제 pending 도구를 만드는 점, cd.yml 패키징 단계와 Cargo include 를 소스로 확인). 새 필수 2건: 테마 질의 타임아웃이 1 s 인데 '100 ms 상한' 문장이 남음 → 응답 터미널 +100 ms, 무응답 +1.1 s 로 통일; 레인 공백 접기의 활동 정의·테스트·키맵·끝 구간 처리가 없음 → 활동 정의, 실행 중 구간·끝 구간 비접기, 테스트 4개, z 키 행 추가. 판별 요청(스냅샷은 Mode::Replay 로 결정적 적재, 상단 바 경과 시간을 스탬프에 포함, 입력 비우기는 raw mode 직후, Tab 기존 동작과 테스트 수정, G3 를 위한 스냅샷 도우미 위치, cd.yml 복사본 처리와 G14 아카이브 검사, T2 QA 하위 프로세스)도 반영했다.

## 3차: 통과

- 3차 delta 리뷰(독립 서브에이전트 reviewer, anthropic/claude-opus-5-5 xhigh 상향) 통과. 2차 지적 2건과 판별 요청이 모두 해소됐다: 테마 타임아웃 수치가 225·424행에서 일치하고, 레인 공백 접기의 활동 정의와 테스트 4개(접기·실행 중 비접기·끝 구간 비접기·z 토글)가 서로 모순되지 않으며 fixture 와도 충돌하지 않는다.
- 공격 1: 스냅샷의 벽시계 의존. App::new(Mode::Replay) → ReplayLoaded → go_live → seek(head) 로 끝까지 적재하면 load_replay 가 replay=true 로 두어 now_reference 가 커서를 돌려주므로(timeline.rs:172-178, 430-436) UTC 주입과 함께 결정적이다.
- 공격 2: raw mode 뒤 입력 비우기. ratatui::init(tui.rs:45)이 enable_raw_mode(ratatui init.rs:399)를 부르고 EventStream 은 tui.rs:51-60 에서 만들어지므로, 그 사이에 crossterm poll(0)/read 로 동기 비우기가 가능하다.
- 공격 3: G14 명령. gh release download -O - 가 파이프 출력에서 동작하고 v0.1.0 에서 실제로 실행해 rc=0 을 확인했다. install.sh:59 는 바이너리만 풀어 파일 추가가 설치에 영향이 없다.
- 비차단 메모(구현 시 반영): 스냅샷에서 스피너 위상과 상단 바 경과 시간은 now_reference(또는 App 주입 시계)에서 나와야 하며 준비 단계에서 tick_timeline(ZERO) 로 정착시킬 것; T9 무응답 측정은 TERM=xterm-256color 와 env -u COLORFGBG -u AG_THEME 로; Cargo include 검증은 cargo package --list | grep -c '^design/seed/' = 2 로; 비우기 단계를 T8 에도 적용하고 handler 의 ']' 가 수정키를 무시하는 점에 유의; handler.rs:368 테스트는 그래프 뷰 설정과 노드 타입 교체도 필요; herdr 패널의 OSC 11 응답은 T2 에서 실측.
