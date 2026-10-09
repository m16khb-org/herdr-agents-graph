# herdr-agents-graph

A [herdr](https://herdr.dev) plugin that shows what the focused agent pane is
doing right now: the main agent and the subagents it spawned, what each one is
working on and why, which tool it is running and for how long, and what
failed. It reads Claude Code, Codex, and omp sessions, and opens the one herdr
reports for the pane, so there is no file or id to look up.

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
`v<version>` tag (`.github/workflows/cd.yml`); until the `v0.1.0` release
exists the build step has nothing to download, and `AG_RELEASE_BASE` can point
it at a directory of locally built archives instead.

`setup-keys` binds `prefix+shift+z` to the overlay graph in your herdr config
(backed up first); `remove-keys` takes it out again. Press the key on an agent
pane to open the graph, and again to close it. `open-split` and `open-tab`
place the graph beside the agent or in its own tab. Inside it, `?` lists every
key.

Remove it with `herdr plugin uninstall m16khb.herdr-agents-graph`.

## Supported agents

| Agent | What herdr reports | Session files read |
| --- | --- | --- |
| Claude Code | session id | `~/.claude/projects/<project>/<id>.jsonl` and its `subagents/` |
| Codex | thread id | `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`, children included |
| omp | transcript path | `~/.omp/agent/sessions/<project>/<ts>_<id>.jsonl` and its `<ts>_<id>/<Name>.jsonl` children |

The provider is read off a file's first record, so a session file opens the
same way whichever agent wrote it.

## The screen

The top bar names the agent and the session and says whether it is live,
replaying or paused, how long it has run, how many output tokens it has
written, and — for omp, which records it — what it has cost. The bar at the
bottom counts what needs attention (failed agents, tools running for more than
30 s) and lists the keys that work in the current view.

| View | Key | What it shows |
| --- | --- | --- |
| Now (default) | `1` | One row per agent, as a tree: its status, what it is doing (the running tool's stated intent, else its task, else its latest thought), the tool in flight with its timer, and its tokens. Failures rise to the top. `enter` lists the selected agent's last five tool calls. |
| Lanes | `2` | One lane per agent along time; every tool call is a bar coloured by its result. Long idle stretches fold to `┆12m┆` (`z` unfolds them), and `←` `→` move the cursor. |
| Graph | `3` | The spawn tree on a canvas you can pan and zoom. |

From 100 columns the Now view keeps the selected agent's detail open on the
right: the prompt and the thought that led to its spawn, and every tool call
with its intent and time. In a narrower pane `enter` twice opens the detail
full-size.

Agent rows and the detail show a status as a glyph and a word as well as a
colour (`● running`, `✓ done`, `✗ failed`, `◌ idle`, `■ stopped`); a row whose
agent is running a tool shows the tool and its timer in place of the word.
A failed call's lane bar ends in `✗`, and a graph zoomed out to cells keeps
each agent's glyph.

## Keys

| Key | Action |
| --- | --- |
| `tab`, `shift-tab`, `1` `2` `3` | switch view |
| `j` `k`, `↓` `↑` | select the next or previous agent (scroll inside a full-size detail) |
| `enter`, `esc` | expand the row, then open its detail; go back |
| `space` | play or pause |
| `[` `]` | previous or next prompt |
| `G`, `g`, `end` | back to live |
| `←` `→` | Lanes: move the cursor · Graph: select sideways |
| `z` | Lanes: fold idle gaps |
| `h` `l`, `H` `J` `K` `L` | Graph: pan |
| `+` `-` `0` | Graph: zoom in, zoom out, reset |
| `f`, `o`, `r` | follow the active agent, frame everything, rearrange the graph |
| `s` | replay: skip idle gaps or play in real time |
| `i`, `?` | session details, every key |
| `pgup` `pgdn` | scroll the detail's tool list |
| `q`, `ctrl-c` | quit |

Changed from 0.1.0:

| Key | 0.1.0 | Now |
| --- | --- | --- |
| `tab`, `shift-tab` | next or previous node in the graph | next or previous view |
| `j` `k` | scroll the detail panel; pan the graph with no selection | select the next or previous agent in every view; scroll a full-size detail |
| `↑` `↓` | move between graph nodes by position | select the next or previous agent |
| vertical pan | `j` `k` | `J` `K` |
| `o`, `r`, `s`, `i` | shown in the status bar | unchanged, listed under `?` only |

## Colours

The colours are the design tokens of
[SEED Design](https://github.com/daangn/seed-design) (Apache-2.0), pinned under
`design/seed/` and compiled into `src/ui/seed/tokens.rs`;
`scripts/sync-seed.sh <commit>` followed by
`UPDATE_SEED=1 cargo test --locked seed_tokens` updates both. SEED's brand role
is the carrot orange of its owner; here it takes the same steps of SEED's own
purple palette instead, and no logo, name or character of Daangn Market is used.
This project is not affiliated with or endorsed by Daangn Market Inc. (당근마켓).

The theme follows the terminal. On start the graph asks the terminal for its
background colour (OSC 11) and picks the light or dark tokens; a herdr pane
answers within a millisecond, while a terminal that never answers delays the
first frame by about one second. Without an answer it reads `COLORFGBG`, and
failing that it is dark. These variables decide instead:

| Variable | Values | Effect |
| --- | --- | --- |
| `AG_THEME` | `light`, `dark` | use these tokens and do not ask the terminal |
| `AG_COLOR` | `256`, `truecolor` | force 256 colours or 24-bit colour; otherwise 24-bit when `COLORTERM` is `truecolor` or `24bit` |
| `AG_BG` | `none` | keep the terminal's own background (for transparent terminals) |

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

The redesigned screens were measured the same way on the same day: idle CPU is
0.33 % with a finished session on screen and 0.17 % with a tool timer ticking
every second, where 0.1.0 measured 0.63–0.67 % on the machine that afternoon.
While nothing on screen moves by itself the loop now wakes five times a second
instead of sixty. The release binary grew from 4.08 MB to 4.28 MB for the
theme query and the token tables.

## Relationship to zoetrope

This is a fork of [zoetrope](https://github.com/furkankly/zoetrope) by Furkan
Kalaycioglu (MIT), started from commit `b1f31dd`. The omp provider began as
zoetrope pull request #26 by bl-dev0 and was updated for omp 18.8.4. The graph
layout and the timeline are zoetrope's. The screens, the keys and the colours
were redrawn on SEED Design's tokens, so the Now and Lanes views have no
counterpart there. This fork also adds omp, the streaming reads, the idle
savings, the Codex `token_usage_record` fallback, and a herdr bridge that needs
no `jq` and installs a checksummed release instead of building or using a
binary on `PATH`. See `NOTICE`.

## License

MIT. See `LICENSE` and `NOTICE`. The SEED Design tokens under `design/seed/`
and the generated `src/ui/seed/tokens.rs` are Apache-2.0
(`design/seed/LICENSE`, `design/seed/NOTICE`); release archives carry those as
`LICENSE-APACHE-SEED` and `NOTICE-SEED`.
