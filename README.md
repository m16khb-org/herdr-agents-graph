# herdr-agents-graph

A [herdr](https://herdr.dev) plugin that draws the focused agent pane's session
as a live flow graph: the main agent, the subagents it spawned, and every tool
call, updating as the agent works. It reads Claude Code, Codex, and omp
sessions, and opens the one herdr reports for the pane, so there is no file or
id to look up.

## Install

```text
herdr integration install omp            # and/or: claude, codex
herdr plugin install m16khb-org/herdr-agents-graph/herdr-plugin
herdr plugin action invoke setup-keys --plugin m16khb.herdr-agents-graph
```

The agent's herdr integration is what tells herdr which session a pane runs
(an id for Claude Code and Codex, the transcript path for omp). Start the agent
again after installing it: a session that was already running never reports.

The plugin's build step downloads the `agents-graph` release binary named by
the plugin version, checks it against the release's `SHA256SUMS`, and keeps it
inside the plugin checkout (`herdr-plugin/bin/`). It needs `curl`, `tar`, and
`shasum` or `sha256sum`; nothing is compiled, nothing lands on `PATH`, and no
`jq`, Homebrew, or cargo is involved. Releases are published by pushing a
`v<version>` tag (`.github/workflows/cd.yml`).

`setup-keys` binds `prefix+shift+z` to the overlay graph in your herdr config
(backed up first); `remove-keys` takes it out again. Press the key on an agent
pane to open the graph, and again to close it. `open-split` and `open-tab`
place the graph beside the agent or in its own tab. Inside the graph, `?`
lists its keys.

Remove it with `herdr plugin uninstall m16khb.herdr-agents-graph`.

## Supported agents

| Agent | What herdr reports | Session files read |
| --- | --- | --- |
| Claude Code | session id | `~/.claude/projects/<project>/<id>.jsonl` and its `subagents/` |
| Codex | thread id | `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`, children included |
| omp | transcript path | `~/.omp/agent/sessions/<project>/<ts>_<id>.jsonl` and its `<ts>_<id>/<Name>.jsonl` children |

The provider is read off a file's first record, so a session file opens the
same way whichever agent wrote it.

## Command line

The plugin drives the same binary you can run yourself:

```text
agents-graph                     follow the current project's live session
agents-graph <file.jsonl>        replay a session file from the start
agents-graph <id>                replay a session by id, or a unique prefix of one
agents-graph <dir>               follow another project's live session
agents-graph <file> --follow     follow a file's live edge instead of replaying
agents-graph --provider <name>   force the format (claude, codex, omp)
agents-graph inspect <file|id>   print the session tree, no terminal UI
agents-graph herdr resolve       print the focused herdr pane's provider and session
agents-graph herdr toggle [p]    open the graph pane, or close the one it opened
```

Build it with `cargo build --release`; the binary is `target/release/agents-graph`.

## Measured against zoetrope

Same machine (macOS, Apple silicon, 10 cores), same files, zoetrope 0.2.0 from
Homebrew against agents-graph 0.1.0, 2026-10-09. Wall times are the median of
three runs with a warm page cache; memory is the peak resident set size from
`/usr/bin/time -l`. Idle CPU is the CPU time spent between 5 s and 35 s after
opening a finished session with `--follow` in a 200×60 terminal
(`scripts/idle-cpu.sh`).

| Measure | zoetrope 0.2.0 | agents-graph 0.1.0 |
| --- | --- | --- |
| `inspect` a 96 MB Claude Code transcript: wall time | 0.08 s | 0.08 s |
| the same: peak memory | 110 MB | 17 MB |
| `inspect` a 521 MB Codex rollout: wall time | 0.17 s | 0.15 s |
| the same: peak memory | 562 MB | 55 MB |
| idle CPU, finished session on screen | 5.0 % | 0.3 % |
| omp session, provider detected from the file | read as Claude: 1 agent, 0 tool calls | 13 agents, 835 tool calls |

What changed to get there: transcripts are streamed a line at a time instead of
read whole; the terminal redraws only when something on screen changes instead
of at 60 fps; polling slows from 200 ms to 2 s after 30 s without a change; and
omp is a provider of its own rather than being mistaken for Claude Code.

## Relationship to zoetrope

This is a fork of [zoetrope](https://github.com/furkankly/zoetrope) by Furkan
Kalaycioglu (MIT), started from commit `b1f31dd`. The omp provider began as
zoetrope pull request #26 by bl-dev0 and was updated for omp 18.8.4. The graph,
the timeline, and the terminal UI are zoetrope's; this fork adds omp, the
streaming reads, the idle savings, the Codex `token_usage_record` fallback, and
a herdr bridge that needs no `jq` and installs a checksummed release instead of
building or using a binary on `PATH`. See `NOTICE`.

## License

MIT. See `LICENSE` and `NOTICE`.
