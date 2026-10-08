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


def rpc(method, params):
    """One request on herdr's socket. `plugin.action.invoke` here can name
    the pane it is invoked for, which the CLI cannot, so nothing has to steal
    focus to aim the action."""
    import socket

    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(30)
    s.connect(os.environ.get("HERDR_SOCKET_PATH", f"{HOME}/.config/herdr/herdr.sock"))
    s.sendall((json.dumps({"id": "ag-qa", "method": method, "params": params}) + "\n").encode())
    buf = b""
    while not buf.endswith(b"\n"):
        chunk = s.recv(65536)
        if not chunk:
            break
        buf += chunk
    s.close()
    return json.loads(buf)


def graph_panes():
    return [p["pane_id"] for p in herdr("pane", "list")["result"]["panes"] if p.get("label") == "agents-graph"]


def wait_for(check, seconds=15):
    deadline = time.time() + seconds
    while time.time() < deadline:
        value = check()
        if value:
            return value
        time.sleep(0.5)
    return check()


def toggle_twice(target, agent):
    """Press `open` for `target` twice: the first press should add one graph
    pane whose main card is the `agent` session (`● omp`, `● claude`, ...),
    the second should remove it."""
    before = set(graph_panes())
    invoke = {"plugin_id": PLUGIN_ID, "action_id": "open", "context": {"focused_pane_id": target}}
    answer = rpc("plugin.action.invoke", invoke)
    print(f"press 1 for {target}: {json.dumps(answer)[:160]}")
    opened = wait_for(lambda: [p for p in graph_panes() if p not in before])
    card = re.compile(rf"[●◌✓✗■] {agent}\b")
    shows_main = False
    if opened:
        text = wait_for(lambda: card.search(herdr("pane", "read", opened[0])) and herdr("pane", "read", opened[0]), 20)
        lines = [l.strip(" │") for l in str(text).splitlines() if card.search(l)]
        shows_main = bool(lines)
        print(f"  graph pane {opened}, main card: {lines[:1]}")
    rpc("plugin.action.invoke", invoke)
    closed = wait_for(lambda: not [p for p in graph_panes() if p not in before])
    print(f"press 2: graph pane gone={closed}")
    return len(opened) == 1 and shows_main and closed


def install_from_branch():
    branch = run(["git", "branch", "--show-current"])[1].strip()
    fake_release("/tmp/fake-release", tamper=False)
    env = dict(os.environ, AG_RELEASE_BASE="file:///tmp/fake-release")
    run(["herdr", "plugin", "uninstall", PLUGIN_ID], timeout=120)
    rc, out, err = run(
        ["herdr", "plugin", "install", "m16khb-org/herdr-agents-graph/herdr-plugin", "--ref", branch, "--yes"],
        env=env,
        timeout=600,
    )
    print(f"install --ref {branch} rc={rc}: {(out + err).strip()[-300:]}")
    listed = [l for l in run(["herdr", "plugin", "list"])[1].splitlines() if PLUGIN_ID in l]
    print(f"plugin list: {listed}")
    return rc == 0 and bool(listed)


def uninstall():
    rc = run(["herdr", "plugin", "uninstall", PLUGIN_ID], timeout=120)[0]
    gone = PLUGIN_ID not in run(["herdr", "plugin", "list"])[1]
    print(f"uninstall rc={rc}, gone={gone}")
    return gone


def g9():
    """Install from the pushed branch with a file:// release, toggle the graph
    for this session's own omp pane twice, then uninstall."""
    ok_install = install_from_branch()
    try:
        ok_toggle = ok_install and toggle_twice(os.environ["HERDR_PANE_ID"], "omp")
    finally:
        gone = uninstall()
    done("G9", ok_install and ok_toggle and gone, f"install={ok_install} toggle={ok_toggle} gone={gone}")


