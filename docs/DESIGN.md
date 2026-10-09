# herdr-agents-graph — Design Document (v1)

**herdr-agents-graph** (binary `agents-graph`) is a terminal UI and Herdr plugin that visualizes Claude Code, Codex, and omp agent sessions as a live flow graph: the main agent, its subagents, workflows, and tool activity — rendered with [rataflow](https://crates.io/crates/rataflow). The design comes from [zoetrope](https://github.com/furkankly/zoetrope), which this project forks.

> **This is the v1 structural spec** (module map, transcript format, type shapes). For the *invariants and principles* the implementation now follows — order-independence, the content-vs-presentation clocks, ground-truth-over-heuristics, and the derived-state heuristics catalogue — see [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Hard constraints

1. **No network IO, ever.** All IO is local filesystem. No reqwest/hyper/etc. in the dep tree (dev-deps included). "Your transcripts never leave your machine" goes in the README.
2. **Defensive parsing.** The transcript format is undocumented/internal. Unknown entry types, missing fields, malformed lines: skip, never panic. Mirrors Claude Code's own resilience.
3. **rwy's async architecture**, ported: single-task UI loop owns all state; background tokio tasks feed typed messages over bounded mpsc; no `Arc<Mutex>`.
4. Dependencies live in `Cargo.toml`, with the reason for each beside it; the split that matters is the portable core versus the `native` feature (tokio, crossterm, the filesystem).

## CLI

One TUI command over the unified timeline engine (see the **Timeline** section) plus the headless `inspect`. The launch only picks **defaults** — *what* to open and *where the playhead starts*; once open, scrub / follow / pause / go-live all work regardless.

```
agents-graph             # follow the current project's live session
agents-graph <file.jsonl>         # replay a session file, played from the start (any provider's file)
agents-graph <id>                 # replay a session by id, or a unique prefix, across providers
agents-graph <dir>                # follow another project's live session
agents-graph <file> --follow      # ride a file's live edge instead of replaying it
agents-graph <file> --speed 8     # playback speed (default 8.0)
agents-graph --provider codex ... # force the format instead of reading it off the content
agents-graph inspect <file|id>    # no TUI: print session info + parsed tree (smoke-test)
```

Resolution goes through `provider::open` (see [DISCOVERY.md](DISCOVERY.md)): a **file** is `Target::Path` and bulk-loads + tails (replay feeder), the provider read off its first record; an **id** is `Target::Id`, looked up across every provider's roots; a **dir** (or none → cwd) is `Target::Here` and follows the newest session of that project, any provider. `--follow` only changes the start position (head vs beginning) via `Mode`. `Cli = View { target: Option<String>, follow: bool, speed: f64, provider: Option<Provider> } | Inspect { file, provider }`. Arg parsing: hand-rolled over `std::env::args` (no clap; keep deps lean).

## Workspace layout

One crate, `herdr-agents-graph` (library `agents_graph`, binary `agents-graph`): the portable core plus the native frontend, split by a Cargo feature (`default = ["native"]`). With `native` off the library builds as a portable core — model, timeline, graph, UI rendering, providers — with no runtime and no IO of its own.

```text
Cargo.toml          # one package: the library + the `agents-graph` bin (required-features = ["native"])
src/                # the agents_graph library + the bin (src/main.rs); src/herdr.rs is `agents-graph herdr resolve|toggle`
herdr-plugin/       # the Herdr plugin: a manifest and shell scripts that launch `agents-graph`; ships by git clone, its build step downloads the release binary
```

## Dependencies (Cargo.toml)

`edition = "2024"`, `license = "MIT"`.

```toml
# Portable core: model, timeline, graph, UI rendering, parsing.
ratatui       = { version = "0.30", default-features = false }
rataflow      = { version = "0.1.0", default-features = false, features = ["sugiyama"] }
serde         = { version = "1", features = ["derive"] }
serde_json    = "1"
chrono        = { version = "0.4", features = ["serde", "clock"] }
web-time      = "1"          # Instant/SystemTime; re-exports std on native
imbl          = "7"          # persistent collections: a SessionModel clone shares structure
unicode-width = "0.2"        # display-column width for truncation (CJK/emoji are 2 cols)

# native feature → the native frontend (all optional, gated #[cfg(feature = "native")])
crossterm = { version = "0.29", features = ["event-stream"], optional = true }
tokio     = { version = "1", features = ["rt-multi-thread","macros","time","sync"], optional = true }
futures   = { version = "0.3", optional = true }
anyhow    = { version = "1", optional = true }
# native also flips on ratatui/crossterm + rataflow/crossterm.

```

**One binary:** `agents-graph` → `src/main.rs` (`required-features = ["native"]`). It reaches users as a release archive, downloaded by the plugin's build step, not from a package manager. No network deps anywhere (hard constraint #1; the download happens in the plugin's shell script, outside the binary).

## Module map

```
src/
├── lib.rs         # crate root — the portable core shared by the native frontend and any other host
├── main.rs        # native binary + CLI parsing; spawns the tailer task, runs the TUI; the `inspect` subcommand
├── tui.rs         # terminal lifecycle + the central native event loop (tick_clock/tick_camera/tick_timeline/status_tick/draw, drawing only when the frame is stale; 16 ms tick while in motion, 200 ms idle)
├── handler.rs     # input routing: one keymap for every view → App, whitelisted navigation → the flow; mouse via `App.hit`; process_flow_events
├── herdr.rs       # native-only: `agents-graph herdr resolve|toggle`, the Herdr plugin's JSON handling (see HERDR-PLUGIN.md)
├── fact.rs        # the provider boundary: Fact + FactKind, the vocabulary every provider speaks and the model folds
├── provider/
│   ├── mod.rs     # the input side: Provider enum, SessionFile, Session, the Stream enum, open / sweep / assemble (DISCOVERY.md)
│   ├── harness.rs # (test) the conformance check: fold a fixture in shuffled orders and diff the result against its goldens
│   ├── claude/
│   │   ├── mod.rs       # the Claude provider: Entry → Facts (`facts`, `Record`) and the per-file `Stream`; the tool-summary lexicon
│   │   ├── wire.rs      # Claude's serde model for JSONL entries + meta.json sidecars
│   │   └── discovery.rs # the ~/.claude/projects layout: cwd sanitization, session / subagent / journal scans, the primitives
│   ├── codex/
│   │   ├── mod.rs       # the Codex provider: rollout lines → Facts through a `Stream` that learns its thread from its first line; `token_usage_record` is a fallback token source only when a rollout has no `token_count` (`Stream::finish`)
│   │   ├── wire.rs      # Codex's serde model: the envelope, `response_item`, `event_msg` and its `item_completed` items
│   │   └── discovery.rs # the ~/.codex/sessions/YYYY/MM/DD layout: rollouts, the head read that classifies them, the primitives
│   └── omp/
│       ├── mod.rs       # the omp provider: session records → Facts through a per-file `Stream` (the interactive root, or a spawned child's own transcript)
│       ├── wire.rs      # omp's serde model for its session JSONL
│       └── discovery.rs # the ~/.omp/agent/sessions/<project-key> layout: roots, `<ts>_<uuid>/` child directories, the primitives
├── state/
│   ├── mod.rs     # App: owns the Flow + SessionModel + Timeline + SessionInfo + UI state; handle_ui_event, seek, camera
│   ├── session.rs # SessionModel: the pure domain model (agents, statuses, tool calls) folded from Facts — knows no format
│   ├── timeline.rs# Timeline: the ts-ordered item list + playhead (time-travel); pacing, gap-compression, seek, floor
│   ├── graph.rs   # incremental SessionModel → Flow projection (never rebuilds except backward seek; Sugiyama on `r`)
│   ├── frame.rs   # RedrawGate / FrameStamp / `in_motion`: whether the frame on screen is stale, so a quiet session stops repainting
│   ├── info.rs    # SessionInfo: untimed session metadata, folded off the timeline (i overlay + inspect header)
│   └── render.rs  # the headless text view: what `inspect` prints, and what a provider's golden test compares
├── tailer/        # background FEEDER (pure: no pacing/seeking — the App owns the playhead)
│   ├── mod.rs     # task entry + shared wire types (TailRequest / UiEvent); the wire carries Statements
│   ├── live.rs    # live tailing — one poll loop per session (200 ms, slowing after 30 s quiet), emits UiEvent::Batch
│   ├── replay.rs  # replay assembly (native): parse all files up front, merge by ts, then keep tailing
│   ├── item.rs    # portable replay-stream pieces — ReplayItem (a statement + its Timing), dating, and Bundle (files as text in, streams kept for appends); IO-free
│   └── bytes.rs   # incremental byte reader: streaming `read_lines` for the first read, `read_appended` in 1 MiB chunks, split-on-\n / buffer-partial (pure, testable)
└── ui/
    ├── mod.rs        # draw: top bar, body (the active view, detail beside or over it), hint bar; overlays and the snackbar last
    ├── chrome.rs     # the top bar, the hint bar, the help and session-info overlays
    ├── text.rs       # width-aware truncation, wrapping, and the duration / token / cost / clock formatters
    ├── snapshots.rs  # (test) golden frames of each view in both themes → assets/ui
    ├── views/
    │   ├── mod.rs    # shared naming and badges, `attention` (failed agents, slow tools)
    │   ├── now.rs    # the Now view: one row per agent, what it is doing
    │   ├── lanes.rs  # the Lanes view: one lane per agent along time
    │   ├── detail.rs # the detail panel for the selected agent
    │   └── graph.rs  # the Graph view: rataflow canvas + AgentNode / AgentEdge renderers
    └── seed/
        ├── mod.rs    # SEED Design integration
        ├── tokens.rs # GENERATED from design/seed by the generator test; do not edit
        ├── gen.rs    # (test) the generator and its up-to-date check
        ├── theme.rs  # Theme: token column (light/dark × truecolor/256) and the semantic styles; the only place that builds a Color
        ├── snapshot.rs # (test) buffer → text + style runs named by token
        └── widgets/  # SEED components: Badge, Callout, Chip, Divider, KeyHint, ListItem, ProgressDots/Spinner, Skeleton, Snackbar, TabList
```

## Claude Code transcript format (verified against real data, Claude Code 2.1.153–2.1.165)

This is one provider's format, modelled in `provider/claude/wire.rs` and found on disk by `provider/claude/discovery.rs`. Nothing in the core (`fact`, `state`, `ui`) depends on any of it; the feeders reach a provider only through `provider::open`, `sweep` and `provider_of`.

### Layout
- Main: `~/.claude/projects/<sanitized-cwd>/<session-uuid>.jsonl`. Sanitization: absolute cwd, every `/` → `-` (leading slash → leading dash). Only `<uuid>.jsonl` directly in that dir are transcripts.
- Subagents: `<session-uuid>/subagents/agent-<agentId>.jsonl` + `agent-<agentId>.meta.json`. `agentId` = 17 hex chars (NOT a UUID), also present as a field on every line of its file.
- Workflow subagents: `<session-uuid>/subagents/workflows/<wf-id>/agent-*.jsonl` + `.meta.json` + `journal.jsonl` (ledger of `started`/`result` entries — NOT a transcript; `result` entries carry `agentId` = workflow-subagent completion marker).
- **Ignore**: `vercel-plugin/skill-injections.jsonl` (no `type` field), `memory/`, `sessions-index.json`, `tool-results/` (output overflow spill).

### meta.json
`{agentType: String, description: Option<String>, toolUseId: Option<String>}`. Direct Agent calls have all three; workflow subagents have only `agentType: "workflow-subagent"`.
**Linkage:** `meta.toolUseId` === the `Agent` tool_use block `.id` in the main transcript; `meta.agentType` === `tool_use.input.subagent_type`.

### Entries — `#[serde(tag = "type")]` + `#[serde(other)] Unknown`
- **Transcript entries** (`user`, `assistant`, `system`, `attachment`): envelope has `uuid`, `parentUuid` (null only on the single root — distinguish *present-and-null* from *absent*), `timestamp` (ISO8601 UTC millis, e.g. `2026-06-05T13:51:15.151Z`), `sessionId`, `isSidechain` (false in main, true in subagent files), optional `promptId`, `requestId` (assistant-only). Subagent lines add `agentId` (all lines) and `attributionAgent` (assistant lines only — do NOT rely on it; join on `agentId`).
- **assistant**: `.message = {role, model, content[], stop_reason, usage}`. Content block types: `text {text}`, `thinking {thinking, signature}`, `tool_use {id, name, input, caller?}` (`caller` is newer-schema; Option). `usage.output_tokens` etc. — sub-fields vary by version, all Option/default. `model` e.g. `claude-opus-4-8`.
- **user**: `.message.content` is **string OR array** (untagged enum). Array blocks: `text`, `tool_result {tool_use_id, content, is_error?}` — `content` is **also string-or-array**; `is_error` is `Option<bool>`, **missing means success**. Top-level optional `toolUseResult` (object|string) sibling of `.message`.
- **system**: subtypes via `subtype` field (`turn_duration`, `stop_hook_summary`, `local_command`, …) — keep as a lean variant, mostly ignored.
- **attachment**: `.attachment.type` various — context injections, not graph material.
- **Flat metadata entries** (`ai-title`, `last-prompt`, `mode`, `permission-mode`, `file-history-snapshot`, `queue-operation`): NO uuid/parentUuid/timestamp — must deserialize into lean variants (a struct requiring envelope fields fails). **`ai-title` provides the session title for the header bar.**
- **Ledger entries** (`started {key, agentId}`, `result {key, agentId, result}`): appear in subagent files and journal.jsonl. Excluded from graph; `result` in journal.jsonl marks workflow-subagent completion.
- `summary` type: documented to exist, never observed — handled by Unknown.
- Format is strict JSONL: one JSON object per line, no embedded newlines, no blob lines. Longest observed line 38KB; largest file 2MB/792 lines.

### Tool calls
`tool_use` names observed: Bash, Edit, Read, Write, ToolSearch, AskUserQuestion, Agent, Workflow, TaskStop, WebFetch. `Agent` input: `{description, prompt, subagent_type}`. Pair `tool_use.id` with the later user `tool_result.tool_use_id` → pending (no result yet = in-flight) vs complete (`is_error` decides Failed).

## Domain model (state/session.rs)

Folded from `Fact`s only, through one entry point, `apply_fact`; the model never
sees a provider's records. Collections are `imbl` persistent structures, so a
snapshot is an O(1) clone (the seek ladder depends on that).

```rust
pub struct SessionModel {
    pub session_id: String,
    pub(crate) agents: OrdMap<String, AgentInfo>, // keyed by node id ("main", the provider's agent id, or a group id)
    pub(crate) spawn_order: Vector<String>,       // stable discovery order (map key order ≠ spawn order)
    pub last_activity: Option<DateTime<Utc>>,
    // Order-independent join stores — a fact attaches whether it arrives before or
    // after the thing it refers to (ARCHITECTURE.md §1.1):
    //   labels:          agent_id → (agent_type, description) — `Label` facts (a group named before it exists)
    //   completed_calls: call_id  → (is_err, end_ts)          — every `ToolEnd`; a spawning call's end is the ACK
    //   ended:           agent_id → AgentStatus               — `Ended` facts: authoritative, outrank the ack
    //   spawn_context:   call_id  → SpawnContext              — provenance from `Spawn` facts (era + reasoning)
    //   prompts:         Vector<PromptInfo>                   — the root's prompt eras (prompt_for_ts attribution)
    //   last_reasoning:  agent_id → String                    — cross-record reasoning fallback
}
pub struct AgentInfo {
    pub kind: AgentKind,                 // Main | Subagent | Group
    pub interactive: bool,               // main/fork — no completion signal; selects the liveness branch
    pub agent_type: Option<String>,      // "claude-code-guide", "workflow-subagent", "fork", …
    pub description: Option<String>,     // meta.description or Agent tool_use input.description
    pub parent: Option<String>,          // node id of parent (main or wf-id)
    pub spawned_by: Option<String>, // the spawning call id — the spawn/ack join key
    pub status: AgentStatus,             // Running | Idle | Done | Failed | Stopped
    pub(crate) terminal: bool,           // authoritative completion — pins against time-derived revival
    pub model: Option<String>,
    pub tool_calls: Vec<ToolCallInfo>,   // {id, name, summary: Option<String>, ts, state: Pending|Ok|Err}
    pub output_tokens: u64,              // summed from usage (deduped per requestId)
    pub first_ts, last_ts: Option<DateTime<Utc>>,
    // + internal indices: tool_index (tool_use_id → slot), seen_request_ids (token-dedup)
}
```

**Untimed session metadata → `SessionInfo` (not the timeline).** Session-level facts (`Title`, `Session { label, value }`, `Tally`) carry no timestamp and are not activity, so they would otherwise clump at the front of the sorted timeline. Feeders route them into `SessionInfo { title, fields: Vec<(label, value)>, tallies }` instead, whichever record carried them (`Statement::take_session_meta`; a Codex root names itself and its app on one line): the provider labels the rows (Claude: `mode`, `permission`, `last prompt`; tallies `queued`, `file edits`; Codex: `app`, `version`, `cwd`), values are latest-wins per label in first-seen order, and the `i` overlay and `inspect` render whatever rows arrived (`state::render`). Lives on `App` (not `SessionModel`), so it survives backward-seek rebuilds — it's session-constant. The title is on `SessionInfo`, NOT on `SessionModel`, so the model holds only timed, foldable state.

**Graph topology (v1): nodes are agents, not messages.** One node per agent + one group node per workflow run. Edges: one parent edge per agent, id `e-<child id>` (`graph::edge_id`), `workflow → its subagents`. Sessions have 800+ lines — per-message nodes would be noise; agents are the story.

**Status rules** — the concrete derivations; the *principles* they follow (ground-truth-over-heuristics, reversibility, the async completion model) are in [`ARCHITECTURE.md`](ARCHITECTURE.md) §2–4.

- **The `Agent` tool result is a SPAWN ACK, not a completion** (`"Async agent launched successfully"`). A direct subagent is completed by the main-transcript `tool_result` (`tool_use_id == spawned_by`, `is_error` → Failed) **only if the ack is not superseded** by the agent's own later activity — `resolve_spawn_status`: `last_ts > ack_ts` ⇒ still `Running`, non-terminal. A superseded (async) subagent stays `Running` and settles to `Done` at `end_of_stream`.
- **`<task-notification>` is the authoritative terminal report** for a background agent. The Claude provider states it as `Ended(Done | Stopped | Failed)`; the fold records it in the `ended` store, where it **outranks** the ack and time-derived liveness, and pins `terminal`. (`meta.stoppedByUser` is deliberately **not** applied — the meta folds at the agent's *first* activity, so applying it would strand the agent `Stopped` for the whole replay; only the timestamped notification is trusted.)
- **Workflow subagent**: `Done` when `journal.jsonl` has a `result` naming its `agentId` (the provider states `Ended(Done)`; pins `terminal`). Group node (`recompute_group_status`): an all-children-**terminal** rollup (`Failed` if any child failed, else `Done`; `Done`/`Stopped` both count as terminal; a childless group stays `Running`; re-derived every call, so it reverts if a running child is discovered late). The Workflow tool_use's `tool_result` is a *launch ack* ("Workflow launched in background…"), NOT a completion — never complete groups from it.
- **Liveness** (`recompute_liveness`, against the timeline's **`now` reference** — wall clock at a live edge, the playhead otherwise, so a scrubbed/paced view shows the as-of-then state with no wall-clock bleed): "active" = `now − last_ts ≤ INTERACTIVE_IDLE_SECS` (~2 min) **OR the agent holds a pending tool_call** (an unresolved tool is direct proof it's working — §2.2/§4). Interactive → `Running`/`Idle` (never claims completion); non-interactive non-terminal → `Running`/`Done`, **reversible**; non-interactive terminal → keeps its status. `end_of_stream` settles interactive agents to `Idle` and any still-`Running` async agent to `Done`.
- Edge `animated = target agent Running`.

## Tailer — the feeder (tailer/)

**Decision: poll-based, no `notify` dep** (poll is simpler, and 200ms is imperceptible). Polling starts at 200 ms and backs off to 500 ms, 1 s, then 2 s after 30 s without a change (`src/tailer/live.rs`); a change snaps it back. The initial read streams line by line (`tailer::read_lines`) and returns a tail state carrying the file's identity, so a rename over the file between the read and the first poll counts as a reset. The tailer is a **pure feeder** — it no longer paces or seeks (that moved to the App's `Timeline`); it only produces an ordered update stream and keeps the files watched.

```rust
pub enum TailRequest { Watch(Target) }                   // switch session; only request now (a path, an id, or a directory to follow)
pub enum UiEvent {
    ReplayLoaded { session_id, items: Vec<ReplayItem>, speed, info: SessionInfo }, // bulk hand-off
    Batch { session_id: String, statements: Vec<Statement> }, // per poll tick, one per record read
    SessionReset { session_id: String },                 // truncation/rotation/auto-switch
    Error(String),
}
pub struct Statement { at: Option<DateTime<Utc>>, facts: Vec<Fact> } // what one record stated, and when it was written
pub struct ReplayItem { timing: Timing, pub facts: Vec<Fact> }      // one statement + its Timing; .ts() → Some only when Dated
pub enum Timing {                     // how an item is placed on the timeline (tailer/item.rs)
    Dated(DateTime<Utc>),             // has, or has derived, a real timestamp
    Pending(String /* agent */),      // undated fact about an agent — awaits a cross-file join on that agent
    Leader,                           // genuinely undated, about no agent — rides at the head permanently
}
```

The wire carries `Statement`s (`src/fact.rs`), not any provider's records: one
per record read, holding the record's own time plus the facts it stated. The
two times differ on purpose — a record sits on the timeline where the file
wrote it (the only moment a live viewer could have known it), while each fact
keeps its own time for the fold (a completion record can also say when the
call started). A feeder opens a session with `provider::open` and holds one
`provider::Stream` per tailed file (the provider's own stream inside: Claude's
carries the inherited timestamp for lines that lack one, Codex's the thread
its file is by) and calls `push(line)`; whole-read sidecars (Claude's
`meta.json`) are stated once through `Provider::sidecar`, and files that
appear later come from `Session::rescan`. Nothing above a feeder sees a
provider's record or path vocabulary. `Timing` for an undated statement reads its facts' envelopes —
`Pending(agent)` if it is about an agent, else `Leader` — and dating joins a
birth (`Agent`) to that agent's first dated activity and an ending (`Ended`)
to its last.

**Two load strategies, one tail loop.** Both feeders end in the shared `tail_loop`, so EVERY session keeps tailing for appends (a replayed file that grows just "goes live" on its own — completion is unknowable, so nothing is ever assumed finished):
- **File target** (`run_replay`): `build_replay` parses every session file, dates undated statements — sidecar births, ledger endings — (`date_and_sort`), **routes session-level statements into `SessionInfo`** (off the timeline via `Statement::is_session_meta`), and merges the rest into a ts-sorted `Vec<ReplayItem>` → one `ReplayLoaded`. Then enter `tail_loop`, resuming each file's tail from the **byte offset the parse consumed** (a *snapshot seed* — not live EOF — so lines appended *during* the parse aren't dropped). Auto-switch disabled (`follow = None`; you asked for this file).
- **Dir/none target** (`run_live`): announce `SessionReset` (id adoption), then `tail_loop` — the first poll backfills the existing file (arrival order); subsequent polls emit appends; the project dir is re-scanned for a *newer* session (throttled auto-switch: `SWITCH_SCAN_EVERY`~2s, only after `SWITCH_IDLE`~30s idle, dir targets only).

