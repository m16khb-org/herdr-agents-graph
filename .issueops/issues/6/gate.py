#!/usr/bin/env python3
"""Gate checks for issue #6 (see gates.md). One gate per call:

    python3 .issueops/issues/6/gate.py G9

Prints what it observed, then `PASS Gn` (exit 0) or `FAIL Gn: why` (exit 1).
Run from the worktree root. Session files are found on this machine, not
named, so the ledger carries no personal paths (the largest Codex rollout is
the 521 MB baseline of issue #1).
"""

import glob
import hashlib
import os
import re
import subprocess
import sys

HOME = os.path.expanduser("~")
BIN = "target/release/agents-graph"
BASE = "1c744d24931a6634f8dea129c218f1823775f1c0"


def run(cmd, timeout=3600, env=None):
    p = subprocess.run(
        cmd, capture_output=True, text=True, timeout=timeout, env=env, stdin=subprocess.DEVNULL
    )
    return p.returncode, p.stdout, p.stderr


def done(gate, ok, why=""):
    print(f"PASS {gate}" if ok else f"FAIL {gate}: {why}")
    sys.exit(0 if ok else 1)


def results(text):
    return re.findall(r"^test result: .*$", text, re.M)


def passed(line):
    m = re.search(r"(\d+) passed; (\d+) failed", line)
    return (int(m.group(1)), int(m.group(2))) if m else (0, 0)


def cargo_tests(gate, args, want_passed=None, at_least=None):
    """Run `cargo test --locked <args>`; sum the result lines (other targets
    filter to 0 passed) and compare the totals."""
    rc, out, err = run(["cargo", "test", "--locked", *args])
    lines = results(out + err)
    total = [sum(x) for x in zip(*(passed(r) for r in lines))] if lines else [0, 0]
    print(f"rc={rc}")
    print("\n".join(lines))
    print(f"passed={total[0]} failed={total[1]}")
    ok = rc == 0 and total[1] == 0
    if want_passed is not None:
        ok = ok and total[0] == want_passed
    if at_least is not None:
        ok = ok and total[0] >= at_least
    done(gate, ok, f"rc={rc}, totals={total}")


def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def g1():
    rc, out, err = run(["cargo", "test", "--locked"])
    lines = results(out + err)
    print(f"rc={rc}")
    print("\n".join(lines))
    failing = [r for r in lines if " 0 failed" not in r]
    done("G1", rc == 0 and lines and not failing, f"rc={rc}, failing: {failing}")


def g2():
    before = sha("src/ui/seed/tokens.rs")
    env = dict(os.environ, UPDATE_SEED="1")
    rc, out, err = run(["cargo", "test", "--locked", "seed_tokens"], env=env)
    after = sha("src/ui/seed/tokens.rs")
    rc2, out2, err2 = run(["cargo", "test", "--locked", "seed_tokens"])
    source = open("design/seed/SOURCE.md").read()
    mismatched = []
    for path in sorted(glob.glob("design/seed/*.yaml")):
        digest = sha(path)
        if digest not in source:
            mismatched.append(path)
        print(f"{digest}  {path}")
    print(f"tokens.rs before={before} after={after}")
    print(f"regenerate rc={rc}, compare rc={rc2}; yaml not in SOURCE.md: {mismatched}")
    done(
        "G2",
        rc == 0 and rc2 == 0 and before == after and not mismatched,
        f"rc={rc}/{rc2}, changed={before != after}, mismatched={mismatched}",
    )


def g3():
    hits = []
    for root in ["src/ui", "src/state"]:
        for path in glob.glob(f"{root}/**/*.rs", recursive=True):
            if path.startswith("src/ui/seed/"):
                continue
            for n, line in enumerate(open(path), 1):
                if "Color::" in line:
                    hits.append(f"{path}:{n}: {line.strip()}")
    print("\n".join(hits) or "no Color:: outside src/ui/seed")
    print(f"count={len(hits)}")
    done("G3", not hits, f"{len(hits)} hit(s)")


def g4():
    cargo_tests("G4", ["ui::seed::theme"], at_least=3)


def g5():
    cargo_tests(
        "G5",
        [
            "--lib",
            "--",
            "omp_tool_intent_and_cost",
            "omp_object_cost_keeps_the_rest_of_the_line",
            "claude_agent_description_is_intent",
            "codex_spawn_task_name_is_intent",
        ],
        want_passed=4,
    )


