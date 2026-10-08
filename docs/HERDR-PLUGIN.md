# The Herdr plugin

A bridge, not a second frontend. [Herdr](https://herdr.dev) already knows which
agent occupies a pane and the native id of the session running there, which is
the pair `agents-graph` would otherwise have to infer. The plugin asks for it and hands
it over. The user-facing half is [`herdr-plugin/README.md`](../herdr-plugin/README.md);
this is the rest.

It lives in `herdr-plugin/`, ships by git clone rather than with the crate, and
is a manifest plus four shell scripts and a small `herdr` subcommand of the
binary. Herdr plugin panes run an ordinary argv command in a real TTY, so
`agents-graph` itself is the plugin UI. Nothing is embedded in
Herdr's render loop, and nothing about the graph is written twice.

---

## 1. Where it sits on the boundary

[DISCOVERY.md](DISCOVERY.md) fixed the input side: a provider states what a file
is, the core decides what a session is, and a feeder reaches it through `open`.
The plugin is the case `Target::Id` was written for: an agent's own
integration reports a session id, the file is not, and finding the file is
the core's job across every provider's roots. omp reports a path
instead, because their integration already knows the transcript file; that
lands in `Target::Path`, the case the plain CLI uses for an explicit file.
Either way the core already knows what to do with it, so the bridge never
special-cases the kind.

That is the whole reason the bridge stays small. It never reads a transcript,
never learns a layout, and never guesses a project from a working directory. It
resolves one pair, `(agent, session id or path)`, and spends it on one command:

```
agents-graph --provider <agent> --follow <id-or-path>
```

`--provider` narrows the lookup to the agent Herdr named. `--follow` is right
by definition here, since the pane's agent is running.

## 2. The scripts

| Script | Role |
|---|---|
| `herdr/pane.sh` | all three actions (`open`, `open-split`, `open-tab`), differing only in the placement they pass. It execs `agents-graph herdr toggle <placement>` |
| `herdr/open.sh` | the pane command. Runs `agents-graph herdr resolve`, then execs `agents-graph --provider P --follow V` |
| `herdr/keys.sh` | the `setup-keys` / `remove-keys` actions |
| `herdr/install.sh` | the `[[build]]` step: download and verify the release binary into `herdr-plugin/bin/` |

The JSON handling is not in shell. `src/herdr.rs` implements
`agents-graph herdr resolve` and `agents-graph herdr toggle`, so no script
parses it, and the binary is only ever run from `herdr-plugin/bin/`, never from
`PATH`.

Three decisions in there are worth keeping.

**Resolution reads `focused_pane_id`, never `HERDR_PANE_ID`.** In a pane
command, `HERDR_PANE_ID` is the plugin's own newly created pane, so asking Herdr
about it returns a pane with no agent. `focused_pane_id` in
`HERDR_PLUGIN_CONTEXT_JSON` is the pane the plugin was invoked from, and it is
the same field for an action and for a pane command.

**Every message is printed in the pane, and held until the user presses enter.**
An action runs headless with its output going to `herdr plugin log`, and Herdr
notifications can be switched off, in which case a notification is silently
dropped. The pane's own terminal is the only surface that cannot be turned off.
Resolution therefore happens in the pane, where its failures are visible, and
not in the action, where they would not be.

**Toggling is by pane id, not by label.** `toggle` writes the id of the pane it
opened to `$HERDR_PLUGIN_STATE_DIR/open-pane`. The next press reads it, and if
`herdr pane get` still finds that pane it closes it through
`herdr plugin pane close`. If the pane was closed by hand the id is stale, and
the press opens a new one. No label is compared, so focus can be anywhere and a
renamed pane cannot be mistaken for ours.

## 3. Keys

Plugin v1 declares actions, event hooks, panes and link handlers, and nothing
else. Keybindings live in the user's own config, and Herdr 0.8.2 has no command
palette, so an action with no key is reachable only from the command line. Of
the plugins surveyed, most document a snippet to paste and three ship an action
that writes it; `setup-keys` is the latter.

It resolves the config path the way Herdr does (`HERDR_CONFIG_PATH`, then
`$XDG_CONFIG_HOME/herdr/config.toml`, then `~/.config/herdr/config.toml`),
refuses to touch a file that does not already pass `herdr config check`, backs
it up, writes one marker-fenced block, validates the result and restores the
backup if it does not parse, then reloads the running Herdr.

It binds one key. Where the graph opens is a standing preference rather than a
per-press decision, so claiming three chords of a shared keyspace to express it
would be a poor trade. The other two placements are written into the same block
as commented bindings, which documents them where the user will look.

## 4. Versions

Two numbers, and the manifest version is tied to the crate:

| Number | Where | Bumped when |
|---|---|---|
| `version` | `herdr-plugin.toml` | every release. The build step downloads the release tagged `v<version>`, so it must equal the crate version (the release workflow refuses a tag that disagrees with either) |
| `min_herdr_version` | `herdr-plugin.toml` | a script starts using a newer Herdr API (now 0.9.3, which reports `agent_session` kinds and `focused_pane_id`). The only one that can block an install |

`herdr plugin install` clones the repo and then runs the build step, which
downloads `agents-graph-<target>.tar.gz` and `SHA256SUMS` from the GitHub release
for `v<version>`, checks the archive against the checksum, and installs the
binary as `herdr-plugin/bin/agents-graph`. It stops with nothing installed if
the platform has no release, the checksum has no entry, or it does not match.
`AG_RELEASE_BASE` overrides the download location. `herdr plugin link` runs no
build step, so a linked checkout needs `bin/agents-graph` put there by hand.

**Why the manifest version mirrors the crate.** The plugin is how the binary is
distributed: the version names the release to download. A version with no
release makes the build step fail, so the version is bumped with the crate and
never ahead of a published release.

**Why the binary is not on `PATH`.** It lives inside the plugin checkout, so it
cannot shadow or be shadowed by another tool, and uninstalling the plugin
removes it.

## 5. Notes on Herdr's API

Read from `herdr api schema --json` on Herdr 0.8.2 (protocol 20), the plugin and
socket docs at v0.9.0, and live responses.

- `PaneInfo` is `pane_id` (not `id`), `agent`, `agent_session`, `agent_status`,
  `cwd`, `foreground_cwd`, `display_agent`, `focused`, `tokens`, `workspace_id`.
- `pane.get` nests that record one level down, as
  `{"result": {"pane": {...}, "type": "pane_info"}}`; `pane.list` answers
  `.result.panes`. Every CLI response is `{id, result}` or `{id, error}`.
- `AgentSessionInfo` is `{source, agent, kind, value}`, all required, and the
  whole object is absent until an integration reports a session.
- `AgentSessionRefKind` is `"id"` or `"path"`. Herdr maps `herdr:claude` and
  `herdr:codex` to an id, reported by each agent's `SessionStart` hook;
  `herdr:omp` maps to a path instead, since its own
  integration extension already knows the transcript file and there is
  nothing left to look up. `agents-graph herdr resolve` accepts an id for `claude` and `codex` and a
  path for `omp`, checks that the omp file exists, and exits 2 with a readable
  message otherwise.
- `PluginInvocationContext` is flat: `focused_pane_id`, `focused_pane_agent`,
  `focused_pane_cwd`, `focused_pane_status`, `workspace_cwd`, and so on.
- Action `contexts` are `global`, `workspace`, `tab`, `pane`, `selection`,
  stored but not rendered in 0.8.2. Pane `placement` is `overlay`, `popup`,
  `split`, `tab` or `zoomed`; `overlay` is a temporary zoom that restores the
  previous focus and zoom on close, `popup` is session-modal, has no pane id and
  sits outside the pane APIs.
- Runtime environment: `HERDR_SOCKET_PATH`, `HERDR_BIN_PATH`, `HERDR_PLUGIN_ID`,
  `HERDR_PLUGIN_ROOT`, `HERDR_PLUGIN_CONFIG_DIR`, `HERDR_PLUGIN_STATE_DIR`,
  `HERDR_PLUGIN_CONTEXT_JSON`, plus `HERDR_WORKSPACE_ID`, `HERDR_TAB_ID`,
  `HERDR_PANE_ID` where they apply. Actions also get `HERDR_PLUGIN_ACTION_ID`,
  pane commands `HERDR_PLUGIN_ENTRYPOINT_ID`.

## 6. Working on it

```bash
herdr plugin link "$(pwd)/herdr-plugin"        # link runs no build step: put a binary at herdr-plugin/bin/agents-graph
herdr plugin action list --plugin m16khb.herdr-agents-graph
herdr plugin log list --plugin m16khb.herdr-agents-graph   # where an action's output goes
herdr pane list                                 # .result.panes[]: pane_id, agent, agent_session
```

`herdr plugin link` re-reads the manifest, so run it after editing
`herdr-plugin.toml`. `agents-graph herdr` reads Herdr only through `$HERDR_BIN_PATH`, so it can be
exercised without Herdr by putting a stub `herdr` there that prints a canned `pane.get` response and setting
`HERDR_PLUGIN_CONTEXT_JSON` to `{"focused_pane_id":"w1:p1"}`.

Anything asserted here about Herdr's shapes came from the schema or a live
response, never from memory. Keep it that way: the field names are close enough
to plausible-but-wrong that guessing has cost a debugging session already.
