# Gates: 1

- [x] G1: 전체 테스트가 통과한다(cargo test --locked rc=0, 실패 0)
  CHECK: python3 .issueops/issues/1/gate.py G1
  EXPECT: PASS G1
  EVIDENCE: test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s | PASS G1
- [x] G2: omp 루트 세션을 provider 지정 없이 inspect 하면 omp 로 감지해 agent(s) ≥ 2
  CHECK: python3 .issueops/issues/1/gate.py G2
  EXPECT: PASS G2
  EVIDENCE: auto == --provider omp: True | PASS G2
- [x] G3: 네 형식 판별 테스트가 통과한다
  CHECK: cargo test --locked provider_of_pins_all_four_formats
  EXPECT: /test result: ok\. 1 passed/
  EVIDENCE: Running unittests src/main.rs (target/debug/deps/agents_graph-9b98ef77e9a517c0) | Running tests/real_sessions.rs (target/debug/deps/real_sessions-bd78a4ee776ea93b)
- [x] G4: 521 MB Codex rollout inspect 의 최대 RSS ≤ 104857600
  CHECK: python3 .issueops/issues/1/gate.py G4
  EXPECT: PASS G4
  EVIDENCE: summary: 1 agent(s), 813 tool call(s) | PASS G4
- [x] G5: 완료 세션과 Running main 사본 모두 유휴 CPU ≤ 0.5 % 이고 사본의 main 이 active
  CHECK: python3 .issueops/issues/1/gate.py G5
  EXPECT: PASS G5
  EVIDENCE: main_active=1 | PASS G5
- [x] G6: 이 머신 실세션 전수 파싱 failed=0
  CHECK: python3 .issueops/issues/1/gate.py G6
  EXPECT: PASS G6
  EVIDENCE: real_sessions: total=3149 failed=0 standalone=123 | PASS G6
- [x] G7: herdr-plugin 셸 스크립트의 jq 참조 0
  CHECK: python3 .issueops/issues/1/gate.py G7
  EXPECT: PASS G7
  EVIDENCE: total=0 | PASS G7
- [x] G8: SHA256SUMS 변조 시 설치 스크립트가 checksum mismatch·rc=1·바이너리 미생성
  CHECK: python3 .issueops/issues/1/gate.py G8
  EXPECT: PASS G8
  EVIDENCE: tampered SHA256SUMS: rc=1 stdout='' stderr='checksum mismatch for agents-graph-aarch64-apple-darwin.tar.gz; nothing installed' binary_exists=False | PASS G8
- [x] G9: 실제 herdr install --ref 후 목록에 보이고 invoke 2회로 패널 생성·삭제
  CHECK: python3 .issueops/issues/1/gate.py G9
  EXPECT: PASS G9
  EVIDENCE: uninstall rc=0, gone=True | PASS G9
- [x] G10: rename 교체 뒤 tail 이 새 파일을 처음부터 다시 읽는다
  CHECK: cargo test --locked tail_reattaches_after_rename
  EXPECT: /test result: ok\. 1 passed/
  EVIDENCE: Running unittests src/main.rs (target/debug/deps/agents_graph-9b98ef77e9a517c0) | Running tests/real_sessions.rs (target/debug/deps/real_sessions-bd78a4ee776ea93b)
- [x] G11: 96 MB Claude 세션 inspect real ≤ 0.30 s
  CHECK: python3 .issueops/issues/1/gate.py G11
  EXPECT: PASS G11
  EVIDENCE: real seconds over 3 runs: [0.2, 0.08, 0.08] (best 0.08), rss 16990208 bytes | PASS G11
- [x] G12: codex 골든 무변경과 521 MB rollout tokens: 246129
  CHECK: python3 .issueops/issues/1/gate.py G12
  EXPECT: PASS G12
  EVIDENCE: 521 MB rollout tokens: ['246129'] | PASS G12
