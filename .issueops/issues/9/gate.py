#!/usr/bin/env python3
"""Gate checks for issue #9 (see gates.md). One gate per call:

    python3 .issueops/issues/9/gate.py G10

Prints what it observed, then `PASS Gn` (exit 0) or `FAIL Gn: why` (exit 1).
Run from the worktree root. G7 was abandoned (see gates.md): it has no check.
"""

import glob
import os
import re
import subprocess
import sys

BASE = "5825116416ca9d306cc35ac2964901aa27a17dc9"


def run(cmd, timeout=3600, env=None):
    p = subprocess.run(
        cmd, capture_output=True, text=True, timeout=timeout, env=env, stdin=subprocess.DEVNULL
    )
    return p.returncode, p.stdout, p.stderr


def done(gate, ok, why=""):
    print(f"PASS {gate}" if ok else f"FAIL {gate}: {why}")
    sys.exit(0 if ok else 1)


def totals(text):
    lines = re.findall(r"^test result: .*$", text, re.M)
    passed = failed = 0
    for line in lines:
        m = re.search(r"(\d+) passed; (\d+) failed", line)
        if m:
            passed += int(m.group(1))
            failed += int(m.group(2))
    return lines, passed, failed


def cargo_tests(gate, filters, want_passed):
    """Run the named tests (exact names, every target) and require exactly
    `want_passed` of them to pass with none failing."""
    rc, out, err = run(["cargo", "test", "--locked", "--all-features", "--", *filters])
    lines, passed, failed = totals(out + err)
    for name in re.findall(r"^test (\S+) \.\.\. (?:ok|FAILED)$", out, re.M):
        print(name)
    print(f"rc={rc} passed={passed} failed={failed}")
    done(gate, rc == 0 and failed == 0 and passed == want_passed, f"rc={rc}, {passed}/{failed}")


def g1():
    rc, out, err = run(["cargo", "test", "--locked", "--all-features"])
    lines, passed, failed = totals(out + err)
    print("\n".join(lines))
    print(f"rc={rc} passed={passed} failed={failed}")
    done("G1", rc == 0 and lines and failed == 0, f"rc={rc}, failed={failed}")


def g2():
    rc, out, err = run(["cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"])
    print((out + err).strip().splitlines()[-1] if (out + err).strip() else "")
    print(f"rc={rc}")
    done("G2", rc == 0, f"rc={rc}")


def g3():
    hits = []
    for root in ["src/ui", "src/state"]:
        for path in glob.glob(f"{root}/**/*.rs", recursive=True):
            if path.startswith("src/ui/seed/"):
                continue
            for n, line in enumerate(open(path), 1):
                if "Color::" in line and "Color::Reset" not in line:
                    hits.append(f"{path}:{n}: {line.strip()}")
    print("\n".join(hits) or "no Color:: outside src/ui/seed")
    print(f"count={len(hits)}")
    done("G3", not hits, f"{len(hits)} hit(s)")


def g4():
    rc, out, _ = run(
        ["git", "diff", "--name-only", BASE, "--", "assets/claude", "assets/codex", "assets/omp"]
    )
    _, untracked, _ = run(
        ["git", "ls-files", "--others", "--exclude-standard", "assets/claude", "assets/codex", "assets/omp"]
    )
    changed = out.split() + untracked.split()
    print(f"changed provider goldens: {changed}")
    done("G4", rc == 0 and changed == ["assets/omp/demo.model.txt"], f"{changed}")


def g5():
    env = dict(os.environ, AG_REAL_SESSIONS="1")
    rc, out, err = run(
        ["cargo", "test", "--release", "--locked", "--test", "real_sessions", "--", "--nocapture"],
        env=env,
    )
    m = re.search(r"^real_sessions: total=(\d+) failed=(\d+).*$", out + err, re.M)
    print(m.group(0) if m else (out + err)[-2000:])
    done("G5", rc == 0 and bool(m) and int(m.group(2)) == 0, f"rc={rc}")


def g6():
    rc, _, err = run(["cargo", "build", "--release", "--locked"])
    if rc != 0:
        done("G6", False, err[-500:])
    readings = {}
    for name, make in [
        ("assets/claude/demo.jsonl", None),
        ("/tmp/g6-running.jsonl", ["scripts/make-running-demo.sh", "/tmp/g6-running.jsonl"]),
    ]:
        if make:
            subprocess.run(make, check=True)
        rc, out, err = run(["scripts/idle-cpu.sh", name], timeout=120)
        print(f"{name}: {out.strip()} {err.strip()}")
        m = re.search(r"idle_cpu_percent=([\d.]+)", out)
        readings[name] = float(m.group(1)) if m else 99.0
    print(f"readings={readings}")
    done("G6", all(r <= 0.5 for r in readings.values()), f"idle={readings}")


def g8():
    cargo_tests("G8", ["--exact", "ui::views::minimap::tests::minimap_shape_ignores_zoom_and_pan"], 1)


def g9():
    cargo_tests("G9", ["--exact", "state::tests::app_opens_on_the_graph_view"], 1)


def g10():
    names = [
        "state::tests::done_siblings_fold_in_rows",
        "state::tests::done_siblings_fold_graph_matches_now",
        "handler::tests::done_siblings_fold_again_on_esc",
        "state::tests::done_siblings_fold_skips_live_heuristic_done",
        "handler::tests::done_siblings_fold_row_click_selects_the_fold",
        "state::tests::done_siblings_fold_survives_seek_back_and_end",
        "state::tests::done_siblings_fold_nested_tree",
        "state::tests::done_siblings_fold_reset_on_session_switch",
        "state::tests::done_siblings_fold_follows_growth_while_paused_at_end",
    ]
    cargo_tests("G10", ["--exact", *names], len(names))


def g10b():
    cargo_tests("G10b", ["--exact", "handler::tests::lanes_j_reaches_every_lane"], 1)


def g10c():
    cargo_tests("G10c", ["--exact", "state::tests::fold_keeps_dragged_positions_across_seek"], 1)


def g10d():
    names = [
        "state::tests::a_fold_formed_on_first_load_sits_below_its_parent",
        "state::tests::folding_the_detailed_agent_closes_the_detail",
        "state::tests::done_siblings_fold_nested_tree",
        "ui::snapshots::a_replay_ending_in_a_fold_refits_the_overview",
    ]
    cargo_tests("G10d", ["--exact", *names], len(names))


def g11():
    cargo_tests(
        "G11",
        ["--exact", "provider::omp::tests::omp_subagent_description_is_the_first_task_line"],
        1,
    )


def g12():
    cargo_tests(
        "G12",
        [
            "--exact",
            "ui::views::detail::tests::detail_facts_line_ends_with_ellipsis",
            "ui::text::tests::tool_count_is_singular_for_one",
        ],
        2,
    )


if __name__ == "__main__":
    gate = sys.argv[1] if len(sys.argv) > 1 else ""
    fn = globals().get(gate.lower())
    if not fn:
        sys.exit(f"unknown gate {gate!r}")
    fn()
