# agents-graph in Herdr

[Herdr](https://herdr.dev) runs coding agents in panes and knows which session
each pane is running. This plugin asks it, then opens that session in `agents-graph`
as a live flow graph, without you naming a file or an id. Claude Code, Codex
and omp panes all work.

## Installation

```bash
herdr integration install claude          # and/or: codex, omp
herdr plugin install m16khb-org/herdr-agents-graph/herdr-plugin
herdr plugin action invoke setup-keys --plugin m16khb.herdr-agents-graph
```

**The integration** is what makes the rest possible. It adds a hook or
extension to the agent, so Herdr learns which session is running in a pane —
the native id for Claude Code and Codex, the transcript path for omp, since
its own extension already knows it. Without it Herdr knows a pane
holds an agent but not its session, and the plugin has nothing to open. An
agent already running when you install it never reports one, so start it
again. `herdr integration status` lists what is installed.

**The plugin install** runs a build step that downloads the `agents-graph`
release for this plugin's version (`v<version>`) and your platform, checks the
archive against the release's `SHA256SUMS`, and installs the binary inside the
plugin as `bin/agents-graph`. Nothing is compiled and nothing is put on
`PATH`. If the download or the checksum fails, nothing is installed.

**The key step** writes one marked block into your Herdr config, since plugins
cannot ship keys in their manifest. It backs the file up first, never takes a
key you already use, and reloads Herdr. `remove-keys` deletes the block again.

## Usage

Focus an agent pane and press `prefix+shift+z`, where `prefix` is `ctrl+b`
unless you changed it. The graph opens over the pane and follows the session
live. Press `q` in the graph to close it, or the same key again, so one key
both opens and puts away.

Where it opens is a preference you make once, so `setup-keys` binds one key and
writes the other two placements into the same block as comments:

| Placement | Bound to | Use it when |
| --- | --- | --- |
| overlay | `prefix+shift+z` | a glance. Takes the tab, restores your focus and zoom on close |
| split | commented | both on screen. Splits the agent pane, so the graph gets half of it |
| tab | commented | keeping it open. Full area, one tab switch from the agent |

To switch, uncomment one in your Herdr config, or point the live binding at
`open-split` or `open-tab`, then run `herdr server reload-config`. To use your
own keys instead, skip `setup-keys` and bind the actions yourself:

```toml
[[keys.command]]
key = "prefix+shift+z"
type = "plugin_action"
command = "m16khb.herdr-agents-graph.open"
description = "agents-graph: session graph (overlay)"
```

The actions also run from the command line, with or without a key:

```bash
herdr plugin action invoke open --plugin m16khb.herdr-agents-graph
herdr plugin action invoke open-split --plugin m16khb.herdr-agents-graph
herdr plugin action invoke open-tab --plugin m16khb.herdr-agents-graph
```

## When it cannot open a session

The pane says why and waits for you to press enter, rather than closing on you:

| Herdr reports | What you see |
| --- | --- |
| a session id for a `claude` or `codex` pane, or a path to an existing file for an `omp` pane | the graph |
| a path that no longer exists | a line saying the file is gone |
| an agent but no session yet | how to install the integration, and why the agent has to be restarted |
| no agent in the pane | a line saying to focus an agent pane |
| an agent agents-graph does not read | the same, naming the agent |

There is no fallback through the working directory. Herdr knows the session, so
the plugin uses it or says what it got instead.

## Keys inside the graph

Herdr sees the prefix chord first and everything else reaches the pane, in
every placement. So agents-graph keeps its own keys (`q`, `?`, `space`, arrows,
`hjkl`, `+` and `-`, `[` and `]`, `g` and `G`) and Herdr keeps `prefix+...`.
Press `?` in the graph for the full map.

---

How the bridge is built, why it is versioned the way it is, and the Herdr API
it reads: [`docs/HERDR-PLUGIN.md`](../docs/HERDR-PLUGIN.md).

## Uninstalling

```bash
herdr plugin action invoke remove-keys --plugin m16khb.herdr-agents-graph
herdr plugin uninstall m16khb.herdr-agents-graph
```
