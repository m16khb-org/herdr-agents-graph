//! `agents-graph herdr resolve|toggle`: the herdr plugin bridge.
//!
//! Every JSON exchange with herdr happens here, so the plugin's shell scripts
//! are one `exec` each and need nothing beyond bash.
//!
//! - `resolve` prints `<provider> <id-or-path>` for the pane the plugin was
//!   invoked for. That pane is `focused_pane_id` in the action's
//!   `HERDR_PLUGIN_CONTEXT_JSON`, never `HERDR_PANE_ID` (inside a pane
//!   command that is the plugin's own new pane). herdr 0.9.3 reports Claude
//!   Code and Codex sessions as `{kind: "id"}` and omp sessions as
//!   `{kind: "path"}`.
//! - `toggle <placement>` opens the graph pane, or closes the one it opened
//!   last. The open pane's id is kept in `HERDR_PLUGIN_STATE_DIR/open-pane`,
//!   so the second press works wherever focus is. A pane command's own
//!   context names whatever is focused when the pane starts, which need not
//!   be the pane the action was invoked for, so toggle hands that pane to the
//!   new pane as [`TARGET_PANE_ENV`] and `resolve` reads it first.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;

/// Exit status for "this pane has nothing this plugin can draw": the pane
/// script shows the message and waits for enter instead of drawing.
const EXIT_UNRESOLVED: i32 = 2;

const PLACEMENTS: [&str; 4] = ["overlay", "split", "tab", "zoomed"];

/// Set by `toggle` on the graph pane it opens: the pane to draw.
const TARGET_PANE_ENV: &str = "AGENTS_GRAPH_PANE";

pub fn run(mut args: impl Iterator<Item = String>) -> Result<()> {
    match args.next().as_deref() {
        Some("resolve") => match resolve() {
            Ok((provider, value)) => {
                println!("{provider} {value}");
                Ok(())
            }
            Err(message) => {
                eprintln!("{message}");
                std::process::exit(EXIT_UNRESOLVED);
            }
        },
        Some("toggle") => {
            let placement = args.next().unwrap_or_else(|| "overlay".to_string());
            if !PLACEMENTS.contains(&placement.as_str()) {
                bail!("placement must be one of {}", PLACEMENTS.join(", "));
            }
            toggle(&placement)
        }
        _ => bail!(
            "usage: agents-graph herdr resolve | agents-graph herdr toggle [overlay|split|tab|zoomed]"
        ),
    }
}

fn herdr_bin() -> String {
    std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string())
}

