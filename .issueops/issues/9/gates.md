# Gates: 9

- [x] G1: 전체 테스트 통과(cargo test --locked --all-features, failed 0)
  CHECK: python3 .issueops/issues/9/gate.py G1
  EXPECT: PASS G1
  EVIDENCE: rc=0 passed=340 failed=0 | PASS G1
- [x] G2: clippy 경고 0(cargo clippy --all-targets --all-features -- -D warnings)
  CHECK: python3 .issueops/issues/9/gate.py G2
  EXPECT: PASS G2
  EVIDENCE: rc=0 | PASS G2
- [x] G3: src/ui(seed 제외)와 src/state에 Color:: 리터럴 0
  CHECK: python3 .issueops/issues/9/gate.py G3
  EXPECT: PASS G3
  EVIDENCE: count=0 | PASS G3
- [x] G4: provider 골든 중 assets/omp/demo.model.txt만 바뀜
  CHECK: python3 .issueops/issues/9/gate.py G4
  EXPECT: PASS G4
  EVIDENCE: changed provider goldens: ['assets/omp/demo.model.txt'] | PASS G4
- [x] G5: 실제 세션 파싱 실패 0(real_sessions failed=0)
  CHECK: python3 .issueops/issues/9/gate.py G5
  EXPECT: PASS G5
  EVIDENCE: real_sessions: total=3204 failed=0 standalone=123 | PASS G5
- [x] G6: 끝난 데모와 Running 사본의 유휴 CPU ≤ 0.5 %(기본 그래프 화면)
  CHECK: python3 .issueops/issues/9/gate.py G6
  EXPECT: PASS G6
  EVIDENCE: readings={'assets/claude/demo.jsonl': 0.23, '/tmp/g6-running.jsonl': 0.1} | PASS G6
- [ ] G7: 선택 후 그래프 전환에서 모든 카드가 캔버스 안(graph_switch_keeps_every_card_in_view)
  CHECK: python3 .issueops/issues/9/gate.py G7
  EXPECT: PASS G7
  EVIDENCE: pending
- [x] G8: 미니맵 모양이 확대·축소·이동과 무관(minimap_shape_ignores_zoom_and_pan)
  CHECK: python3 .issueops/issues/9/gate.py G8
  EXPECT: PASS G8
  EVIDENCE: rc=0 passed=1 failed=0 | PASS G8
- [x] G9: 기본 화면이 그래프(app_opens_on_the_graph_view)
  CHECK: python3 .issueops/issues/9/gate.py G9
  EXPECT: PASS G9
  EVIDENCE: rc=0 passed=1 failed=0 | PASS G9
- [x] G10: 완료 묶기 규칙과 지금·그래프 일치(done_siblings_fold 9개)
  CHECK: python3 .issueops/issues/9/gate.py G10
  EXPECT: PASS G10
  EVIDENCE: rc=0 passed=9 failed=0 | PASS G10
- [x] G10b: 레인 보기의 j/k가 모든 레인에 닿음(lanes_j_reaches_every_lane)
  CHECK: python3 .issueops/issues/9/gate.py G10b
  EXPECT: PASS G10b
  EVIDENCE: rc=0 passed=1 failed=0 | PASS G10b
- [x] G10c: 접기와 seek가 끌어 둔 카드 위치를 바꾸지 않음(fold_keeps_dragged_positions_across_seek)
  CHECK: python3 .issueops/issues/9/gate.py G10c
  EXPECT: PASS G10c
  EVIDENCE: rc=0 passed=1 failed=0 | PASS G10c
- [x] G10d: 구현 리뷰 수정 4건(첫 적재 묶음 카드는 부모 아래, 묶일 때 detail 닫힘, esc는 직속 묶음만 접음, 재생 끝 묶음 뒤 Overview 재맞춤)
  CHECK: python3 .issueops/issues/9/gate.py G10d
  EXPECT: PASS G10d
  EVIDENCE: rc=0 passed=4 failed=0 | PASS G10d
- [x] G11: omp description이 session_init.task의 첫 문장 줄(omp_subagent_description_is_the_first_task_line)
  CHECK: python3 .issueops/issues/9/gate.py G11
  EXPECT: PASS G11
  EVIDENCE: rc=0 passed=1 failed=0 | PASS G11
- [x] G12: 사실 줄 생략 표시와 단수 복수(detail_facts_line_ends_with_ellipsis, tool_count_is_singular_for_one)
  CHECK: python3 .issueops/issues/9/gate.py G12
  EXPECT: PASS G12
  EVIDENCE: rc=0 passed=2 failed=0 | PASS G12
ABANDON: G7 사용자 의도 정정(2026-10-09): 항목 1은 prefix+shift+z로 켜면 바로 그래프 보기가 나오는 것이며 항목 6(G9 app_opens_on_the_graph_view)이 덮는다. 선택 후 전환 맞춤 동작은 구현하지 않는다(feedback contract_change, 이슈 본문 갱신).
