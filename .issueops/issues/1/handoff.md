# 인계 자료 (io-4b513b64dc69)

- 작성 시각: 2026-10-08T15:45:34.792158+00:00
- 목적: 이슈 #1 (https://github.com/m16khb-org/herdr-agents-graph/issues/1) 의 MVP 구현. zoetrope@b1f31dd 를 가져와 herdr-agents-graph 로 정체성을 바꾸고 omp provider·스트리밍 적재·TUI dirty·backoff·herdr 서브커맨드·릴리스 설치를 계획대로 구현한다.
- 비목표: pane.focused 구독 자동 전환, omp 분기 트리, 노드별 비용 표시, 업스트림 머지, herdr 설정·다른 플러그인 변경, 이 머신의 zoetrope 제거(후속·별도 승인).
- 승인된 종료점: draft PR 발행 + execution complete. merge·머지 후 정리는 별도 승인.
- source root: /Users/m16khb/Workspace/herdr-agents-graph
- canonical worktree: /Users/m16khb/Workspace/herdr-agents-graph.worktrees/1-agents-graph-mvp (branch 1-agents-graph-mvp)
- base head: 526801f5d324dc4b21fbaca8dd2f9d6b61789515
- full head: 526801f5d324dc4b21fbaca8dd2f9d6b61789515
- diff(base..HEAD): (없음: HEAD == base)
- 계획 경로: /Users/m16khb/Workspace/herdr-agents-graph.worktrees/1-agents-graph-mvp/.issueops/issues/1/artifact/plan.md
- 계획 digest(sha256): 474d3f0b6c8c6f8e0c1e05ea3103cd0215f2f7b7c7dc615dda86916a7037e4d5
- 완료한 조사·검증: docs/research/report.md(커밋 526801f) — herdr 0.9.3 플러그인 표면, 세 에이전트 세션 형식(실파일 확인), 경쟁 도구 기준선(zoe inspect: 96 MB Claude 0.19 s/RSS 110 MB, 521 MB Codex 0.49 s/RSS 561 MB, omp 자동 감지 오분류). 계획은 적대 리뷰 5라운드(revise×4 → pass)를 거쳤고 판정은 위 계획 digest 에 묶여 있다. 환경: macOS arm64, herdr 0.9.3, omp 18.8.4, cargo/rustc 1.97.0, zoe 0.2.0(brew), 저장소 공개(2026-10-08 전환).
- 미완료 작업: 구현 전체(T1~T10). 결과 위치: 워크트리 커밋과 .issueops/evidence/.
- 현재 lifecycle 상태: phase implement 진입 전(stage implement.enter), execution direct, generation 1, 이 자료 작성 뒤 holder 가 release 한다. missing: plan_path(link-plan 필요).
- 새 세션이 먼저 할 일: (1) `issueops next --id io-4b513b64dc69 --json` 으로 released 상태 확인 후 replace preview 부터 next_command 체인을 따라 자기 native actor 로 claim; (2) active(self) 뒤 `issueops link-plan --id io-4b513b64dc69 --plan-path .issueops/issues/1/artifact/plan.md`; (3) compatibility review·gates init(G1~G12 + 5차 리뷰 비차단 메모) 후 implement 진입; (4) 계획 TODO 순서(T1 → Wave 2 → Wave 3)로 구현.
- 읽기 전용 확인 명령: `issueops status --id io-4b513b64dc69 --json`, `issueops execution status --id io-4b513b64dc69 --json`, `git -C /Users/m16khb/Workspace/herdr-agents-graph.worktrees/1-agents-graph-mvp rev-parse HEAD`, `shasum -a 256 /Users/m16khb/Workspace/herdr-agents-graph.worktrees/1-agents-graph-mvp/.issueops/issues/1/artifact/plan.md`.
- 5차 리뷰 비차단 메모(구현 시 반영): T4 Files 에 src/ui/chips.rs 추가(ChipTray·fmt_dur·ttl 에 pub(crate) 접근자); pending chip 이 k개면 재그리기 상한은 초당 k회; 의존 매트릭스 T3 행 Blocks 에 T5 포함.