/// Run `herdr <args>` and parse its stdout as JSON. An `error` object in the
/// answer is an error here too.
fn herdr_json(args: &[&str]) -> Result<Value> {
    let out = Command::new(herdr_bin())
        .args(args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("running herdr {}", args.join(" ")))?;
    if !out.status.success() {
        bail!(
            "herdr {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let v: Value = serde_json::from_slice(&out.stdout)
        .with_context(|| format!("herdr {} did not answer JSON", args.join(" ")))?;
    if let Some(message) = v.pointer("/error/message").and_then(Value::as_str) {
        bail!("herdr: {message}");
    }
    Ok(v)
}

// ---------------------------------------------------------------------------
// resolve
// ---------------------------------------------------------------------------

fn resolve() -> std::result::Result<(&'static str, String), String> {
    let pane_id = target_pane(
        std::env::var(TARGET_PANE_ENV).ok(),
        &std::env::var("HERDR_PLUGIN_CONTEXT_JSON").unwrap_or_default(),
    )
    .ok_or("no focused pane in the invocation context (HERDR_PLUGIN_CONTEXT_JSON)")?;
    let answer = herdr_json(&["pane", "get", &pane_id]).map_err(|e| format!("{e:#}"))?;
    let pane = answer
        .pointer("/result/pane")
        .or_else(|| answer.get("result"))
        .ok_or_else(|| format!("herdr pane get {pane_id} answered without a pane"))?;
    let home = std::env::var_os("HOME").map(PathBuf::from);
    target_of(&pane_id, pane, home.as_deref(), &|p| p.is_file())
}

/// What a `pane.get` record says to draw: the provider name `--provider`
/// takes and the session id or transcript path. `Err` is the message the
/// pane shows instead.
fn target_of(
    pane_id: &str,
    pane: &Value,
    home: Option<&Path>,
    exists: &dyn Fn(&Path) -> bool,
) -> std::result::Result<(&'static str, String), String> {
    let session = pane.get("agent_session").filter(|s| !s.is_null());
    let field = |key: &str| {
        session
            .and_then(|s| s.get(key))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
    };
    let agent = field("agent").or_else(|| pane.get("agent").and_then(Value::as_str));
    let (provider, expected) = match agent {
        Some("claude") => ("claude", "id"),
        Some("codex") => ("codex", "id"),
        Some("omp") => ("omp", "path"),
        Some(other) if !other.is_empty() => {
            return Err(format!(
                "agent '{other}' in pane {pane_id} is not one this plugin reads (Claude Code, Codex, omp)"
            ));
        }
        _ => {
            return Err(format!(
                "pane {pane_id} has no agent: focus a Claude Code, Codex, or omp pane"
            ));
        }
    };
    let what = if expected == "id" {
        "session id"
    } else {
        "session path"
    };
    let (Some(kind), Some(value)) = (field("kind"), field("value")) else {
        return Err(format!(
            "Herdr has no {what} for this {provider} pane.\n\n  \
             The agent's herdr integration reports it when a session starts. Install it\n  \
             if it is missing, then start the agent in that pane again:\n\n    \
             herdr integration install {provider}"
        ));
    };
    if kind != expected {
        return Err(format!(
            "Herdr reports a {kind} for this {provider} pane; this plugin reads a {what}"
        ));
    }
    if expected == "id" {
        return Ok((provider, value.to_string()));
    }
    let path = match (value.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(value),
    };
    if !exists(&path) {
        return Err(format!(
            "Herdr reports {} for this {provider} pane, but that file does not exist (rotated, or deleted since?)",
            path.display()
        ));
    }
    Ok((provider, path.to_string_lossy().into_owned()))
}

/// The pane to draw: the one `toggle` named, else the invocation context's
/// `focused_pane_id`.
fn target_pane(named: Option<String>, context: &str) -> Option<String> {
    named.filter(|id| !id.is_empty()).or_else(|| {
        serde_json::from_str::<Value>(context)
            .ok()?
            .get("focused_pane_id")?
            .as_str()
            .filter(|id| !id.is_empty())
            .map(str::to_string)
    })
}

// ---------------------------------------------------------------------------
// toggle
// ---------------------------------------------------------------------------

fn toggle(placement: &str) -> Result<()> {
    let state_dir = std::env::var_os("HERDR_PLUGIN_STATE_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| {
            anyhow!("HERDR_PLUGIN_STATE_DIR is not set; run this from a herdr action")
        })?;
    let record = state_dir.join("open-pane");

    if let Some(open) = std::fs::read_to_string(&record)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        // A pane closed by hand leaves its id behind; `pane get` failing is
        // how that shows, and the press then opens a new one.
        let still_open = herdr_json(&["pane", "get", &open]).is_ok();
        if still_open {
            // Forget the pane only once it is closed: a failed close keeps the
            // record, so the next press can still reach that pane.
            herdr_json(&["plugin", "pane", "close", &open])?;
        }
        let _ = std::fs::remove_file(&record);
        if still_open {
            return Ok(());
        }
    }

    let plugin = std::env::var("HERDR_PLUGIN_ID")
        .unwrap_or_else(|_| "m16khb.herdr-agents-graph".to_string());
    let mut open = vec![
        "plugin".to_string(),
        "pane".into(),
        "open".into(),
        "--plugin".into(),
        plugin,
        "--entrypoint".into(),
        "graph".into(),
        "--placement".into(),
        placement.into(),
    ];
    let context = std::env::var("HERDR_PLUGIN_CONTEXT_JSON").unwrap_or_default();
    if let Some(target) = target_pane(None, &context) {
        open.extend(["--env".into(), format!("{TARGET_PANE_ENV}={target}")]);
        // A split sits beside the pane it draws. herdr places overlays (and
        // the other placements) on the active pane and rejects a target there.
        if placement == "split" {
            open.extend(["--target-pane".into(), target]);
        }
    }
    let answer = herdr_json(&open.iter().map(String::as_str).collect::<Vec<_>>())?;
    let pane_id = find_pane_id(&answer)
        .ok_or_else(|| anyhow!("herdr plugin pane open answered without a pane id: {answer}"))?;
    std::fs::create_dir_all(&state_dir)
        .with_context(|| format!("creating {}", state_dir.display()))?;
    std::fs::write(&record, format!("{pane_id}\n"))
        .with_context(|| format!("writing {}", record.display()))?;
    Ok(())
}

