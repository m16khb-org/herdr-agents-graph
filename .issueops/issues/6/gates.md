# Gates: 6

- [x] G1: 전체 테스트가 통과한다(cargo test --locked rc=0, 실패 0)
  CHECK: python3 .issueops/issues/6/gate.py G1
  EXPECT: PASS G1
  EVIDENCE: test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s | PASS G1
- [x] G2: UPDATE_SEED=1 재생성 뒤 design·tokens.rs diff 0, YAML sha256 이 SOURCE.md 와 같다
  CHECK: python3 .issueops/issues/6/gate.py G2
  EXPECT: PASS G2
  EVIDENCE: regenerate rc=0, compare rc=0; yaml not in SOURCE.md: [] | PASS G2
- [x] G3: src/ui(seed 제외)·src/state 의 Color:: 리터럴 0
  CHECK: python3 .issueops/issues/6/gate.py G3
  EXPECT: PASS G3
  EVIDENCE: count=0 | PASS G3
- [x] G4: 테마·256 대체 테스트 passed ≥ 3, 0 failed
  CHECK: python3 .issueops/issues/6/gate.py G4
  EXPECT: PASS G4
  EVIDENCE: passed=11 failed=0 | PASS G4
- [x] G5: intent·cost 테스트 4개 통과
  CHECK: python3 .issueops/issues/6/gate.py G5
  EXPECT: PASS G5
  EVIDENCE: passed=4 failed=0 | PASS G5
- [x] G6: 뷰 스냅샷 12개 통과
  CHECK: python3 .issueops/issues/6/gate.py G6
  EXPECT: PASS G6
  EVIDENCE: passed=12 failed=0 | PASS G6
- [x] G7: 상태 배지가 글리프+단어를 렌더(badge_renders_glyph_and_word)
  CHECK: python3 .issueops/issues/6/gate.py G7
  EXPECT: PASS G7
  EVIDENCE: passed=1 failed=0 | PASS G7
- [x] G8: 키맵 표 테스트(keymap_matches_table) 통과
  CHECK: python3 .issueops/issues/6/gate.py G8
  EXPECT: PASS G8
  EVIDENCE: passed=1 failed=0 | PASS G8
- [x] G9: 521 MB Codex rollout inspect 의 최대 RSS ≤ 104857600
  CHECK: python3 .issueops/issues/6/gate.py G9
  EXPECT: PASS G9
  EVIDENCE: summary: 1 agent(s), 813 tool call(s) | PASS G9
- [x] G10: 끝난 세션·Running 사본·pending-tool 사본 유휴 CPU 모두 ≤ 0.5 %
  CHECK: python3 .issueops/issues/6/gate.py G10
  EXPECT: PASS G10
  EVIDENCE: readings={'assets/claude/demo.jsonl': 0.3, '/tmp/demo-running.jsonl': 0.17, '/tmp/demo-pending.jsonl': 0.13} | PASS G10
- [x] G11: 이 머신 실세션 전수 파싱 failed=0
  CHECK: python3 .issueops/issues/6/gate.py G11
  EXPECT: PASS G11
  EVIDENCE: real_sessions: total=3172 failed=0 standalone=123 | PASS G11
- [x] G12: Claude·Codex·omp 골든이 1c744d2 대비 무변경
  CHECK: python3 .issueops/issues/6/gate.py G12
  EXPECT: PASS G12
  EVIDENCE: git diff rc=0; untracked under goldens: none | PASS G12
- [x] G15: cargo package --list 에 design/seed/ 항목이 정확히 2개(LICENSE·NOTICE)
  CHECK: python3 .issueops/issues/6/gate.py G15
  EXPECT: PASS G15
  EVIDENCE: count=2 | PASS G15
- [x] G16: src/ui 에 Utc::now·Instant::now·Local::now 직접 호출 0(시계는 App 주입)
  CHECK: python3 .issueops/issues/6/gate.py G16
  EXPECT: PASS G16
  EVIDENCE: count=0 | PASS G16