Per-file tail state `{ offset, partial, overflowed, identity: (dev, ino) }`. Each tick: stat the file; a shrink (`len < offset`) **or an inode swap** (rotation — a different `(dev,ino)` even if not shorter) → reset + `SessionReset` and re-attach; grown → read appended bytes, split on `\n`, hand complete lines to the file's provider `Stream`, buffer the trailing partial (a runaway line past `MAX_PARTIAL`=8 MiB is dropped, not buffered forever). asks `Session::rescan` for files that appeared; absent dirs are fine).

**Everything stamped with session_id; App drops events where `!is_current(session_id)`** (rwy's identity-stamping; stale buffered messages across a switch).

## Timeline — the unified replay/live model (state/timeline.rs)

**Live and replay are NOT two modes — one time-shifted timeline (time-travel).** There is one ts-ordered item list and one playhead (`cursor`); the only real difference is whether the right edge is fixed (a finished file) or growing (a session being written). The **edge is always the last event** — never wall-clock now — so an old session never grows an empty tail toward the present.

```rust
pub struct Timeline {
    pub items: Vec<ReplayItem>,   // bulk-loaded, then appended as the feeder tails
    pub replay: bool,             // launch intent: replaying a session file vs following live. NOT a completeness claim (a replay can grow & go live). Not runtime-derivable. Does NOT gate pacing (the edge does); gates the `now` reference (playhead vs wall-clock) + the end-settle latch
    pub cursor: Option<DateTime<Utc>>,  // playhead = the universal "now" for rendering
    pub folded: usize,            // items applied to the derived model so far
    pub follow_head: bool,        // pinned to the edge (playing/following) vs parked (scrubbed)
    pub speed: f64,
    pub compress_gaps: bool,      // skip idle dead air (default on); `s` toggles faithful pacing
    // + cached head, gap-pacing anchor/elapsed, `undated_agents` (Pending-item join set), ended latch
}
```

- **Pin-vs-pace is decided by the edge, not the mode.** `advance`/`append_live` compare cursor-vs-head: behind the edge the cursor always **paces forward**; only at the edge does it **pin** (and live appends snap in). So `space` resumes from the playhead in *both* modes, and a scrubbed-back live session **catches up** to the edge then follows — there's no "play = jump to live." `End`/`go_live` is the explicit jump.
- **`replay`** is the one surviving "mode" bit — the **launch intent**: are you replaying a session file, or following a live session? Set from the launch `Mode`, and **not runtime-derivable** (a quiet live session is byte-identical to a finished file, so the flag can't be eliminated). It is NOT a claim the file is complete — a replay can grow and go live (the feeder always tails; nothing is assumed finished), which is why it's named for the intent, not a "bounded/complete" property. It does NOT gate pacing (the edge does); it gates only the `now` reference and the end-settle latch.
- **Pacing** (`advance`, per 16ms frame): paces the cursor toward the next event, **compressing dead air** — but not with a flat cap. `compress_gap` is a **log-compression** curve (`GAP_FAITHFUL_KNEE`=0.8s, `GAP_COMPRESS_SCALE`=0.6): real-time below the knee, then `knee + scale·ln(1 + (t−knee)/knee)` above it — *graded*, so a 5-minute wait still reads longer than a 5-second one (an hour of dead air crosses in <10s). The `s` key sets `compress_gaps = false` for faithful real-time pacing. The App folds the prefix `items[0..fold_target()]` (`App::fold_to`); the live append and replay paths share it.
- **`now` reference** = wall clock only at a *live* edge (`!replay && follow_head && at_edge`), the cursor otherwise (incl. live catch-up) — so a replay always judges liveness as-of-the-playhead (its timestamps are in the past, unrelated to wall time). See Status rules.
- **Seek / scrub** (`App::seek`, `seek_to_fraction`, `seek_prompt`, `go_live`): forward → fold in place (cheap); backward → `App::rebuild_to` re-folds the prefix into a fresh `SessionModel` and re-syncs, carrying view across by id (`graph::restore_positions` + `select_node`). A seek is discontinuous → the camera glide cancels and no status flash is set; in-flight tools need no reset because every frame derives them from the model (see [`ARCHITECTURE.md`](ARCHITECTURE.md) §5). `space` is a unified play/pause that resumes from the current cursor; `End`/`go_live` re-pins to the edge.
- **Position is event-indexed, not time-linear** — real sessions cluster work then sit idle (the rwy sample: ~11 min across 10.65 h), so a time-linear fraction would bury all action in a sliver. `fold_at_fraction` maps a fraction over `[floor, len]` where `floor` is the unavoidable start clump (same-timestamp ties + dated metadata that can only fold atomically), so fraction 0 reaches the start; `App::seek_to_fraction` seeks by it. The Lanes view draws time, not event index: it folds idle stretches itself (`┆12m┆`, `z`).
- **Emergent transport** (`App::transport_at` → Live / Playing / Paused / History / Idle): "Live" = following the edge **and** a fresh append (`last_batch_at` within ~10s), so a resumed *replay* reads Live and an old followed session reads Idle. Drives the top bar's transport badge — never a hardcoded mode.

## Event loop — native (tui.rs)

The native terminal loop draws only when `RedrawGate` says the frame is stale: an event arrived, the app is animating (a playing playhead, a camera glide on the graph), or a part of the picture that changes with time alone moved on. `FrameStamp` captures exactly those parts: the transport badge, the view, the marching-ants phase (graph view only, while an edge is animated and appends are recent), the summed whole seconds every in-flight tool has run, the count of tools past `SLOW_TOOL`, the elapsed-time seconds, the Lanes live-edge column, whether a snackbar is up, and whether a flash is up. Two equal stamps paint the same cells, so the frame is skipped. The loop ticks at 16 ms (`TICK`) only while the picture moves by itself (`App::in_motion`: playback, a glide, ants, a flash, a drag) and at 200 ms (`IDLE_TICK`) otherwise, when only whole-second timers and multi-second deadlines can change; input and tailer events wake it without waiting for a tick. The theme is detected once, before raw mode, and the terminal's reply is drained from the input (`drain_input`) so it never reaches the keymap. The portable core is shared (`lib.rs`); only the loop and IO are native.

```
theme = Theme::detect()                          // before raw mode: the OSC 11 reply must not reach the keymap
ratatui::init() → drain_input() → execute!(EnableMouseCapture)
spawn crossterm EventStream reader → unbounded mpsc
loop {
    app.tick_clock(now);                           // App.clock: the wall-time UI deadlines (snackbar, flash)
    app.tick_auto_pan(elapsed);                    // rataflow auto-pan; `panning` keeps the fast cadence
    app.tick_animation(elapsed);                   // marching-ant phase
    app.tick_camera(elapsed);                      // ease the Follow CameraGlide (GLIDE_SECS = 0.3)
    app.tick_timeline(elapsed);                    // advance the replay playhead + fold due items
    // ≥ STATUS_TICK (1 s): app.status_tick() re-derives interactive liveness for a quiet session
    if app.reconcile_folds() { gate.mark() }       // re-sync the graph when the done-subagent folds changed, whatever changed them
    if gate.due(&app, now, wall, panning) { terminal.draw(|f| ui::draw(f, &mut app, &theme))?; }
    let cadence = if panning || app.in_motion(now) { TICK /*16 ms*/ } else { IDLE_TICK /*200 ms*/ };
    tokio::select! {
        _ = sleep_until(now + cadence) => {}
        Some(ev) = ui_rx.recv() => { app.handle_ui_event(ev); gate.mark() }
        Some(ev) = event_rx.recv() => { gate.mark(); if handler::handle_event(&ev, app) { break } }
    }
    while let Ok(ev) = event_rx.try_recv() { ... }  // drain → no lag
    while let Ok(ev) = ui_rx.try_recv() { ... }
}
execute!(DisableMouseCapture); ratatui::restore()
```
The crossterm input channel is **unbounded** (input must never block); the **cap-32 bounded** channels (`CHANNEL_CAP`, `main.rs`) are the tailer-request + UI-event channels — they backpressure on `send().await`, and the tailer batches per tick so this is fine. Panic hook: `ratatui::init` installs screen restore but NOT mouse-capture disable — a custom hook layer also disables mouse capture.

## Graph sync (state/graph.rs — rwy's incremental pattern)

- **Never rebuild — except a backward seek.** Forward (live/replay playback): `flow.node_content_mut(id)` → mutate in place; else `flow.add_node(...)` + `flow.add_edge(...)` (duplicate-id `Err` is an idempotent no-op; add nodes before edges). The ONE exception is scrubbing into the past: folding is forward-only, so `App::rebuild_to` builds a fresh `SessionModel` + `Flow` from the prefix and `graph::restore_positions` carries node positions across by id (selection too) so the layout doesn't jump.
- Node: `Node::new(id, (0.0, 0.0), (W, H), AgentNode{…})` — fixed dims ~`(30.0, 7.0)` main/workflow, ~`(26.0, 6.0)` subagents (explicit dims; no DOM-style measuring). Handles: `Handle::source(HandlePosition::Bottom).with_hidden(true)`, `Handle::target(HandlePosition::Top).with_hidden(true)` (clean look, rwy does this).
- **Layout (strictly user-driven):** `sync` never auto-relayouts (it retains a `relayout` param, but every caller passes `false`). A Sugiyama pass on every new node reflowed the whole graph and read as "jumpy" as a session grew (confirmed by toggling it off), so newcomers always get local placement (below parent, fanned past siblings) and nothing existing ever moves on its own. The ONLY relayout trigger is `r` (`App::relayout_now` → `flow.apply_layout(Sugiyama::vertical())` + reframe for the current camera). Layout is orthogonal to the camera: `o`/`f` move the viewport only and never rearrange nodes. `layout_dirty` (set on structural growth, cleared by `r`) records that the layout is stale; no indicator is drawn for it. (Earlier designs auto-relayouted, then auto-relayouted except in Manual, then applied debt on `o`/`f`; all superseded — layout is now always explicit.)
- **Camera** (supersedes the original "fit only on first populate" — a one-shot fit goes stale as the graph grows): three mutually exclusive modes on `App.camera`. **Overview** (default) re-requests fit-view on every structural change — the camera pulls back as the swarm grows. **Follow** holds readable zoom (≥ `FOLLOW_ZOOM`) and centers on the most recently active agent (`SessionModel::last_active_agent_id`, latest `last_ts`, spawn-order tie-break). **Manual**: a uniform rule in `process_flow_events`, not per-event — every `FlowEvent` there is a user gesture (programmatic `select_node`/`center_on`/`set_offset` are quiet and never surface), so **Follow yields to ANY interaction**: click, spatial nav (`SelectionChanged`), pan/zoom (`ViewportChanged`), or node drag (`NodeDragged`). **Overview** yields only to a viewport change — selecting/dragging while auto-framing is fine to leave in Overview. Dropping Follow on selection is deliberate: the user is inspecting a node, and spatial nav already pans to keep it visible (rataflow's `ensure_selected_node_visible`, 1-cell margin) — staying in Follow would fight that and glide back over the selection. Keys name destinations: `o` → Overview, `f` → Follow — the only exits from Manual. Session reset → Overview. Follow auto-narrates the selection (quiet `select_node`, no event); a user selection ends that by dropping to Manual. Camera moves in Follow are eased (`CameraGlide`), cancelled the instant the user takes over.
- **Semantic zoom:** `AgentNode` renders at two levels chosen by on-screen size. **Card** (default): priority-ordered lines (title → description → tools → status) with ellipsis overflow — degrades continuously. **Cell** (below `CELL_MIN_WIDTH`/`CELL_MIN_HEIGHT`, ~zoom 0.5): solid status-toned fill (brand when selected) with the status glyph in its first column, no border or other text — a zoomed-out swarm reads as a field of status cells. `AgentEdge` (wraps `StepEdge`) carries no label at any zoom. Swarm view = cells + animated edges only.
- `flow.set_edge_animated(edge_id, running)` on status change; node card colors react to status via content mutation.
- Flow config: `Flow::new().with_deselect_on_pane_click(false)` + `flow.deselect_on_drag = false` (the selection persists), `with_min_zoom(0.1)` (Sugiyama trees outgrow default 0.5 fit-view limit).
- Selection survives sync because node ids are stable (`main`, agentId, wf-id) and we never clear()+re-add.
- **Folds** (`state::view::fold_set` → `FoldSet { by_parent, member_of }`, the single owner): two or more subagents of one parent that are `Done` for good — `is_terminal()`, or the replay has ended and sits at its end (`App::settled`, derived each time from `Timeline::ended`/`at_edge`, never latched) — fold into one. A live agent judged done only by quiet stays out (it can resume). A parent in `App.expanded_folds` stays unfolded; the selection plays no part, so every view agrees. Folds form in `tree_order`, so a hidden agent's subtree is hidden with it and starts no fold of its own. `view::rows` turns the tree into the Now view's rows (`Row::Agent` / `Row::Folded` at its first member's place); `sync` takes the same `FoldSet`: folded agents leave the canvas through `retain_nodes` (rataflow's Sugiyama ignores `hidden`, so hiding would leave holes), their positions go to `App.parked_positions` and come back on unfold, and each fold gets one card (`"{parent}\u{1f}done"`, edge `e-{card}`) at its first member's parked place — or, for a fold formed before any member reached the canvas (a live session's first read), where that member's local placement would have put it — titled `N done` with the members named and their calls and tokens summed. A fold moves no card (layout stays on `r`); Overview re-frames, a seek does not. Every path that changes the inputs either re-syncs (`resync`, `status_tick`, the replay's end) or is caught by `App::reconcile_folds` once per loop turn, which compares with `synced_folds`. A re-sync records the folds in `synced_folds` itself, so a path that re-syncs requests its own Overview fit (the replay's end does) — `reconcile_folds` will not see the change. A session reset clears the expansions, the parked positions and a selected fold.

## UI

- **Shell** (`ui::draw`, ui/mod.rs): a one-row top bar, the body, a one-row hint bar. Overlays and the snackbar draw last. Below `MIN_WIDTH` × `MIN_HEIGHT` (44 × 10) the screen says "Enlarge the window" and nothing else. Every colour comes through `Theme`; every time comes from `App` state (`now_reference`, `App.clock`), so a frame is a function of `App` and `Theme`.
- **Views** (`state::View`: `Now` | `Lanes` | `Graph`, keys `1` `2` `3`, `tab` cycles; the app opens on `Graph`, since the Herdr key launches it with no view argument): all three share one `Selection { agent, folded, expanded, detail, scroll, follow }`, so the selected agent survives a view switch. `folded` names a selected fold by its parent and is never set with `agent`; in Now and Graph a selected agent that a fold hides becomes its fold, its row expansion and full-size detail closed (`App::normalize_fold_selection`, on entering the view and before every frame), while the lanes keep the agent. `App.hit` (`HitMap`: tab rects, agent rows, the lane axis, the graph canvas) is rewritten by every draw and read by the mouse handler; a fold's row registers under `view::fold_row_key`.
- **Now view** (ui/views/now.rs): one row per agent in tree order — status glyph, name, what it is doing (the running tool's stated intent, else its task, else its latest thought), and on the right the in-flight tool with its timer (derived from the model each frame; see [`ARCHITECTURE.md`](ARCHITECTURE.md) §5) or the status word and timing, and output tokens. Up to two failed agents rise to `Callout`s on top. `enter` expands the selected row into its last five tool calls. From `SIDE_DETAIL_WIDTH` (100) columns the selected agent's detail stays open on the right (40 % of the body). Below `NARROW` (60) columns the tool chip and the token count are hidden. `Skeleton` fills the view while the timeline is empty.
  - A fold is one row: `✓ N done`, the members' names, their tokens summed. Selected from 100 columns, the side detail lists its members (status, name, timing, tokens). `j`/`k` walk `view::rows` here and on the graph; on the lanes they walk every agent, starting from a selected fold's first member.
- **Lanes view** (ui/views/lanes.rs): one row per agent, time across; a tool call is a bar over the columns it covers, coloured by `tool_tone` of its state, and a failed call's bar ends in `✗`. A call counts as running (extends to the edge with `▮`) under the same rule as `SessionModel::in_flight`: pending, and its agent not terminal. Idle stretches nobody worked through fold into a three-column `┆12m┆` marker (`fold_gaps`, `z`); the axis carries time labels. `←` `→` move a cursor by whole columns (`LaneAxis` records the time at each column's left edge, so folded gaps are skipped), and a click on the axis seeks there. While the playhead follows the edge the lane edge advances with the clock.
- **Detail** (ui/views/detail.rs): one agent in full — name, status badge, model, timing, tokens and, only when recorded, cost; its description; why it exists (the prompt that led to its spawn, the first prompt for the main agent, then the reasoning right before the spawn); then a scrollable list of every tool call (state glyph, name, intent or summary, clock time flush right), with an era header wherever the calls cross a prompt. The list tails the newest call (`Selection.follow`) until scrolled up. Data comes from `SessionModel`, keyed by the selected id; `Selection.detail` opens it full-size.
- **Graph view** (ui/views/graph.rs): rataflow's canvas and `Background`, and our own minimap (ui/views/minimap.rs). The view rebuilds the rataflow palette from `Theme` each frame; cards and edges recover the theme from it (`Theme::from_palette`). `AgentNode` and `AgentEdge` are plain data in `state/graph.rs`; their `NodeContent` / `EdgeContent` renderers live in this file.
  - **Minimap**: a 26 × 10 bordered box titled ` map ` in the canvas's top-right corner, drawn only on a canvas of at least 60 × 20. Its scale comes from the node bounds alone, one scale for both axes, centered — rataflow's `MiniMap` also fitted the viewport in, so zooming out or panning rescaled it and cards ran together or slid off its edge. Cards are whole-cell blocks (the selected one in brand, the rest `stroke.neutral-solid`); the part on screen is a `┌─┐│└┘` frame clipped to the map, drawn under the cards. The `?` help names it.
  - **Agent card**: rounded border (brand when selected, `stroke.neutral-weak` otherwise) around the surface; title (glyph + agent type, else the kind's default label), description, `1 tool` / `N tools · last_tool` (`text::fmt_tool_count`, shared with the detail), and a footer with the status word in its tone plus `N tok`. **Five status glyphs** (single source `AgentStatus::glyph`, word `status_word`, colour `status_tone`): `●` running, `◌` idle, `✓` done, `✗` failed, `■` stopped; a running `●` breathes to `○` on the animation clock. Every status is a glyph, a word and a colour together.
  - **Edge**: a step edge from parent to agent with no label; the edge of a running child takes the informative stroke and animates (marching ants).
- **Top bar** (`chrome::top_bar`): provider · session title, the transport badge (`◉ live` / `▶ replay n/N · time` / `⏸ paused · time` / `⏮ past · time` / `◦ quiet`), the session's elapsed time, output tokens, recorded cost (omp), and the view tabs. The badge's words and glyphs are never an agent status's (`● ◌ ✓ ✗ ■`, `active`/`idle`/`stopped`), so a quiet session beside an `● active` agent is not a contradiction. Narrow bars drop the least important segments first (title, cost, tokens); the tabs register their rects in `HitMap`.
- **Hint bar** (`chrome::bottom_bar`): on the left what needs attention — failed agents, tools in flight for more than `SLOW_TOOL` (30 s), the last tailer error — on the right the keys that work in the current view (`key_hints`), ending in `? help`.
- **Overlays**: `?` help (every key) and `i` session info (the untimed `SessionInfo`: mode, permission, last prompt, queued/file-edit counts) — both centered floating boxes, `esc` closes. A **snackbar** (`Snackbar`, `Snack { message, until }`) shows a short notice for `SNACK_FOR` (3 s), for example "Back to live". A **flash** (`Flash { ids, until }`, `FLASH_FOR` 150 ms) emphasises agents whose status just changed; a seek sets none.
- **Design tokens**: colours and radii come from SEED Design's rootage tokens (Apache-2.0), pinned in `design/seed/` (`design/seed/SOURCE.md` names the commit; `scripts/sync-seed.sh <commit>` re-pins) and compiled into the generated `src/ui/seed/tokens.rs`. The generator (`src/ui/seed/gen.rs`) is a test: `seed_tokens_match_vendored_yaml` compares its output with the committed file, and `UPDATE_SEED=1` rewrites it. The brand role is remapped from SEED's carrot to the same steps of its purple palette, and alpha tokens are composited to opaque; both are announced in the generated header. Colours reach the screen only through `Theme` (`src/ui/seed/theme.rs`: mode light/dark × depth truecolor/256, resolved once by `Theme::detect` before the terminal enters raw mode); nothing outside `src/ui/seed/` names a `Color::` value. SEED components (`Badge`, `Callout`, `Chip`, `KeyHint`, `ListItem`, `Skeleton`, `Snackbar`, `TabList`, …) live in `src/ui/seed/widgets/`. Snapshot goldens (`src/ui/snapshots.rs`) record each view in both themes at 80×24 and 120×40 as `assets/ui/<view>-<theme>-<WxH>.txt`: the frame's text plus its style runs named by token (`seed::snapshot`), so a retuned token value does not churn them but a cell painted with the wrong role, or with no token at all, fails. `UPDATE_GOLDEN=1` regenerates them.
- **Companions**: `Background::new(&flow)` then `&mut flow`, then the minimap over the canvas (render order matters; Widget impl is on `&mut Flow`, companions take `&Flow` — separate calls avoid borrow conflicts).
- Keys (handler.rs, one keymap for every view; the help overlay and the README list the same table): `q`/`ctrl-c` quit; `tab`/`shift-tab`/`1` `2` `3` switch view; `j`/`k`/`↓`/`↑` move the shared selection (scroll inside a full-size detail); `enter` expands the row, then opens its detail; `esc` goes back (close detail, collapse, deselect, close overlay); `space` play/pause (resume from cursor); `[`/`]` step prompt eras; `End`/`g`/`G` go live; `s` toggle gap-compression (faithful vs skip-idle pacing); `?`/`i` overlays; `pgup`/`pgdn` scroll the detail's tool list. View-specific: Lanes `←`/`→` move the cursor and `z` folds idle gaps; Graph `←`/`→` select sideways, `h` `l` `H` `J` `K` `L` pan, `+` `-` `0` zoom. `o`/`f` camera Overview/Follow and `r` relayout apply to the graph. Only navigation and viewport keys reach `flow.handle_key_event` (a whitelist — the graph is read-only, destructive library bindings are blocked). Mouse: a tab click switches view, a row click selects, the lane axis seeks, and on the graph canvas everything goes to `flow.handle_mouse_event`. Consume `into_events()`; any flow event drops Follow (`process_flow_events`).
- Folds on the keys: `enter` on a selected fold unfolds it and selects its first member (before expanding a row); `esc` on an agent of an unfolded fold folds back the fold it is a direct member of (`FoldSet::by_parent`; `member_of` also holds descendants, so with nested folds unfolded it would match an ancestor's) and selects the fold (after collapse, before deselect). A click on a fold's row or card selects the fold.

## inspect subcommand

`agents-graph inspect <file|id>`: open the session (`provider::open`, any provider, any file of it, or an id), fold every file, and print: session title, **session info** (the provider's labelled rows: mode · permission · queued · file edits · last prompt for Claude; app · version · cwd for Codex), agent/tool totals, then the agent tree (type, description, status, #tools, tokens). Exit non-zero on an unreadable or unrecognised file. **This is the headless smoke test** — CI-runnable end-to-end check of parser + session model + info extraction with no TTY.

## Testing (inline #[cfg(test)], no tests/ dir)

Worth testing: transcript line parsing against real-format fixture strings (every entry type incl. flat metadata, polymorphic content, missing is_error, Unknown), sanitization rule, partial-line buffering + truncation reset (tailer state machine over an in-memory/tempfile sequence), session model status transitions (spawn → running → done/failed; the async layer — spawn-ack supersession, `<task-notification>` terminal report, `end_of_stream`; workflow journal completion; pending-tool liveness), graph sync idempotency (same update twice = no duplicate nodes; selection preserved), the in-flight derivation (`SessionModel::in_flight`), the frame stamp (`RedrawGate`), and the rendered views (snapshot goldens in `assets/ui`). Not worth testing: render output, getters.

**Order-independence is guarded by property tests** (the load-bearing invariant — ARCHITECTURE.md §1.1): `live_delivery_converges_to_bulk_ordering` (timeline.rs — 400 random per-file interleavings land the same ts sequence as the bulk sort, nothing left undated) and the model shuffle-invariance test (session.rs — final model state is a pure function of the fact set). the test suite, all inline; no `tests/` dir.

## Pitfalls checklist (from research — verify before calling done)

- [ ] `rataflow::Error` (no FlowError); `add_edge_from_connection(conn, content)` two args (unused in v1 — read-only graph)
- [ ] Draw only when the frame is stale (`RedrawGate`); post-select try_recv drains; unbounded crossterm channel
- [ ] `tick_animation` wired (rwy reference loop lacks it)
- [ ] Partial trailing line buffered; `len < offset` → reset + SessionReset
- [ ] `parentUuid` present-and-null (root) vs absent (metadata) — Option handling, lean variants for flat types
- [ ] `is_error` missing = success
- [ ] user content + tool_result content polymorphic string|array
- [ ] Only `<uuid>.jsonl` in project dir + `subagents/**/agent-*.jsonl`; never skill-injections.jsonl/journal as transcript
- [ ] camera modes per the Camera section (Overview auto-fit / Follow tracking / Manual); min_zoom raised
- [ ] No network deps anywhere in the tree
- [ ] First render has zero canvas size — `request_fit_view` (deferred) not `fit_view`