def g6():
    cargo_tests("G6", ["--lib", "ui::snapshots::"], want_passed=12)


def g7():
    cargo_tests("G7", ["badge_renders_glyph_and_word"], want_passed=1)


def g8():
    cargo_tests("G8", ["keymap_matches_table"], want_passed=1)


def largest(pattern):
    files = glob.glob(pattern, recursive=True)
    return max(files, key=os.path.getsize) if files else None


def g9():
    rc, _, err = run(["cargo", "build", "--release", "--locked"])
    if rc != 0:
        done("G9", False, err[-500:])
    path = largest(f"{HOME}/.codex/sessions/**/rollout-*.jsonl")
    rc, out, err = run(["/usr/bin/time", "-l", BIN, "inspect", "--provider", "codex", path])
    rss = int(re.search(r"(\d+)\s+maximum resident set size", err).group(1))
    summary = re.search(r"\d+ agent\(s\), \d+ tool call\(s\)", out)
    print(f"codex rollout: {os.path.getsize(path)} bytes")
    print(f"maximum resident set size: {rss} bytes")
    print(f"summary: {summary.group(0) if summary else '?'}")
    done("G9", rc == 0 and rss <= 104857600, f"rc={rc}, rss={rss}")


def g10():
    rc, _, err = run(["cargo", "build", "--release", "--locked"])
    if rc != 0:
        done("G10", False, err[-500:])
    readings = {}
    for name, make in [
        ("assets/claude/demo.jsonl", None),
        ("/tmp/demo-running.jsonl", ["scripts/make-running-demo.sh", "/tmp/demo-running.jsonl"]),
        (
            "/tmp/demo-pending.jsonl",
            ["scripts/make-running-demo.sh", "--pending-tool", "/tmp/demo-pending.jsonl"],
        ),
    ]:
        if make:
            subprocess.run(make, check=True)
        rc, out, err = run(["scripts/idle-cpu.sh", name], timeout=120)
        print(f"{name}: {out.strip()} {err.strip()}")
        m = re.search(r"idle_cpu_percent=([\d.]+)", out)
        readings[name] = float(m.group(1)) if m else 99.0
    print(f"readings={readings}")
    done("G10", all(r <= 0.5 for r in readings.values()), f"idle={readings}")


def g11():
    env = dict(os.environ, AG_REAL_SESSIONS="1")
    rc, out, err = run(
        ["cargo", "test", "--release", "--locked", "--test", "real_sessions", "--", "--nocapture"],
        env=env,
    )
    m = re.search(r"^real_sessions: total=(\d+) failed=(\d+).*$", out + err, re.M)
    print(m.group(0) if m else (out + err)[-2000:])
    ok = rc == 0 and m and int(m.group(2)) == 0
    done("G11", bool(ok), f"rc={rc}")


def g12():
    rc, out, err = run(
        ["git", "diff", "--exit-code", BASE, "--", "assets/claude", "assets/codex", "assets/omp"]
    )
    rc2, untracked, _ = run(
        ["git", "ls-files", "--others", "--exclude-standard", "assets/claude", "assets/codex", "assets/omp"]
    )
    print(f"git diff rc={rc}; untracked under goldens: {untracked.split() or 'none'}")
    done("G12", rc == 0 and not untracked.strip(), f"rc={rc}, untracked={untracked.split()}")


def g15():
    rc, out, err = run(["cargo", "package", "--list", "--locked", "--allow-dirty"])
    seed = [l for l in out.splitlines() if l.startswith("design/seed/")]
    print(f"rc={rc}")
    print("\n".join(seed))
    print(f"count={len(seed)}")
    done(
        "G15",
        rc == 0 and sorted(seed) == ["design/seed/LICENSE", "design/seed/NOTICE"],
        f"rc={rc}, {seed}",
    )


def g16():
    hits = []
    for path in glob.glob("src/ui/**/*.rs", recursive=True):
        for n, line in enumerate(open(path), 1):
            if re.search(r"\b(Utc|Instant|Local|SystemTime)::now\b", line):
                hits.append(f"{path}:{n}: {line.strip()}")
    print("\n".join(hits) or "no wall-clock reads in src/ui")
    print(f"count={len(hits)}")
    done("G16", not hits, f"{len(hits)} hit(s)")


if __name__ == "__main__":
    gate = sys.argv[1] if len(sys.argv) > 1 else ""
    fn = globals().get(gate.lower())
    if not fn:
        sys.exit(f"unknown gate {gate!r}")
    fn()