/// The first `pane_id` string anywhere in a herdr answer.
fn find_pane_id(v: &Value) -> Option<&str> {
    match v {
        Value::Object(map) => map
            .get("pane_id")
            .and_then(Value::as_str)
            .or_else(|| map.values().find_map(find_pane_id)),
        Value::Array(items) => items.iter().find_map(find_pane_id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn resolve_pane(pane: Value) -> std::result::Result<(&'static str, String), String> {
        target_of("w1:p1", &pane, Some(Path::new("/home/me")), &|p| {
            p == Path::new("/home/me/.omp/agent/sessions/-x/a.jsonl")
        })
    }

    #[test]
    fn resolve_accepts_only_known_kinds() {
        let session = |agent: &str, kind: &str, value: &str| json!({"agent": agent, "agent_session": {"source": format!("herdr:{agent}"), "agent": agent, "kind": kind, "value": value}});
        assert_eq!(
            resolve_pane(session("claude", "id", "5f78")),
            Ok(("claude", "5f78".to_string()))
        );
        assert_eq!(
            resolve_pane(session("codex", "id", "01a0")),
            Ok(("codex", "01a0".to_string()))
        );
        assert_eq!(
            resolve_pane(session(
                "omp",
                "path",
                "/home/me/.omp/agent/sessions/-x/a.jsonl"
            )),
            Ok(("omp", "/home/me/.omp/agent/sessions/-x/a.jsonl".to_string()))
        );
        // A home-relative path is expanded before it is checked.
        assert_eq!(
            resolve_pane(session("omp", "path", "~/.omp/agent/sessions/-x/a.jsonl")),
            Ok(("omp", "/home/me/.omp/agent/sessions/-x/a.jsonl".to_string()))
        );

        // The wrong kind for the agent, a missing file, an agent this plugin
        // does not read, and a pane with no agent are all refused.
        assert!(resolve_pane(session("claude", "path", "/tmp/x.jsonl")).is_err());
        assert!(resolve_pane(session("omp", "id", "abc")).is_err());
        assert!(
            resolve_pane(session("omp", "path", "/gone.jsonl"))
                .unwrap_err()
                .contains("does not exist")
        );
        assert!(
            resolve_pane(session("pi", "path", "/x.jsonl"))
                .unwrap_err()
                .contains("not one this plugin reads")
        );
        assert!(
            resolve_pane(json!({"agent": null, "agent_session": null}))
                .unwrap_err()
                .contains("has no agent")
        );
    }

    /// An omp pane started before its integration was installed has an agent
    /// but no session; the message names the integration to install.
    #[test]
    fn resolve_without_a_session_names_the_integration() {
        let err = resolve_pane(json!({"agent": "omp", "agent_session": null})).unwrap_err();
        assert!(err.contains("Herdr has no session path for this omp pane"));
        assert!(err.contains("herdr integration install omp"));
    }

    #[test]
    fn pane_id_is_found_wherever_herdr_nests_it() {
        assert_eq!(
            find_pane_id(
                &json!({"id": "r1", "result": {"type": "pane_info", "pane": {"pane_id": "w1:p9"}}})
            ),
            Some("w1:p9")
        );
        assert_eq!(find_pane_id(&json!({"result": {}})), None);
    }

    /// The graph pane draws the pane its action was invoked for, not whatever
    /// is focused by the time the pane starts.
    #[test]
    fn the_named_pane_outranks_the_pane_commands_own_focus() {
        let focus = r#"{"focused_pane_id":"w1:p1"}"#;
        assert_eq!(
            target_pane(Some("w2:p7".into()), focus).as_deref(),
            Some("w2:p7")
        );
        assert_eq!(
            target_pane(Some(String::new()), focus).as_deref(),
            Some("w1:p1")
        );
        assert_eq!(target_pane(None, focus).as_deref(), Some("w1:p1"));
        assert_eq!(target_pane(None, "{}"), None);
        assert_eq!(target_pane(None, ""), None);
    }
}