def agent_pane(command, cwd):
    """A pane of our own, split off this one, running `command`, once herdr
    reports its session and the session's transcript exists (Claude Code
    writes it with the first exchange, a moment after it reports the id).
    Returns (pane id, agent_session)."""
    own = os.environ["HERDR_PANE_ID"]
    split = herdr("pane", "split", own, "--direction", "down", "--cwd", cwd)
    pane = split["result"]["pane"]["pane_id"]
    herdr("pane", "run", pane, command)
    session = wait_for(lambda: herdr("pane", "get", pane)["result"]["pane"].get("agent_session"), 60)
    if session and session.get("kind") == "id":
        wait_for(lambda: run([BIN, "inspect", session["value"]])[0] == 0, 60)
    return pane, session


def codex_pane(cwd):
    """A pane of our own running Codex. Codex holds a new session at "Hooks
    need review" until the user trusts herdr's changed hook — a decision this
    QA does not make for them — so the prompt is skipped (esc, hooks stay
    untrusted) and the session the hook would have reported is reported the
    same way it does, with `herdr pane report-agent-session`."""
    started = time.time()
    pane, _ = agent_pane("codex 'Reply with the single word ok.'", cwd)
    if wait_for(lambda: "Hooks need review" in herdr("pane", "read", pane), 20):
        herdr("pane", "send-keys", pane, "esc")
        print("codex: skipped the hook-review prompt (hooks left untrusted)")

    def rollout():
        for path in glob.glob(f"{HOME}/.codex/sessions/**/rollout-*.jsonl", recursive=True):
            if os.path.getmtime(path) < started:
                continue
            meta = json.loads(open(path).readline())["payload"]
            if meta.get("cwd") == cwd and meta.get("thread_source") == "user":
                return meta["id"]
        return None

    thread = wait_for(rollout, 60)
    if thread:
        herdr(
            "pane", "report-agent-session", pane, "--source", "herdr:codex",
            "--agent", "codex", "--agent-session-id", thread,
        )
    session = herdr("pane", "get", pane)["result"]["pane"].get("agent_session")
    return pane, session


def t10():
    """Real herdr QA across the three agents (evidence, not a gate: it starts
    Claude Code and Codex). Every pane it touches is its own."""
    results = {}
    ok_install = install_from_branch()
    try:
        results["omp"] = toggle_twice(os.environ["HERDR_PANE_ID"], "omp")
        # Each agent starts in a directory it already trusts, so no trust
        # prompt stands between the start and its first session report.
        for agent, start in [
            ("claude", lambda: agent_pane("claude 'Reply with the single word ok.'", f"{HOME}/Workspace")),
            ("codex", lambda: codex_pane(f"{HOME}/Workspace/issueops")),
        ]:
            pane, session = start()
            print(f"{agent} pane {pane}: agent_session={session}")
            try:
                results[agent] = bool(session) and session.get("kind") == "id" and toggle_twice(pane, agent)
            finally:
                herdr("pane", "close", pane)
        # Failure path: the pane script's own message for an omp pane that has
        # no session yet, shown in a pane of our own.
        bare = next(
            (p["pane_id"] for p in herdr("pane", "list")["result"]["panes"] if p.get("agent") == "omp" and not p.get("agent_session")),
            None,
        )
        if bare:
            here = os.getcwd()
            split = herdr("pane", "split", os.environ["HERDR_PANE_ID"], "--direction", "down", "--cwd", here)
            pane = split["result"]["pane"]["pane_id"]
            ctx = json.dumps({"focused_pane_id": bare})
            herdr(
                "pane", "run", pane,
                f"HERDR_PLUGIN_CONTEXT_JSON='{ctx}' HERDR_PLUGIN_ROOT={here}/herdr-plugin bash {here}/herdr-plugin/herdr/open.sh",
            )
            text = wait_for(lambda: "integration install omp" in str(herdr("pane", "read", pane)) and herdr("pane", "read", pane), 20)
            print("\n".join(l for l in str(text).splitlines() if l.strip())[-600:])
            results["omp-without-session"] = "Herdr has no session path for this omp pane" in str(text)
            herdr("pane", "close", pane)
    finally:
        results["uninstalled"] = uninstall()
    print(f"results: {results}")
    done("T10", ok_install and all(results.values()), str(results))


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
