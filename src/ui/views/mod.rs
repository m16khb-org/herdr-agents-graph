//! The body views and what they share: how an agent is named and badged,
//! and what counts as needing the user's attention.

pub(crate) mod detail;
pub(crate) mod graph;
pub(crate) mod lanes;
pub(crate) mod minimap;
pub(crate) mod now;

use chrono::{DateTime, Utc};

use crate::state::session::{SessionModel, ToolState};

/// One agent that needs looking at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Failure<'a> {
    pub id: &'a str,
    pub name: &'a str,
    /// What failed: the failing call (`bash · cargo test`) or the agent's
    /// own end.
    pub what: String,
}

/// What the screen calls out: failed agents (an agent whose status failed,
/// or whose latest finished call errored), and tools in flight past
/// [`crate::state::SLOW_TOOL`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Attention<'a> {
    pub failed: Vec<Failure<'a>>,
    pub slow: usize,
}

/// Gather the [`Attention`] items, in tree order.
pub(crate) fn attention(model: &SessionModel, wall: Option<DateTime<Utc>>) -> Attention<'_> {
    let mut out = Attention::default();
    for (id, _) in model.tree_order() {
        let Some(agent) = model.agent(id) else {
            continue;
        };
        let last_done = agent
            .tool_calls()
            .rev()
            .find(|c| c.state != ToolState::Pending);
        let failed_call = last_done.filter(|c| c.state == ToolState::Err);
        if agent.status == crate::state::session::AgentStatus::Failed || failed_call.is_some() {
            let what = match failed_call {
                Some(c) => match c.summary.as_deref().filter(|s| !s.is_empty()) {
                    Some(s) => format!("{} · {s}", c.name),
                    None => format!("{} failed", c.name),
                },
                None => agent.status_word().to_string(),
            };
            out.failed.push(Failure {
                id,
                name: agent.display_name(),
                what,
            });
        }
    }
    out.slow = wall.map_or(0, |w| model.slow_tools(w));
    out
}
