#!/usr/bin/env python3
"""Gate checks for issue #1 (see gates.md). One gate per call:

    python3 .issueops/issues/1/gate.py G4

Prints what it observed, then `PASS Gn` (exit 0) or `FAIL Gn: why` (exit 1).
Run from the worktree root. Session files are found on this machine, not
named, so the ledger carries no personal paths:

- omp: the root session under ~/.omp/agent/sessions with the most children;
- Codex: the largest rollout under ~/.codex/sessions (the 521 MB baseline);
- Claude: the largest transcript under ~/.claude/projects (the 96 MB one).
"""

import glob
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

HOME = os.path.expanduser("~")
BIN = "target/release/agents-graph"
PLUGIN_ID = "m16khb.herdr-agents-graph"


def run(cmd, timeout=1800, env=None, **kw):
    p = subprocess.run(
        cmd, capture_output=True, text=True, timeout=timeout, env=env, stdin=subprocess.DEVNULL, **kw
    )
    return p.returncode, p.stdout, p.stderr


def done(gate, ok, why=""):
    print(f"PASS {gate}" if ok else f"FAIL {gate}: {why}")
    sys.exit(0 if ok else 1)


def largest(pattern):
    files = glob.glob(pattern, recursive=True)
    return max(files, key=os.path.getsize) if files else None


def omp_root_with_most_children():
    best = None
    for root in glob.glob(f"{HOME}/.omp/agent/sessions/*/*.jsonl"):
        kids = len(glob.glob(root[: -len(".jsonl")] + "/*.jsonl"))
        if best is None or (kids, root) > best:
            best = (kids, root)
    return best


def timed(cmd):
    """(rc, stdout, real seconds, max RSS bytes) from /usr/bin/time -l."""
    rc, out, err = run(["/usr/bin/time", "-l", *cmd])
    real = float(re.search(r"([\d.]+) real", err).group(1))
    rss = int(re.search(r"(\d+)\s+maximum resident set size", err).group(1))
    return rc, out, real, rss


def g1():
    rc, out, err = run(["cargo", "test", "--locked"], timeout=3600)
    results = re.findall(r"^test result: .*$", out + err, re.M)
    print(f"rc={rc}")
    print("\n".join(results))
    failed = [r for r in results if " 0 failed" not in r]
    done("G1", rc == 0 and results and not failed, f"rc={rc}, failing: {failed}")


def g2():
    kids, root = omp_root_with_most_children()
    rc, auto, _ = run([BIN, "inspect", root])
    rc2, forced, _ = run([BIN, "inspect", "--provider", "omp", root])
    m = re.search(r"(\d+) agent\(s\), (\d+) tool call\(s\)", auto)
    print(f"omp root: {os.path.basename(root)} with {kids} child file(s)")
    print(f"auto-detect: {m.group(0) if m else auto[:200]!r}")
    print(f"auto == --provider omp: {auto == forced}")
    agents = int(m.group(1)) if m else 0
    done(
        "G2",
        rc == 0 and rc2 == 0 and auto == forced and agents >= 2,
        f"rc={rc}/{rc2}, agents={agents}",
    )


def g4():
    path = largest(f"{HOME}/.codex/sessions/**/rollout-*.jsonl")
    rc, out, real, rss = timed([BIN, "inspect", "--provider", "codex", path])
    summary = re.search(r"\d+ agent\(s\), \d+ tool call\(s\)", out)
    print(f"codex rollout: {os.path.getsize(path)} bytes")
    print(f"maximum resident set size: {rss} bytes, real {real:.2f} s")
    print(f"summary: {summary.group(0) if summary else '?'}")
    done("G4", rc == 0 and rss <= 104857600, f"rc={rc}, rss={rss}")


def g5():
    readings = []
    for target in ["assets/claude/demo.jsonl", "/tmp/demo-running.jsonl"]:
        if target.startswith("/tmp/"):
            subprocess.run(["scripts/make-running-demo.sh", target], check=True)
        rc, out, err = run(["scripts/idle-cpu.sh", target], timeout=120)
        print(f"{target}: {out.strip()} {err.strip()}")
        m = re.search(r"idle_cpu_percent=([\d.]+)", out)
        readings.append(float(m.group(1)) if m else 99.0)
    _, out, _ = run([BIN, "inspect", "/tmp/demo-running.jsonl"])
    active = len(re.findall(r"\[main\].*\(active\)", out))
    print(f"main_active={active}")
    done(
        "G5",
        all(r <= 0.5 for r in readings) and active == 1,
        f"idle={readings}, main_active={active}",
    )


def g6():
    env = dict(os.environ, AG_REAL_SESSIONS="1")
    rc, out, err = run(
        ["cargo", "test", "--release", "--locked", "--test", "real_sessions", "--", "--nocapture"],
        timeout=3600,
        env=env,
    )
    m = re.search(r"^real_sessions: total=(\d+) failed=(\d+)( standalone=\d+)?$", out + err, re.M)
    print(m.group(0) if m else (out + err)[-2000:])
    ok = rc == 0 and m and int(m.group(2)) == 0 and int(m.group(1)) >= 150
    done("G6", bool(ok), f"rc={rc}")


def g7():
    hits = 0
    for script in sorted(glob.glob("herdr-plugin/herdr/*.sh")):
        n = sum("jq" in line for line in open(script))
        print(f"{script}: {n}")
        hits += n
    print(f"total={hits}")
    done("G7", hits == 0, f"{hits} line(s) mention jq")


def fake_release(dest, tamper):
    """A release directory like the workflow's, built from target/release."""
    target = {
        ("Darwin", "arm64"): "aarch64-apple-darwin",
        ("Darwin", "x86_64"): "x86_64-apple-darwin",
        ("Linux", "x86_64"): "x86_64-unknown-linux-musl",
        ("Linux", "aarch64"): "aarch64-unknown-linux-musl",
    }[(os.uname().sysname, os.uname().machine)]
    shutil.rmtree(dest, ignore_errors=True)
    os.makedirs(dest)
    archive = f"agents-graph-{target}.tar.gz"
    subprocess.run(
        ["tar", "czf", f"{dest}/{archive}", "-C", "target/release", "agents-graph"], check=True
    )
    digest = subprocess.run(
        ["shasum", "-a", "256", archive], cwd=dest, capture_output=True, text=True, check=True
    ).stdout
    if tamper:
        digest = ("0" if digest[0] != "0" else "1") + digest[1:]
    with open(f"{dest}/SHA256SUMS", "w") as f:
        f.write(digest)


def install(base):
    env = dict(os.environ, AG_RELEASE_BASE=f"file://{base}")
    return run(["bash", "herdr-plugin/herdr/install.sh"], env=env, timeout=120)


def g8():
    shutil.rmtree("herdr-plugin/bin", ignore_errors=True)
    fake_release("/tmp/fake-release-bad", tamper=True)
    rc, out, err = install("/tmp/fake-release-bad")
    exists = os.path.exists("herdr-plugin/bin/agents-graph")
    print(f"tampered SHA256SUMS: rc={rc} stdout={out.strip()!r} stderr={err.strip()!r} binary_exists={exists}")
    done(
        "G8",
        rc == 1 and "checksum mismatch" in err and not exists,
        f"rc={rc}, exists={exists}",
    )


def herdr(*args):
    rc, out, err = run(["herdr", *args], timeout=60)
    if rc != 0:
        raise RuntimeError(f"herdr {' '.join(args)}: {err.strip()}")
    return json.loads(out) if out.strip().startswith("{") else out


def panes():
    return herdr("pane", "list")["result"]["panes"]


def g9():
    """Install from the pushed branch with a file:// release, toggle the graph
    over this session's own omp pane twice, then uninstall."""
    branch = run(["git", "branch", "--show-current"])[1].strip()
    own = os.environ["HERDR_PANE_ID"]
    fake_release("/tmp/fake-release", tamper=False)
    env = dict(os.environ, AG_RELEASE_BASE="file:///tmp/fake-release")
    run(["herdr", "plugin", "uninstall", PLUGIN_ID], timeout=120)
    rc, out, err = run(
        ["herdr", "plugin", "install", f"m16khb-org/herdr-agents-graph/herdr-plugin", "--ref", branch, "--yes"],
        env=env,
        timeout=600,
    )
    print(f"install rc={rc}: {(out + err).strip()[-400:]}")
    listed = PLUGIN_ID in run(["herdr", "plugin", "list"])[1]
    print(f"listed={listed}")
    seen = []
    try:
        for press in (1, 2):
            herdr("pane", "focus", own)
            run(["herdr", "plugin", "action", "invoke", f"{PLUGIN_ID}.open"], timeout=60)
            time.sleep(3)
            graph = [p["pane_id"] for p in panes() if p.get("label") == "agents-graph" or p.get("title") == "agents-graph"]
            seen.append(graph)
            print(f"after press {press}: graph panes {graph}")
            if press == 1 and graph:
                text = herdr("pane", "read", graph[0])
                print(f"graph pane shows main: {'main' in str(text)}")
    finally:
        herdr("pane", "focus", own)
        rc_un = run(["herdr", "plugin", "uninstall", PLUGIN_ID], timeout=120)[0]
        gone = PLUGIN_ID not in run(["herdr", "plugin", "list"])[1]
        print(f"uninstall rc={rc_un}, gone={gone}")
    done("G9", rc == 0 and listed and len(seen[0]) == 1 and not seen[1] and gone, f"seen={seen}")


def g11():
    path = largest(f"{HOME}/.claude/projects/*/*.jsonl")
    reals = []
    for _ in range(3):
        rc, _, real, rss = timed([BIN, "inspect", "--provider", "claude", path])
        reals.append(real)
    print(f"claude transcript: {os.path.getsize(path)} bytes")
    print(f"real seconds over 3 runs: {reals} (best {min(reals):.2f}), rss {rss} bytes")
    done("G11", rc == 0 and min(reals) <= 0.30, f"best real {min(reals):.2f}")


def g12():
    base = "b1f31dd26bd4e9e513885e39edb78d0850a5d1fe"
    goldens = sorted(
        glob.glob("assets/codex/cli-*.txt") + glob.glob("assets/codex/desktop-*.txt")
    )
    rc_diff = run(["git", "diff", "--exit-code", base, "--", *goldens])[0]
    print(f"codex goldens vs {base[:7]}: {'unchanged' if rc_diff == 0 else 'CHANGED'} ({len(goldens)} files)")
    path = largest(f"{HOME}/.codex/sessions/**/rollout-*.jsonl")
    _, out, _ = run([BIN, "inspect", "--provider", "codex", path])
    tokens = re.findall(r"tokens: (\d+)", out)
    print(f"521 MB rollout tokens: {tokens}")
    done("G12", rc_diff == 0 and tokens and tokens[0] == "246129", f"diff={rc_diff}, tokens={tokens}")


if __name__ == "__main__":
    gate = sys.argv[1] if len(sys.argv) > 1 else ""
    fn = globals().get(gate.lower())
    if not fn:
        sys.exit(f"unknown gate {gate!r}")
    fn()
