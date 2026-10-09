//! Application state.
//!
//! [`App`] owns the rataflow `Flow` (the graph view's canvas), the pure
//! [`SessionModel`] (domain truth), and all view state (pause, mode, the
//! current session id, the camera, the active view and the shared selection).
//! [`App::handle_ui_event`] folds tailer events into the model and re-syncs the
//! graph; events for stale sessions are dropped via [`App::is_current`].

// Two projections of the model. Crate-private: what the frontends need from
// them is re-exported below, as the types of `App`'s fields.
mod frame;
pub(crate) mod graph;
pub mod info;
pub mod render;
pub mod session;
pub(crate) mod timeline;
pub mod view;

// `App`'s public fields, nameable from outside without exposing the module
// layout they live in.
pub use self::frame::{FrameStamp, RedrawGate};
pub use self::graph::AgentFlow;
pub use self::info::SessionInfo;
pub use self::session::SLOW_TOOL;
pub use self::timeline::Timeline;
pub use self::view::{Flash, HitMap, LaneAxis, Selection, Snack, View};

use self::session::SessionModel;
use self::view::{FoldSet, Row, fold_card_id, fold_of_card};
use crate::tailer::UiEvent;

/// Whether the app is watching a live session or replaying a finished one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Live mode: tail the latest session for a cwd.
    Live,
    /// Replay mode: timestamp-paced playback of a finished transcript.
    Replay,
}

/// Camera modes: who drives the viewport.
///
/// `o`/`f` name destinations, not toggles — any manual pan/zoom switches to
/// [`Manual`](Camera::Manual) from any mode, and the camera keys are the only
/// way back out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Camera {
    /// Auto-frame everything: re-fits on structural change, pulling back as
    /// the graph grows (default).
    Overview,
    /// Hold readable zoom and track the most recently active agent.
    Follow,
    /// The user has the camera; the app never moves it.
    Manual,
}

/// The zoom floor centering engages at (Follow tracking and click-to-center)
/// — cards must be readable.
pub const FOLLOW_ZOOM: f64 = 1.0;

/// Breathing room (terminal cells, per side) kept around a centered card when
/// clamping zoom so the card fits the canvas — see [`App::center_node`].
const NODE_FIT_PAD: f64 = 2.0;

/// Emergent transport state — derived, never stored as a mode. "Live" is not a
/// property of the file (completion is unknowable); it's "following the edge
/// while it still grows."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// Following the edge and appends are arriving right now.
    Live,
    /// Playing forward behind the edge — a replay, or a live session catching up.
    Playing,
    /// Playback halted (`space`) — in a replay or at a live edge.
    Paused,
    /// Parked in the past (scrubbed back off the edge).
    History,
    /// At the edge with no fresh activity — a finished/quiet session.
    Idle,
}

/// How recently an append must have arrived to count as "live".
const LIVE_FRESH: std::time::Duration = std::time::Duration::from_secs(10);

/// Duration of a Follow camera glide between focus targets: SEED's longest
/// motion step, `duration.d6` (300 ms).
const GLIDE_SECS: f64 = 0.3;

/// How long a snackbar notice stays: ten `duration.d6` steps.
pub const SNACK_FOR: std::time::Duration = std::time::Duration::from_secs(3);

/// How long a status change stays emphasised: SEED `duration.color-transition`.
pub const FLASH_FOR: std::time::Duration = std::time::Duration::from_millis(150);

/// Offset distance (world units) below which a glide is a no-op snap — avoids
/// micro-glides and lets a stationary followed agent settle exactly on target.
const GLIDE_SNAP_EPS: f64 = 0.5;

/// An in-progress eased pan of the viewport offset between two points (Follow).
///
/// Tweens the viewport `offset` so a Follow focus change glides rather than
/// teleports. Offsets (not world centers) are stored so the end state is
/// byte-exact with `center_on(target)` regardless of zoom — the destination is
/// captured by probing `center_on` once when the glide starts.
#[derive(Debug, Clone, Copy)]
pub struct CameraGlide {
    pub(crate) from: (f64, f64),
    pub(crate) to: (f64, f64),
    /// Progress in `0.0..=1.0`.
    pub(crate) t: f64,
}

impl CameraGlide {
    /// Eased offset at the current progress (smoothstep ease-in-out).
    fn offset(&self) -> (f64, f64) {
        let e = self.t * self.t * (3.0 - 2.0 * self.t);
        (
            self.from.0 + (self.to.0 - self.from.0) * e,
            self.from.1 + (self.to.1 - self.from.1) * e,
        )
    }
}

/// Items folded between two rungs of the snapshot ladder.
///
/// The trade is latency against memory: a backward seek re-folds at most this
/// many items, and the ladder holds `folded / STRIDE` rungs. Snapshots are
/// cheap because [`SessionModel`] is built from persistent collections — a rung
/// shares structure with its neighbours instead of copying the model.
///
/// Read the stride off `drag_10_back`, not off a single seek: it keeps improving
/// as the stride shrinks, while `back_to_50pct` and `back_one_hop` appear to
/// plateau only because one fixed target sits an arbitrary distance from its
/// rung. The drag averages over positions.
///
/// 1024 keeps a hop under a millisecond even on the pathological bench scale
/// (~31k items, 293 agents), where what is left is `resync` rather than the
/// fold, so shrinking further spends memory against a cost the fold no longer
/// dominates.
const SNAPSHOT_STRIDE: usize = 1024;

/// A folded model captured at a known point on the timeline.
struct Snapshot {
    /// How many items were folded into `model`.
    folded: usize,
    /// The [`Timeline::generation`] this was taken under. A rung from an older
    /// generation describes a prefix that no longer exists — see the field's
    /// docs — and must be discarded rather than restored.
    generation: u64,
    /// The model as of `folded` items. Restoring it is a clone, which is O(1).
    ///
    /// This includes derived projections (liveness, workflow rollups) as they
    /// stood when the rung was taken, which are meaningless at a different
    /// playhead — every restore re-runs them via `resync`.
    model: SessionModel,
}

/// The central application state, owned by the single UI task.
///
/// Its fields are public for the two frontends that drive it, this crate's own
/// [`tui`](crate::tui) loop and the browser crate. They are frontend plumbing
/// rather than an interface to build on, and carry no stability promise.
pub struct App {
    /// The rendered flow graph (agent cards + step edges).
    pub flow: AgentFlow,
    /// The pure domain model the graph is projected from.
    pub session: SessionModel,
    /// Live vs replay.
    pub mode: Mode,
    /// Play/pause state — freezes the playhead in **both** replay and live (a
    /// live pause parks at the edge and buffers appends). See `toggle_play_pause`.
    pub is_paused: bool,
    /// Id of the session currently being watched; events for other ids are
    /// dropped (stale buffered messages across a switch).
    pub current_session_id: String,
    /// Who drives the viewport: overview (auto-fit), follow (track activity),
    /// or manual. Manual pan/zoom takes the camera; `o`/`f` give it back.
    pub camera: Camera,
    /// Whether the help overlay is shown (`?` toggles, `esc` closes).
    pub show_help: bool,
    /// Layout deferred while the camera is Manual: structural changes use
    /// local placement only (nothing existing moves under the user); the full
    /// Sugiyama runs when the camera re-engages (`o`/`f`).
    pub layout_dirty: bool,
    /// Most recent tailer error, surfaced in the hint bar (`None` = healthy).
    pub last_error: Option<String>,
    /// True once the event loop should terminate.
    pub should_quit: bool,
    /// In-progress Follow camera glide, advanced each frame by
    /// [`tick_camera`](Self::tick_camera). `None` when the camera is settled or
    /// not in Follow.
    pub camera_glide: Option<CameraGlide>,
    /// The unified replay/live timeline: the ts-ordered item list plus the
    /// playhead. The App folds the prefix `items[0..fold_target()]` into
    /// [`session`](Self::session); advanced each frame by
    /// [`tick_timeline`](Self::tick_timeline).
    pub timeline: Timeline,
    /// When the last genuine append (tail `Batch`) folded — drives the emergent
    /// "live" state ([`transport_at`](Self::transport_at)). `None` until one arrives.
    pub last_batch_at: Option<web_time::Instant>,
    /// Session-level metadata kept OFF the timeline (final mode/permission-mode,
    /// last prompt, queued/file-edit counts), shown in the `i` overlay. Session-
    /// constant, so it survives seek rebuilds (lives here, not in `session`).
    pub session_info: SessionInfo,
    /// Whether the `i` session-info overlay is shown (`i` toggles, `esc` closes).
    pub show_info: bool,
    /// Node id awaiting a center-glide, set on a selection and consumed by the
    /// graph view AFTER the flow has rendered — `center_on` probes the
    /// last-rendered size, so centering at event time would use a stale or
    /// absent canvas.
    pub pending_center: Option<String>,
    /// Scrub target (a time on the lane axis) queued by input handling,
    /// applied once per frame by [`tick_timeline`](Self::tick_timeline) — a
    /// burst of drag events costs ONE seek (a backward seek rebuilds the whole
    /// model), not one per mouse event.
    pub pending_seek: Option<chrono::DateTime<chrono::Utc>>,
    /// Ladder of folded-model snapshots, ascending by `folded`, used to start a
    /// backward seek near its target instead of re-folding from item zero.
    snapshots: Vec<Snapshot>,
    /// A mirror of rataflow's edge-animation clock (advanced by
    /// [`tick_animation`](Self::tick_animation) exactly as rataflow advances
    /// its own), so the frame loop can see when the ants' phase changes.
    ants_ms: u64,
    /// Which view the body shows.
    pub view: View,
    /// The agent the user is looking at, shared by every view.
    pub selection: Selection,
    /// A transient notice, shown until its deadline.
    pub snackbar: Option<Snack>,
    /// Agents whose status just changed, emphasised briefly.
    pub flash: Option<Flash>,
    /// Lane view: fold idle stretches of the time axis (`z`).
    pub fold_gaps: bool,
    /// Where the last frame put clickable things.
    pub hit: HitMap,
    /// First agent row the Now view shows (it scrolls to keep the selection
    /// visible and writes the offset back).
    pub now_offset: usize,
    /// The loop's clock, set every tick ([`tick_clock`](Self::tick_clock)).
    /// Rendering reads monotonic time from here (snackbar and flash deadlines)
    /// rather than from the system.
    pub clock: web_time::Instant,
    /// The offset clock times are shown in: the local zone when the app
    /// starts; tests pin UTC.
    pub utc_offset: chrono::FixedOffset,
    /// Statuses as of the last sync, to notice changes worth a flash.
    last_statuses: std::collections::HashMap<String, session::AgentStatus>,
    /// Parents whose done subagents the user unfolded (`enter` on a fold).
    pub expanded_folds: std::collections::HashSet<String>,
    /// Where folded agents' cards stood, so they come back to the same spot.
    pub parked_positions: std::collections::HashMap<String, rataflow::types::Position>,
    /// The folds the graph was last synced with — see
    /// [`reconcile_folds`](Self::reconcile_folds).
    synced_folds: FoldSet,
}

impl App {
    /// Construct the initial app state for a session id and mode, with an empty
    /// configured flow and a fresh model.
    pub fn new(session_id: String, mode: Mode) -> Self {
        App {
            flow: graph::new_flow(),
            session: SessionModel::new(session_id.clone()),
            mode,
            is_paused: false,
            current_session_id: session_id,
            camera: Camera::Overview,
            show_help: false,
            layout_dirty: false,
            last_error: None,
            should_quit: false,
            camera_glide: None,
            timeline: Timeline::new(),
            last_batch_at: None,
            session_info: SessionInfo::default(),
            show_info: false,
            pending_center: None,
            pending_seek: None,
            snapshots: Vec::new(),
            ants_ms: 0,
            // The herdr key launches with no view argument, so this is what
            // opening shows: the graph.
            view: View::Graph,
            selection: Selection::default(),
            snackbar: None,
            flash: None,
            fold_gaps: true,
            hit: HitMap::default(),
            now_offset: 0,
            clock: web_time::Instant::now(),
            utc_offset: *chrono::Local::now().offset(),
            last_statuses: Default::default(),
            expanded_folds: Default::default(),
            parked_positions: Default::default(),
            synced_folds: FoldSet::default(),
        }
    }

    /// The transport state as of now — a test shorthand; the ui reads
    /// [`transport_at`](Self::transport_at) with the loop's clock.
    #[cfg(test)]
    pub fn transport(&self) -> Transport {
        self.transport_at(web_time::Instant::now())
    }

    /// The emergent transport state at `now` (the top bar's badge). "Live"
    /// requires both following the edge AND a recent append, so an old
    /// session followed to its (static) edge reads `Idle`, not `Live`.
    pub fn transport_at(&self, now: web_time::Instant) -> Transport {
        // Paused is explicit user intent, so it outranks "parked in the past":
        // pausing drops `follow_head` (it parks the cursor), and a deliberate
        // pause should read `Paused`, not `History`.
        if self.is_paused {
            return Transport::Paused;
        }
        if !self.timeline.follow_head {
            return Transport::History;
        }
        let fresh = self
            .last_batch_at
            .is_some_and(|t| now.saturating_duration_since(t) < LIVE_FRESH);
        if fresh && self.timeline.at_edge() {
            return Transport::Live;
        }
        // Following but behind the edge = playing forward (a replay playing
        // through OR a live session catching up after a scrub-back), regardless
        // of the `replay` flag.
        if !self.timeline.at_edge() {
            return Transport::Playing;
        }
        Transport::Idle
    }

    /// Seek to a fraction (`0.0..=1.0`) of the timeline. Index-based (see
    /// `Timeline::fold_at_fraction`), so activity is evenly reachable. No-op
    /// when empty.
    pub fn seek_to_fraction(&mut self, f: f64) {
        let len = self.timeline.items.len();
        if len == 0 {
            return;
        }
        // Map the click to a folded-item count over the reachable range so the
        // leftmost click lands on the start clump (position 0).
        let target = self.timeline.fold_at_fraction(f).clamp(1, len);
        self.timeline.cursor = self.timeline.ts_at_index(target - 1);
        self.timeline.follow_head = target >= len;
        self.commit_seek(target);
    }

    /// Step the playhead to the previous/next prompt-era boundary (the natural
    /// tick marks of a session). Re-pins to the edge when stepping past the last
    /// prompt; clamps to the start when stepping before the first.
    pub fn seek_prompt(&mut self, forward: bool) {
        let Some(cur) = self.timeline.cursor else {
            return;
        };
        // Prompt timestamps from the WHOLE timeline, sorted — NOT the folded
        // model. The folded model is (re)built from `items[0..folded]`, so it
        // can't see prompts ahead of the playhead; `]` (next era) needs those,
        // or it overshoots straight to the edge.
        let mut bounds: Vec<chrono::DateTime<chrono::Utc>> = self
            .timeline
            .items
            .iter()
            .filter_map(|i| {
                i.facts
                    .iter()
                    .find(|f| {
                        matches!(f.kind, crate::fact::FactKind::Prompt(_))
                            && f.agent.as_deref() == Some(session::MAIN_ID)
                    })
                    .and_then(|f| i.ts().or(f.ts))
            })
            .collect();
        bounds.sort_unstable();

        let target = if forward {
            bounds.into_iter().find(|t| *t > cur)
        } else {
            bounds.into_iter().rev().find(|t| *t < cur)
        };
        match target {
            Some(t) => self.seek(t),
            // Past the last prompt → snap to the edge; before the first → start.
            None if forward => self.go_live(),
            None => {
                if let Some(start) = self.timeline.start_ts() {
                    self.seek(start);
                }
            }
        }
    }

    /// Re-pin to the timeline edge ("go live" / jump to end) and resume playback.
    pub fn go_live(&mut self) {
        self.is_paused = false;
        if let Some(head) = self.timeline.head_ts() {
            self.seek(head);
        } else {
            self.timeline.follow_head = true;
        }
    }

    /// Fold a tailer [`UiEvent`] into state.
    ///
    /// Drops events whose `session_id` is not [current](Self::is_current),
    /// applies batched updates to the [`SessionModel`], re-syncs the graph, and
    /// re-fits the view on structural changes while the follow camera is
    /// engaged. Switches the current session id on reset.
    pub fn handle_ui_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::Batch {
                session_id,
                statements,
            } => {
                if !self.is_current(&session_id) {
                    return;
                }
                // Route session-level metadata to the info store, whichever
                // record carried it; only real activity (timestamped / dated)
                // goes on the timeline.
                let mut activity = Vec::with_capacity(statements.len());
                for mut statement in statements {
                    for f in statement.take_session_meta() {
                        self.session_info.apply(&f);
                    }
                    if !statement.facts.is_empty() {
                        activity.push(statement);
                    }
                }
                // Stamp freshness for the emergent "live" state.
                self.last_batch_at = Some(web_time::Instant::now());
                // Whether we're riding the edge — decide BEFORE `append_live`
                // moves the cursor.
                let following = self.timeline.following();
                if following {
                    // Attach: nothing is folded yet, so the whole backfill
                    // arrives as one batch. Fold it BY INDEX, which walks the
                    // sorted prefix and leaves a rung every stride — the direct
                    // apply below jumps `folded` straight to the edge, so the
                    // only rung it can ever take sits at the edge, and the very
                    // first backward seek would fold from item zero.
                    //
                    // Safe only here: an incremental batch can re-sort an item
                    // in BELOW `folded`, which an index fold from `folded` would
                    // skip. At zero there is no prefix to skip.
                    let structural = if self.timeline.folded == 0 {
                        self.timeline.append_live(activity);
                        let to = self.timeline.items.len();
                        let structural = self.fold_range(0, to);
                        self.timeline.folded = to;
                        structural
                    } else {
                        // The model is order-independent, so apply the new
                        // updates DIRECTLY (their position in the now-ts-sorted
                        // `items` is irrelevant) — out-of-order live arrivals
                        // never force a rebuild. Then mark the whole stream
                        // folded.
                        let mut structural = false;
                        for statement in &activity {
                            for fact in &statement.facts {
                                structural |= self.session.apply_fact(fact);
                            }
                        }
                        self.timeline.append_live(activity);
                        self.timeline.folded = self.timeline.items.len();
                        structural
                    };
                    self.note_fold();
                    self.commit_fold(structural);
                } else {
                    // Scrubbed back: new live data extends the (sorted) timeline
                    // but stays hidden until we seek forward. A rare late PAST
                    // item (ts ≤ cursor) becomes due → rebuild to fold it.
                    self.timeline.append_live(activity);
                    let target = self.timeline.fold_target();
                    if target != self.timeline.folded {
                        self.rebuild_to(target);
                    }
                }
            }
            UiEvent::ReplayLoaded {
                session_id,
                items,
                speed,
                info,
            } => {
                if !self.is_current(&session_id) {
                    return;
                }
                // Bulk hand-off: the App owns pacing from here. Fold the first
                // moment immediately so t=0 renders; `tick_timeline` paces on.
                self.session_info = info;
                self.timeline.load_replay(items, speed);
                if self.mode == Mode::Live {
                    // `--follow` on a file: ride the (possibly growing) edge
                    // instead of replaying from the start.
                    self.timeline.replay = false;
                    self.timeline.follow_head = true;
                    self.timeline.cursor = self.timeline.head_ts();
                }
                let target = self.timeline.fold_target();
                self.fold_to(target);
            }
            UiEvent::SessionReset { session_id } => {
                // Truncation/rotation/switch: adopt the new id and rebuild
                // from scratch so stale nodes don't linger. The tailer's
                // initial ANNOUNCE arrives as a same-id reset on a still-empty
                // graph — there is nothing to wipe, and resetting view state
                // there would clobber a camera/pin choice the user made while
                // waiting for the session to appear.
                let genuine = !self.is_current(&session_id) || self.flow.nodes().count() > 0;
                self.current_session_id = session_id.clone();
                self.session = SessionModel::new(session_id);
                self.flow = graph::new_flow();
                // Ids repeat across sessions (the root is always `main`, omp
                // task names recur), so what the user unfolded or dragged in
                // the old session must not carry over.
                self.expanded_folds.clear();
                self.parked_positions.clear();
                self.synced_folds = FoldSet::default();
                self.selection.folded = None;
                // A different session (or a truncated one) shares nothing with
                // the rungs we hold, and a fresh `Timeline` restarts the
                // generation counter — so they cannot be told apart by it.
                self.snapshots.clear();
                // A reset is a fresh live timeline (only live emits resets — the
                // initial announce, truncation, or auto-switch). Replay arrives
                // via ReplayLoaded, never a reset.
                self.timeline = Timeline::new();
                self.last_statuses.clear();
                self.flash = None;
                if genuine {
                    self.camera = Camera::Overview;
                    self.selection = Selection::default();
                    self.now_offset = 0;
                    self.layout_dirty = false;
                    self.camera_glide = None;
                    self.snack("Switched to a new session");
                }
            }
            UiEvent::Error(msg) => {
                self.last_error = Some(msg);
            }
        }
    }

    /// Fold the timeline prefix forward to `target` items and reconcile the view.
    ///
    /// The shared fold path for both feeders (live `Batch` append and replay
    /// pacing): apply newly-due updates to the model, re-sync the graph, note
    /// status changes, and reframe for the camera. A no-op when nothing new
    /// is due. Backward moves (`target < folded`) are a seek and rebuild, handled
    /// in [`seek`](Self::seek) — not here.
    fn fold_to(&mut self, target: usize) {
        if target <= self.timeline.folded {
            return;
        }
        let structural = self.fold_range(self.timeline.folded, target);
        self.timeline.folded = target;
        self.commit_fold(structural);
    }

    /// Fold `items[from..to]` into the model, taking a ladder rung every
    /// [`SNAPSHOT_STRIDE`] items. Returns whether anything structural changed.
    ///
    /// Rungs are taken INSIDE the loop, not after it: a jump straight to the
    /// live edge folds the whole timeline in one call, and a ladder that only
    /// recorded where each fold *ended* would hold a single rung at the edge —
    /// useless for seeking backward, which needs a rung at or before its target.
    fn fold_range(&mut self, from: usize, to: usize) -> bool {
        let generation = self.timeline.generation;
        self.drop_stale_rungs(generation);
        let mut structural = false;
        for i in from..to {
            for fact in &self.timeline.items[i].facts {
                structural |= self.session.apply_fact(fact);
            }
            self.maybe_snapshot(i + 1, generation);
        }
        structural
    }

    /// Record a ladder rung for a fold that did not go through
    /// [`fold_range`](Self::fold_range) — the live `Batch` path, which applies
    /// updates directly (the model is order-independent) and jumps `folded` to
    /// the edge. Successive batches leave rungs roughly a stride apart, which
    /// is what a later backward seek restores from.
    fn note_fold(&mut self) {
        let generation = self.timeline.generation;
        self.drop_stale_rungs(generation);
        self.maybe_snapshot(self.timeline.folded, generation);
    }

    /// Push a rung if `folded` is a full stride past the nearest rung below it.
    ///
    /// Shared by both fold paths so the cadence cannot drift between them.
    ///
    /// Measured against the nearest rung at or below `folded`, not the last one
    /// in the vec: with a rung already at the live edge, comparing against the
    /// last would refuse every rung beneath it, so a fold that walks the early
    /// timeline could never leave a ladder behind and each backward seek would
    /// re-fold from zero.
    fn maybe_snapshot(&mut self, folded: usize, generation: u64) {
        let at = self.snapshots.partition_point(|s| s.folded <= folded);
        let below = at.checked_sub(1).map_or(0, |i| self.snapshots[i].folded);
        if folded >= below + SNAPSHOT_STRIDE {
            self.snapshots.insert(
                at,
                Snapshot {
                    folded,
                    generation,
                    model: self.session.clone(),
                },
            );
            // This rung folds the prefix as it stands NOW, so every re-sort
            // recorded so far is already baked into it — including the ones
            // that predate the whole ladder (a replay load moves every index).
            // Leaving them pending would make the next prune apply them to
            // rungs they cannot possibly invalidate, and the ladder would
            // collapse to a single rung on the first out-of-order batch.
            self.timeline.take_disturbance();
        }
    }

    /// Reconcile the ladder with a re-sorted or replaced item list.
    ///
    /// A rung describes `items[0..folded]`, so it survives exactly as long as
    /// that prefix does. The timeline reports how much of it the re-sorts left
    /// alone ([`Timeline::take_disturbance`]); rungs below that line are still
    /// accurate and are re-stamped to the new generation instead of thrown
    /// away. That distinction is what keeps the ladder alive on a live session:
    /// a subagent's entries arriving a moment out of order re-sort the tail on
    /// most batches, and voiding the whole ladder there left every backward
    /// seek folding from item zero, which is the case it exists for.
    fn drop_stale_rungs(&mut self, generation: u64) {
        if self
            .snapshots
            .last()
            .is_some_and(|s| s.generation != generation)
        {
            let stable = self.timeline.take_disturbance();
            self.snapshots.retain(|s| s.folded <= stable);
            for rung in &mut self.snapshots {
                rung.generation = generation;
            }
        }
    }

    /// Post-apply step shared by `fold_to` (forward, index-based — replay pacing
    /// and seeks) and the live `Batch` path (which applies updates directly,
    /// since the model is order-independent and `items` is kept ts-sorted): roll
    /// workflow status up, re-sync the graph, note status changes worth a
    /// flash, and reframe the camera.
    fn commit_fold(&mut self, structural: bool) {
        let sync_structural = self.resync();
        // The first live fold carries the whole existing file: the baseline is
        // empty then, so history is absorbed without flashing.
        self.note_status_changes(true);

        match self.camera {
            // Overview: a structural change re-frames the graph (deferred fit,
            // safe before first render).
            Camera::Overview => {
                if (structural || sync_structural) && self.session.agent_count() > 0 {
                    self.flow.request_fit_view();
                }
            }
            // Follow: keep the most recently active agent centered.
            Camera::Follow => self.track_activity(),
            Camera::Manual => {}
        }
    }

    /// Advance the timeline playhead one frame and fold any newly-due items.
    ///
    /// Replay paces the cursor forward by `elapsed × speed`; live following is a
    /// no-op here (the cursor is pinned to the head as `Batch`es arrive). On
    /// replay end, settle interactive agents to idle exactly once.
    pub fn tick_timeline(&mut self, elapsed: std::time::Duration) {
        // Apply at most one queued scrub per frame (see `pending_seek`).
        if let Some(t) = self.pending_seek.take() {
            self.seek(t);
        }
        self.timeline.advance(elapsed, self.is_paused);
        let target = self.timeline.fold_target();
        self.fold_to(target);
        if self.timeline.just_ended() {
            // The recording is over: every interactive agent goes idle
            // (completion is unclaimable; activity is provably absent).
            self.session.end_of_stream();
            // Settling is what folds a finished replay's done subagents, and
            // this sync already records the folds, so `reconcile_folds` will
            // not see them change: re-frame here as it would.
            if self.resync() && self.camera == Camera::Overview {
                self.flow.request_fit_view();
            }
            self.note_status_changes(true);
        }
    }

    /// Seek the playhead to `target` (a lane-axis click, an arrow step) and
    /// rebuild the view
    /// as-of-then. Forward seeks fold the new prefix in place (cheap); backward
    /// seeks rebuild a fresh model from the prefix (see `rebuild_to`). Re-pins
    /// to the edge when seeking to/past the head. A seek is discontinuous, so
    /// the glide resets rather than animating across the jump.
    ///
    pub fn seek(&mut self, target: chrono::DateTime<chrono::Utc>) {
        let head = self.timeline.head_ts();
        self.timeline.cursor = Some(target);
        self.timeline.follow_head = head.is_none_or(|h| target >= h);
        let target_fold = self.timeline.fold_target();
        self.commit_seek(target_fold);
    }

    /// Move the model to `target` folded items — the shared body of every seek
    /// (by time, fraction, or index). Assumes `cursor`/`follow_head` are already
    /// set. Forward folds in place; backward rebuilds (see `rebuild_to`). A
    /// seek is discontinuous, so the glide resets rather than animating across
    /// the jump, and status changes become the new baseline instead of flashing.
    ///
    fn commit_seek(&mut self, target: usize) {
        // A seek changes which tools the detail lists — a stale scroll offset
        // would blank the (now-shorter) list until the user scrolled.
        self.selection.reset_scroll();
        // A seek is a discontinuous jump → drop the stale per-gap pacing budget.
        self.timeline.reset_pacing();
        // Seeking to the live edge means "follow from here" — a lingering pause
        // would freeze the playhead at the edge instead of riding it.
        if self.timeline.follow_head {
            self.is_paused = false;
        }
        if target < self.timeline.folded {
            self.rebuild_to(target);
        } else {
            self.fold_range(self.timeline.folded, target);
            self.timeline.folded = target;
            self.resync();
        }

        self.camera_glide = None;
        self.flash = None;
        self.note_status_changes(false);
        // A seek is time-navigation, not a spatial change — the camera is the
        // user's, so don't reframe it (that snapped the graph on every scrub
        // click). Only Follow tracks the action, and it glides (smooth, not a
        // snap). Overview re-fits on live GROWTH, not on a scrub.
        if self.camera == Camera::Follow {
            self.track_activity();
        }
    }

    /// Rebuild the model and graph from scratch by re-folding `items[0..target]`
    /// into a fresh [`SessionModel`] — the only way to move the playhead
    /// *backward* (folding is forward-only). The `Flow` is patched rather than
    /// rebuilt, so node positions (the user's arrangement / a stable layout),
    /// the viewport and the selection are never disturbed in the first place.
    fn rebuild_to(&mut self, target: usize) {
        // Start from the newest ladder rung at or before the target instead of
        // from item zero. Restoring a rung is a clone of a persistent model —
        // O(1) — so the fold that follows is bounded by SNAPSHOT_STRIDE rather
        // than by how far back the user seeked.
        let generation = self.timeline.generation;
        self.drop_stale_rungs(generation);
        let resume = self
            .snapshots
            .iter()
            .rev()
            .find(|s| s.folded <= target)
            .map(|s| (s.model.clone(), s.folded));
        let (model, start) = match resume {
            Some((model, folded)) => (model, folded),
            None => (SessionModel::new(self.current_session_id.clone()), 0),
        };

        // Keep the model we are replacing, to work out what left the canvas.
        let previous = std::mem::replace(&mut self.session, model);
        // Re-folding also rebuilds the ladder above `start`, so a scrub that
        // walks backward repeatedly keeps finding rungs near where it lands.
        self.fold_range(start, target);
        self.timeline.folded = target;

        // Seeking back can only REMOVE agents (a fold never un-spawns one that
        // the earlier prefix already had), and `sync` adds and updates but never
        // removes — so taking those few nodes off the canvas is the whole
        // difference between here and a full rebuild.
        //
        // `OrdMap::diff` skips subtrees the two models share, and they share
        // almost everything: the new model is a rung cloned from this very
        // lineage. So this costs what actually changed, not what exists.
        let departed: Vec<String> = previous
            .agents
            .diff(&self.session.agents)
            .filter_map(|change| match change {
                imbl::ordmap::DiffItem::Remove(id, _) => Some(id.clone()),
                _ => None,
            })
            .collect();
        graph::remove_agents(&mut self.flow, &departed);

        // No flow wipe, so node positions, the viewport and the selection all
        // survive on their own — none of it has to be captured and restored
        // around a teardown any more.
        self.resync();
    }

    /// Roll group status up from children, then project the model onto
    /// the flow. Every event arm that mutates the model funnels through here so
    /// the workflow rollup can never be skipped before a sync (a just-completed
    /// group would otherwise render Running with an animated edge until the next
    /// batch). Returns whether the sync changed graph structure.
    fn resync(&mut self) -> bool {
        self.session.recompute_group_status();
        // Interactive sidechains (forks) derive liveness against the timeline's
        // "now": wall clock at a live edge, the playhead when replaying or
        // scrubbed back (so the as-of-then state shows, no wall-clock bleed).
        let now = self.timeline.now_reference();
        self.session.recompute_liveness(now);
        self.sync_graph()
    }

    /// Project the model onto the flow, the current folds folded away.
    ///
    /// Layout is user-driven: a Sugiyama pass on every new node reflows the
    /// whole graph and reads as "jumpy" as a session grows. So sync NEVER
    /// auto-relayouts — new nodes keep their local placement (below parent,
    /// fanned past siblings), folded cards come back where they stood, and
    /// nothing existing moves until the user asks to tidy (`r`, or
    /// re-engaging the camera with `o`/`f`). `layout_dirty` tracks that there
    /// is un-applied growth for the on-demand path. Returns whether the
    /// structure changed.
    fn sync_graph(&mut self) -> bool {
        let folds = self.folds();
        let structural = graph::sync(
            &mut self.flow,
            &self.session,
            &folds,
            &mut self.parked_positions,
            false,
        );
        self.synced_folds = folds;
        if structural {
            self.layout_dirty = true;
        }
        structural
    }

    /// Whether a finished replay sits at its end: the recording is over, so
    /// a done subagent will not come back. Derived every time rather than
    /// stored, so it holds again after a seek back and a return to the end,
    /// and lets go when the file grows.
    pub fn settled(&self) -> bool {
        self.timeline.replay && self.timeline.ended() && self.timeline.at_edge()
    }

    /// The done subagents folded away right now.
    pub fn folds(&self) -> FoldSet {
        view::fold_set(&self.session, &self.expanded_folds, self.settled())
    }

    /// Bring the graph in line with the folds, whatever changed them: a model
    /// change, time alone, a seek, the replay settling or the file growing
    /// under a paused end, or the user unfolding. Called once per loop turn;
    /// re-syncs only when the folds differ from what the graph was synced
    /// with. A fold moves no card (layout stays on `r`); the Overview camera
    /// re-frames. Returns whether the graph changed.
    pub fn reconcile_folds(&mut self) -> bool {
        if self.folds() == self.synced_folds {
            return false;
        }
        if self.sync_graph() && self.camera == Camera::Overview {
            self.flow.request_fit_view();
        }
        true
    }

    /// Tidy the graph on demand (`r`): run Sugiyama now and reframe for the
    /// current camera. Layout is never automatic (see `resync`),
    /// so this is the user's explicit "rearrange". Forces a pass even when not
    /// `layout_dirty`, so it also re-tidies after manual node dragging.
    pub fn relayout_now(&mut self) {
        graph::relayout(&mut self.flow);
        self.layout_dirty = false;
        match self.camera {
            Camera::Overview => self.flow.request_fit_view(),
            Camera::Follow => self.track_activity(),
            Camera::Manual => {}
        }
    }

    /// Whether `session_id` matches the session currently being watched.
    pub fn is_current(&self, session_id: &str) -> bool {
        self.current_session_id == session_id
    }

    /// Periodic status re-derivation, called by the event loop (~1s).
    ///
    /// Interactive liveness is time-based, so a fully quiet session (no
    /// batches arriving) still needs its running→idle transitions to render —
    /// but the common nothing-changed tick must be near-free, so the graph is
    /// only re-synced when a status actually flipped (no per-second content
    /// rebuilds). Follow's auto-narration also re-engages here: esc-unpin and
    /// stale centering must not wait for the next batch.
    ///
    /// Returns whether the picture may have changed: a status flipped, or
    /// Follow re-tracked (which can move the selection or the camera).
    pub fn status_tick(&mut self) -> bool {
        let now = self.timeline.now_reference();
        let flipped = self.session.recompute_liveness(now);
        if flipped {
            self.session.recompute_group_status();
            // Time alone can move a quiet subagent between Running and Done,
            // which forms or dissolves a fold: a structural change.
            if self.sync_graph() && self.camera == Camera::Overview {
                self.flow.request_fit_view();
            }
            self.note_status_changes(true);
        }
        let following = self.camera == Camera::Follow;
        if following {
            self.track_activity();
        }
        flipped || following
    }

    /// Center the camera on the most recently active agent (Follow mode).
    ///
    /// Quiet no-op when there are no agents or before the first render
    /// (`center_on` needs the canvas size from the last render). The actual move
    /// is an eased glide (`focus_camera` + per-frame
    /// [`tick_camera`](Self::tick_camera)) rather than a teleport.
    pub fn track_activity(&mut self) {
        let Some(id) = self.session.last_active_agent_id() else {
            return;
        };
        self.center_node(&id, true); // Follow: clamp zoom for readability.
        // In Follow the selection narrates the followed agent. This is not a
        // user gesture, so it must not drop Follow the way `select_agent` does.
        if self.selected_agent_id().as_deref() != Some(id.as_str()) {
            self.mirror_selection(Some(id));
        }
    }

    /// Glide the camera so `id`'s node ends up centered in the canvas — the
    /// shared move behind Follow's tracking and click-to-center. A folded
    /// agent is centered by its fold's card. Quiet no-op for an unknown id or
    /// before the first render.
    ///
    /// The pan always glides. `clamp_zoom` additionally snaps the zoom into the
    /// card-readable band — raised to [`FOLLOW_ZOOM`], then clamped down so the
    /// whole card fits the canvas. That snap belongs to
    /// **Follow** (auto-tracking must guarantee legibility) and explicit centering,
    /// so those pass `true`. Manual spatial-nav passes `false`: an arrow press is
    /// just a pan, and must not yank the zoom the user set (the snap-vs-glide
    /// mismatch reads as jumpy, and Manual = the user's camera). Idempotent when
    /// already at the target, so Follow's per-tick re-tracking never restarts it.
    pub fn center_node(&mut self, id: &str, clamp_zoom: bool) {
        let Some(node) = self.flow.node(id).or_else(|| {
            let parent = self.synced_folds.member_of.get(id)?;
            self.flow.node(&fold_card_id(parent))
        }) else {
            return;
        };
        let (w, h) = (node.width, node.height);
        let center = (node.position.x + w / 2.0, node.position.y + h / 2.0);

        let canvas = self.flow.canvas_size();
        if clamp_zoom && canvas.width > 0.0 && canvas.height > 0.0 && w > 0.0 && h > 0.0 {
            // Fit zoom: the largest zoom at which the card (plus breathing
            // room) still fits both canvas dimensions.
            let fit = ((canvas.width - 2.0 * NODE_FIT_PAD) / w)
                .min((canvas.height - 2.0 * NODE_FIT_PAD) / h);
            let mut target = self.flow.viewport.zoom.max(FOLLOW_ZOOM);
            if fit > 0.0 {
                target = target.min(fit);
            }
            self.flow.zoom_to(target);
        }
        self.focus_camera(center);
    }

    /// Glide the viewport so `center` (a world point) ends up centered.
    ///
    /// Probes the destination offset by momentarily calling `center_on` and
    /// reading the result back (then restoring), so the glide lands byte-exact
    /// regardless of zoom. Snaps instead of gliding for sub-pixel moves, and
    /// leaves an in-flight glide undisturbed when the target is unchanged (so
    /// re-tracking the same stationary agent each tick doesn't restart it).
    fn focus_camera(&mut self, center: (f64, f64)) {
        let from = (self.flow.viewport.x, self.flow.viewport.y);
        // Probe: center_on mutates the viewport — capture the offset it lands on,
        // then restore so the glide (or snap) drives the actual move.
        self.flow.center_on(center);
        let to = (self.flow.viewport.x, self.flow.viewport.y);
        self.flow.viewport.set_offset(from.0, from.1);

        let settled =
            (to.0 - from.0).abs() < GLIDE_SNAP_EPS && (to.1 - from.1).abs() < GLIDE_SNAP_EPS;
        if settled {
            self.flow.viewport.set_offset(to.0, to.1);
            self.camera_glide = None;
            return;
        }
        // Same destination as an in-flight glide → let it finish, don't restart.
        if let Some(g) = &self.camera_glide
            && (g.to.0 - to.0).abs() < GLIDE_SNAP_EPS
            && (g.to.1 - to.1).abs() < GLIDE_SNAP_EPS
        {
            return;
        }
        self.camera_glide = Some(CameraGlide { from, to, t: 0.0 });
    }

    /// Advance an in-progress camera glide by `dt`, writing the eased offset to
    /// the viewport. Called every frame from the event loop; a no-op when no
    /// glide is active. Viewport writes are quiet, so this never trips the
    /// Manual-camera detection. Returns whether the move shows — the viewport
    /// moved and the graph is on screen — true on the glide's last step too,
    /// when it lands and stops animating.
    pub fn tick_camera(&mut self, dt: std::time::Duration) -> bool {
        let Some(glide) = self.camera_glide.as_mut() else {
            return false;
        };
        glide.t = (glide.t + dt.as_secs_f64() / GLIDE_SECS).min(1.0);
        let (x, y) = glide.offset();
        let done = glide.t >= 1.0;
        let to = glide.to;
        self.flow.viewport.set_offset(x, y);
        if done {
            self.flow.viewport.set_offset(to.0, to.1);
            self.camera_glide = None;
        }
        self.view == View::Graph
    }

    /// Advance rataflow's edge animation by `elapsed`, keeping the app's
    /// mirror of its clock in step (see `frame_stamp`).
    pub fn tick_animation(&mut self, elapsed: std::time::Duration) {
        self.flow.tick_animation(elapsed);
        // rataflow adds whole milliseconds and wraps at one pattern cycle
        // (`ANIMATION_PATTERN_LENGTH` = 3 phases); the same arithmetic keeps
        // the mirror on the phase it draws.
        let cycle = 3 * self.flow.animation_speed_ms.max(1);
        self.ants_ms = (self.ants_ms + elapsed.as_millis() as u64) % cycle;
    }

    /// Advance the flow's edge-of-canvas auto-pan. Returns whether the canvas
    /// is in motion: an auto-pan moved the viewport, or a drag is under way.
    pub fn tick_auto_pan(&mut self, elapsed: std::time::Duration) -> bool {
        let panned = matches!(
            self.flow.tick_auto_pan(elapsed),
            rataflow::EventResponse::Event(_)
        );
        panned || self.flow.is_dragging()
    }

    /// The selected agent's id, shared by every view.
    pub fn selected_agent_id(&self) -> Option<String> {
        self.selection.agent.clone()
    }

    /// The agent rows every view lists: tree order, with depth.
    pub fn agent_rows(&self) -> Vec<(String, usize)> {
        self.session
            .tree_order()
            .into_iter()
            .map(|(id, depth)| (id.to_string(), depth))
            .collect()
    }

    /// Select `id` (or nothing) as the user's own choice: the detail list
    /// returns to its newest call, the graph's selection mirrors it (quietly,
    /// so no flow event echoes back), the graph glides to it, and Follow hands
    /// the camera to the user. Selecting nothing also lets go of a fold.
    pub fn select_agent(&mut self, id: Option<String>) {
        if self.selection.agent == id && self.selection.folded.is_none() {
            return;
        }
        self.take_camera();
        self.mirror_selection(id);
        self.pending_center = self.selection.agent.clone();
    }

    /// Select the fold under `parent` as the user's own choice — the fold's
    /// counterpart of [`select_agent`](Self::select_agent).
    pub fn select_fold(&mut self, parent: String) {
        if self.selection.folded.as_deref() == Some(parent.as_str()) {
            return;
        }
        self.take_camera();
        let card = fold_card_id(&parent);
        if self.flow.node(&card).is_some() {
            self.flow.select_node(&card);
        } else {
            self.flow.clear_selection();
        }
        self.selection.agent = None;
        self.selection.folded = Some(parent);
        self.selection.expanded = false;
        self.selection.reset_scroll();
        self.pending_center = Some(card);
    }

    /// A user's selection takes the camera from Follow.
    fn take_camera(&mut self) {
        if self.camera == Camera::Follow {
            self.camera = Camera::Manual;
            self.camera_glide = None;
        }
    }

    /// Point the selection at `id` without any of a user gesture's side
    /// effects on the camera: what Follow does as it narrates.
    fn mirror_selection(&mut self, id: Option<String>) {
        match id.as_deref() {
            Some(node) => self.flow.select_node(node),
            None => self.flow.clear_selection(),
        }
        self.selection.agent = id;
        self.selection.folded = None;
        self.selection.expanded = false;
        self.selection.reset_scroll();
    }

    /// Move the selection `step` rows (`j`/`k`): along the Now view's rows,
    /// folds included, or on the lanes along every agent, since the lanes
    /// draw them all. A selected fold steps on the lanes from its first
    /// member. Starts at the root when nothing is selected; stops at either
    /// end.
    pub fn select_step(&mut self, step: isize) {
        let folds = self.folds();
        let rows = if self.view == View::Lanes {
            self.session
                .tree_order()
                .into_iter()
                .map(|(id, depth)| Row::Agent { id, depth })
                .collect()
        } else {
            view::rows(&self.session, &folds)
        };
        if rows.is_empty() {
            return;
        }
        let agent = self.selection.agent.as_deref().or_else(|| {
            let parent = self.selection.folded.as_deref()?;
            folds.by_parent.get(parent)?.first().map(String::as_str)
        });
        let fold = self.selection.folded.as_deref().or_else(|| {
            let id = self.selection.agent.as_deref()?;
            folds.member_of.get(id).map(String::as_str)
        });
        let current = rows.iter().position(|row| match *row {
            Row::Agent { id, .. } => agent == Some(id),
            Row::Folded { parent, .. } => fold == Some(parent),
        });
        let next = match current {
            None => 0,
            Some(at) => (at as isize + step).clamp(0, rows.len() as isize - 1) as usize,
        };
        match rows[next] {
            Row::Agent { id, .. } => {
                let id = id.to_string();
                self.select_agent(Some(id));
            }
            Row::Folded { parent, .. } => {
                let parent = parent.to_string();
                self.select_fold(parent);
            }
        }
    }

    /// In the views that fold (Now, Graph), a selected agent a fold hides
    /// becomes a selection of that fold, and a selected fold that has come
    /// apart is let go. The lanes draw every agent, so there the selection
    /// stays as it is. Called on entering a view and before every frame.
    pub fn normalize_fold_selection(&mut self) {
        if self.view == View::Lanes {
            return;
        }
        let folds = self.folds();
        let hiding = self
            .selection
            .agent
            .as_ref()
            .and_then(|id| folds.member_of.get(id));
        if let Some(parent) = hiding {
            self.selection.folded = Some(parent.clone());
            self.selection.agent = None;
            self.selection.expanded = false;
            self.selection.detail = false;
        } else if self
            .selection
            .folded
            .as_ref()
            .is_some_and(|p| !folds.by_parent.contains_key(p))
        {
            self.selection.folded = None;
        }
    }

    /// Unfold the selected fold (`enter`): its members come back, and the
    /// first of them is selected. Returns whether a fold was selected.
    pub fn expand_fold(&mut self) -> bool {
        let Some(parent) = self.selection.folded.clone() else {
            return false;
        };
        let first = self
            .folds()
            .by_parent
            .get(&parent)
            .and_then(|members| members.first().cloned());
        self.expanded_folds.insert(parent);
        self.reconcile_folds();
        if first.is_some() {
            self.select_agent(first);
        }
        true
    }

    /// Fold the selected agent back into the fold it was unfolded from
    /// (`esc`), and select that fold. Returns whether there was one.
    pub fn collapse_fold(&mut self) -> bool {
        let Some(id) = self.selection.agent.as_deref() else {
            return false;
        };
        let settled = self.settled();
        // A direct member only: `member_of` also holds descendants, so with
        // nested folds unfolded it would match an ancestor's fold too.
        let parent = self.expanded_folds.iter().find(|p| {
            let mut rest = self.expanded_folds.clone();
            rest.remove(*p);
            view::fold_set(&self.session, &rest, settled)
                .by_parent
                .get(*p)
                .is_some_and(|members| members.iter().any(|m| m == id))
        });
        let Some(parent) = parent.cloned() else {
            return false;
        };
        self.expanded_folds.remove(&parent);
        self.reconcile_folds();
        self.select_fold(parent);
        true
    }

    /// Switch the body to `view`.
    pub fn set_view(&mut self, view: View) {
        if self.view == view {
            return;
        }
        self.view = view;
        self.selection.detail = false;
        self.normalize_fold_selection();
        // The graph keeps its own camera; bring the selection into its frame
        // once it is drawn.
        if view == View::Graph {
            self.pending_center = self
                .selection
                .agent
                .clone()
                .or_else(|| self.selection.folded.as_deref().map(fold_card_id));
        }
    }

    /// Show a short notice for [`SNACK_FOR`].
    pub fn snack(&mut self, message: impl Into<String>) {
        self.snackbar = Some(Snack {
            message: message.into(),
            until: self.clock + SNACK_FOR,
        });
    }

    /// Advance the app's clock to `now`, retiring notices that have expired.
    pub fn tick_clock(&mut self, now: web_time::Instant) {
        self.clock = now;
        if self.snackbar.as_ref().is_some_and(|s| s.until <= now) {
            self.snackbar = None;
        }
        if self.flash.as_ref().is_some_and(|f| f.until <= now) {
            self.flash = None;
        }
    }

    /// Compare each agent's status with the last sync's. With `flash`, the
    /// agents that changed are emphasised for [`FLASH_FOR`]; without (a seek,
    /// a first load) the new statuses just become the baseline — history is
    /// not news.
    fn note_status_changes(&mut self, flash: bool) {
        let flash = flash && !self.last_statuses.is_empty();
        let mut changed = Vec::new();
        for id in self.session.spawn_order() {
            let Some(agent) = self.session.agent(id) else {
                continue;
            };
            match self.last_statuses.get_mut(id) {
                Some(before) if *before == agent.status => {}
                Some(before) => {
                    *before = agent.status;
                    if flash {
                        changed.push(id.to_string());
                    }
                }
                None => {
                    self.last_statuses.insert(id.to_string(), agent.status);
                }
            }
        }
        if !changed.is_empty() {
            self.flash = Some(Flash {
                ids: changed,
                until: self.clock + FLASH_FOR,
            });
        }
    }

    /// Drop every ladder rung, keeping the folded model and the canvas.
    ///
    /// A bench hook, not a feature: pricing the ladder means reading the heap
    /// with the rungs and again without them, and the ladder is private. Hidden
    /// from the docs so it is no part of the published API's contract. The
    /// next fold or backward seek simply starts a fresh ladder.
    #[doc(hidden)]
    pub fn drop_snapshot_ladder(&mut self) {
        self.snapshots.clear();
    }

    /// Unified play/pause (`space`) that works from any state — **including at a
    /// live edge**:
    /// - **playing** (following the edge, not paused) → park at the current
    ///   playhead;
    /// - **paused or scrubbed into the past** → resume playing **from the current
    ///   cursor** (re-engage `follow_head` so `advance` paces forward from where
    ///   you are — it does NOT jump to the live edge; `End` does that).
    ///
    /// Pause drops `follow_head` on purpose: that's what freezes a *live* session.
    /// `append_live` snaps the cursor to the edge only while following, so a
    /// parked cursor lets new events buffer behind the edge (the same path a
    /// scrub-back uses) instead of yanking the view to each new event. Resume
    /// then catches up through the buffered gap.
    pub fn toggle_play_pause(&mut self) {
        let playing = self.timeline.follow_head && !self.is_paused;
        if playing {
            self.is_paused = true;
            self.timeline.follow_head = false;
        } else {
            self.is_paused = false;
            self.timeline.follow_head = true;
        }
    }

    /// React to the flow events from a user input gesture.
    ///
    /// Every event here is the product of a USER gesture (programmatic mutations
    /// are quiet), so the rule is uniform: Follow yields to ANY interaction;
    /// Overview yields only to a viewport change (pan/zoom). A selection change
    /// resets the detail list's scroll. Shared by the native input handler and
    /// the browser frontend.
    pub fn process_flow_events(&mut self, events: impl Iterator<Item = rataflow::FlowEvent>) {
        use rataflow::FlowEvent;
        for event in events {
            let drop_camera = self.camera == Camera::Follow
                || (self.camera == Camera::Overview
                    && matches!(event, FlowEvent::ViewportChanged { .. }));
            if drop_camera {
                self.camera = Camera::Manual;
                self.camera_glide = None;
            }
            if let FlowEvent::SelectionChanged { node_ids, .. } = &event {
                // A fold's card selects the fold, not an agent.
                let node = node_ids.first();
                let fold = node.and_then(|id| fold_of_card(id));
                self.selection.folded = fold.map(str::to_string);
                self.selection.agent = node.filter(|_| fold.is_none()).cloned();
                self.selection.expanded = false;
                self.selection.reset_scroll();
                // Pan the selected node to center (pan-only — the arrow-nav path
                // does NOT clamp zoom; see `center_node`). Deferred to the next
                // graph draw, which knows the canvas size. A deselection clears
                // any not-yet-consumed one.
                self.pending_center = node_ids.first().cloned();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fact::{Fact, FactKind, Statement};
    use crate::provider::claude::wire::SubagentMeta;
    use crate::provider::claude::{Record, Source};
    use crate::tailer::UiEvent;

    fn meta_update(agent_id: &str) -> Statement {
        Record::Meta {
            agent_id: agent_id.to_string(),
            workflow: None,
            meta: SubagentMeta {
                agent_type: Some("guide".into()),
                description: None,
                tool_use_id: Some("ag1".into()),
                stopped_by_user: None,
            },
        }
        .statement()
        .unwrap()
    }

    #[test]
    fn auto_switch_reset_adopts_new_session_id() {
        // The auto-switch path emits SessionReset stamped with the NEW id, then
        // the post-switch tailer emits Batches under the NEW id. The App must
        // adopt the new id so those batches are not dropped by is_current.
        let mut app = App::new("OLD".into(), Mode::Live);

        // Reset carrying the NEW session id (auto-switch signal).
        app.handle_ui_event(UiEvent::SessionReset {
            session_id: "NEW".into(),
        });
        assert_eq!(app.current_session_id, "NEW");

        // A batch from the new session must be accepted, not dropped.
        app.handle_ui_event(UiEvent::Batch {
            session_id: "NEW".into(),
            statements: vec![meta_update("newsub")],
        });
        assert!(
            app.session.agent("newsub").is_some(),
            "new-session subagent must be present after auto-switch"
        );
    }

    #[test]
    fn camera_defaults_to_overview_and_resets() {
        let mut app = App::new("s".into(), Mode::Live);
        assert_eq!(app.camera, Camera::Overview);

        app.camera = Camera::Manual; // user took the camera…
        app.handle_ui_event(UiEvent::SessionReset {
            session_id: "s2".into(),
        });
        // …but a fresh session re-engages the default.
        assert_eq!(app.camera, Camera::Overview);
    }

    /// The herdr key opens the binary with no view argument, so the first
    /// screen is whatever `App::new` starts on: the graph.
    #[test]
    fn app_opens_on_the_graph_view() {
        for mode in [Mode::Live, Mode::Replay] {
            assert_eq!(App::new("s".into(), mode).view, View::Graph);
        }
    }

    #[test]
    fn status_changes_flash_but_history_does_not() {
        let mut app = App::new("s".into(), Mode::Live);
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![meta_update("sub1")],
        });
        assert!(app.flash.is_none(), "the live-attach backfill is history");

        let ended = Statement {
            at: None,
            facts: vec![Fact {
                agent: Some("sub1".into()),
                ts: None,
                kind: FactKind::Ended(crate::fact::AgentStatus::Done),
            }],
        };
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![ended],
        });
        let flash = app.flash.clone().expect("a status change flashes");
        assert_eq!(flash.ids, ["sub1"]);
        assert_eq!(flash.until, app.clock + FLASH_FOR);

        // The loop's clock retires it.
        app.tick_clock(app.clock + FLASH_FOR);
        assert!(app.flash.is_none());
    }

    #[test]
    fn announce_reset_preserves_camera_on_empty_graph() {
        // The tailer's initial announce is a same-id reset before any content:
        // a camera choice made while waiting must survive it.
        let mut app = App::new("s".into(), Mode::Live);
        app.camera = Camera::Follow; // user pressed f while waiting
        app.handle_ui_event(UiEvent::SessionReset {
            session_id: "s".into(),
        });
        assert_eq!(
            app.camera,
            Camera::Follow,
            "announce must not clobber camera"
        );

        // A genuine reset (graph populated) still resets the view.
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![meta_update("sub1")],
        });
        app.handle_ui_event(UiEvent::SessionReset {
            session_id: "s".into(),
        });
        assert_eq!(app.camera, Camera::Overview);
    }

    #[test]
    fn status_tick_resumes_follow_narration() {
        let mut app = App::new("s".into(), Mode::Live);
        app.camera = Camera::Follow;
        // Deliver an agent while following.
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![meta_update("sub1")],
        });
        // Simulate a moment with no selection (e.g. just after a reset).
        app.selection.agent = None;
        assert!(app.selected_agent_id().is_none());

        // The 1s status tick must re-engage auto-narration without a batch.
        app.status_tick();
        assert!(
            app.selected_agent_id().is_some(),
            "follow must re-track on the status tick, not wait for a batch"
        );
    }

    #[test]
    fn batches_from_stale_session_are_dropped() {
        let mut app = App::new("CURRENT".into(), Mode::Live);
        app.handle_ui_event(UiEvent::Batch {
            session_id: "STALE".into(),
            statements: vec![meta_update("ghost")],
        });
        assert!(
            app.session.agent("ghost").is_none(),
            "stale-session batch must be dropped"
        );
    }

    #[test]
    fn camera_glide_eases_and_lands_exactly() {
        let mut app = App::new("s".into(), Mode::Live);
        app.camera_glide = Some(CameraGlide {
            from: (0.0, 0.0),
            to: (120.0, 0.0),
            t: 0.0,
        });

        // Halfway through the glide: smoothstep(0.5) == 0.5 → x == 60.
        app.tick_camera(std::time::Duration::from_secs_f64(GLIDE_SECS / 2.0));
        assert!(app.camera_glide.is_some(), "glide still running at t=0.5");
        assert!(
            (app.flow.viewport.x - 60.0).abs() < 1e-6,
            "eased midpoint should be 60, got {}",
            app.flow.viewport.x
        );

        // Overshooting the remaining time clamps to t=1: lands exactly, clears.
        app.tick_camera(std::time::Duration::from_secs_f64(GLIDE_SECS));
        assert!(app.camera_glide.is_none(), "glide cleared on completion");
        assert!(
            (app.flow.viewport.x - 120.0).abs() < 1e-6,
            "glide must land byte-exact on target"
        );
    }

    #[test]
    fn manual_spatial_nav_pans_but_leaves_zoom_untouched() {
        use crate::tailer::ReplayItem;
        use ratatui::widgets::Widget;

        let mut app = App::new("s".into(), Mode::Replay);
        let t: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: vec![ReplayItem::at(Some(t), meta_update("sub1").facts)],
            speed: 8.0,
            info: Default::default(),
        });
        // Render once so the flow has a non-zero canvas (the zoom clamp needs it).
        let area = ratatui::layout::Rect::new(0, 0, 100, 30);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        (&mut app.flow).render(area, &mut buf);
        assert!(app.flow.node("sub1").is_some(), "the subagent node exists");

        // Zoom OUT past the readable band — the clamp WOULD want to snap it in.
        app.flow.zoom_to(0.3);
        let zoom = app.flow.viewport.zoom;
        assert!(zoom < FOLLOW_ZOOM);

        // Manual spatial-nav (clamp_zoom = false) → pans, leaves the zoom alone.
        app.center_node("sub1", false);
        assert_eq!(
            app.flow.viewport.zoom, zoom,
            "arrow-nav must not touch the user's zoom"
        );

        // Follow / explicit center (clamp_zoom = true) → snaps the zoom in for
        // readability — the movement that used to ride along with every arrow.
        app.center_node("sub1", true);
        assert!(
            app.flow.viewport.zoom > zoom,
            "Follow still bumps zoom for readability"
        );
    }

    #[test]
    fn user_pan_cancels_glide() {
        let mut app = App::new("s".into(), Mode::Live);
        app.camera = Camera::Follow;
        app.camera_glide = Some(CameraGlide {
            from: (0.0, 0.0),
            to: (50.0, 0.0),
            t: 0.2,
        });

        // A user viewport gesture hands over the camera AND stops the glide, so
        // the app never fights the user's pan.
        crate::handler::process_flow_events(
            &mut app,
            vec![rataflow::FlowEvent::ViewportChanged {
                x: 1.0,
                y: 2.0,
                zoom: 1.0,
            }]
            .into_iter(),
        );
        assert_eq!(app.camera, Camera::Manual);
        assert!(app.camera_glide.is_none(), "user pan must cancel the glide");
    }

    /// A live attach ships the whole backfill as ONE batch, which is the case
    /// the ladder is least able to help with and most needs to: the live path
    /// applies updates directly and jumps `folded` to the edge, so the only
    /// rung ever taken sits AT the edge — and a backward seek finds nothing at
    /// or before its target and folds from item zero, exactly as if there were
    /// no ladder at all.
    #[test]
    fn a_live_attach_leaves_a_usable_ladder() {
        let t0: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        let n = SNAPSHOT_STRIDE * 3;
        let mut app = App::new("s".to_string(), Mode::Live);
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: (0..n)
                .filter_map(|i| {
                    Record::Entry {
                        source: Source::Main,
                        entry: crate::provider::claude::wire::parse_line(&format!(
                            r#"{{"type":"user","uuid":"u{i}","parentUuid":null,"timestamp":"{}","message":{{"role":"user","content":"hi"}}}}"#,
                            (t0 + chrono::Duration::seconds(i as i64)).to_rfc3339()
                        ))
                        .unwrap(),
                    }
                    .statement()
                })
                .collect(),
        });
        assert_eq!(app.timeline.folded, n, "the backfill folded to the edge");
        assert!(
            app.snapshots.iter().any(|s| s.folded <= n / 2),
            "no rung at or before mid-timeline: {:?}",
            app.snapshots.iter().map(|s| s.folded).collect::<Vec<_>>()
        );
    }

    /// The ladder is only allowed to be FASTER. Restoring a rung and folding
    /// forward from it must land on exactly the model a from-scratch rebuild
    /// would produce — otherwise a backward seek silently shows a different
    /// session than the same seek did yesterday.
    /// Seeking back patches the canvas instead of rebuilding it: agents that do
    /// not exist yet at the target leave, and everything the old teardown had to
    /// capture and restore by hand — node positions, the viewport, the
    /// selection — survives because nothing is thrown away.
    #[test]
    fn seeking_back_patches_the_canvas_instead_of_rebuilding_it() {
        let t0: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        let mut app = App::new("focused".to_string(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "focused".into(),
            items: (0..6)
                .map(|i| {
                    crate::tailer::ReplayItem::at(
                        Some(t0 + chrono::Duration::seconds(i as i64)),
                        meta_update(&format!("sub{i}")).facts,
                    )
                })
                .collect(),
            speed: 8.0,
            info: Default::default(),
        });
        app.go_live();

        let early = "sub1";
        let late = "sub5";
        assert!(app.flow.node(late).is_some(), "precondition: sub5 exists");

        // Arrange the canvas the way a user would: drag a node, pan, select.
        app.flow
            .set_node_positions(std::iter::once((early, (123.0, 456.0))));
        app.flow.zoom_to(2.5);
        let viewport = app.flow.viewport;
        app.select_agent(Some(early.to_string()));

        // Seek back to before sub5 (and sub4, sub3…) were ever spawned.
        app.seek_to_fraction(2.0 / 6.0);

        assert!(
            app.flow.node(late).is_none(),
            "an agent that does not exist at this playhead must leave the canvas"
        );
        assert!(app.session.agent("sub5").is_none(), "and leave the model");
        assert!(app.flow.node(early).is_some(), "sub1 predates the target");

        // The three things the old rebuild had to save and restore explicitly.
        let node = app.flow.node(early).unwrap();
        assert_eq!(
            (node.position.x, node.position.y),
            (123.0, 456.0),
            "a dragged position survived the seek"
        );
        assert_eq!(app.flow.viewport.zoom, viewport.zoom, "viewport survived");
        assert_eq!(app.selected_agent_id().as_deref(), Some("sub1"));
    }

    #[test]
    fn a_ladder_restore_matches_a_full_rebuild() {
        // Enough items that several rungs are taken.
        let count = SNAPSHOT_STRIDE * 3 + 17;
        let t0: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        let items = |n: usize| -> Vec<crate::tailer::ReplayItem> {
            (0..n)
                .map(|i| {
                    crate::tailer::ReplayItem::at(
                        Some(t0 + chrono::Duration::seconds(i as i64)),
                        // Re-touch earlier agents as well as adding new ones, so
                        // the fold is not purely insert-only.
                        meta_update(&format!("sub{}", i % (n / 4).max(1))).facts,
                    )
                })
                .collect()
        };

        let load = |app: &mut App| {
            app.handle_ui_event(UiEvent::ReplayLoaded {
                session_id: "s".into(),
                items: items(count),
                speed: 8.0,
                info: Default::default(),
            });
            app.go_live();
        };

        let target = SNAPSHOT_STRIDE * 2 + 5;

        // With the ladder: fold to the edge (taking rungs), then seek back.
        let mut laddered = App::new("s".to_string(), Mode::Replay);
        load(&mut laddered);
        assert!(
            laddered.snapshots.len() >= 3,
            "expected several rungs, got {}",
            laddered.snapshots.len()
        );
        laddered.seek_to_fraction(target as f64 / count as f64);
        let restored_from = laddered.timeline.folded;

        // Without: same App, same seek, but the ladder emptied first so
        // `rebuild_to` has to fold from item zero.
        let mut plain = App::new("s".to_string(), Mode::Replay);
        load(&mut plain);
        plain.snapshots.clear();
        plain.seek_to_fraction(target as f64 / count as f64);

        assert_eq!(
            restored_from, plain.timeline.folded,
            "both paths must land on the same item"
        );
        assert!(
            laddered.session == plain.session,
            "ladder restore diverged from a full rebuild"
        );
    }

    /// Seed a live App with `n` dated items one second apart, folded to the
    /// edge so the ladder has rungs.
    fn app_with_rungs(n: usize, t0: chrono::DateTime<chrono::Utc>) -> App {
        let mut app = App::new("s".to_string(), Mode::Live);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: (0..n)
                .map(|i| {
                    crate::tailer::ReplayItem::at(
                        Some(t0 + chrono::Duration::seconds(i as i64)),
                        meta_update(&format!("sub{i}")).facts,
                    )
                })
                .collect(),
            speed: 8.0,
            info: Default::default(),
        });
        app.go_live();
        assert!(!app.snapshots.is_empty(), "rungs were taken");
        app
    }

    /// An undated item sorts to the HEAD, so one arriving shifts every index
    /// after it: no rung describes its prefix any more, and all must go.
    #[test]
    fn an_undated_arrival_voids_the_whole_ladder() {
        let t0: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        let mut app = app_with_rungs(SNAPSHOT_STRIDE * 2, t0);

        // A meta with no timestamp of its own — the `needs_dating` path.
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![meta_update("late")],
        });
        // Only a rung taken at the new edge remains; every rung describing the
        // old prefix is gone.
        assert!(
            app.snapshots
                .iter()
                .all(|s| s.folded == app.timeline.folded),
            "a rung describing the pre-sort prefix survived: {:?}",
            app.snapshots.iter().map(|s| s.folded).collect::<Vec<_>>()
        );
    }

    /// The case the ladder exists for: a live session whose sidecars land a
    /// moment out of order. The re-sort touches only the tail, so the rungs
    /// below it still describe their prefix exactly — voiding them wholesale
    /// sent every backward seek back to folding from item zero.
    #[test]
    fn an_out_of_order_batch_keeps_the_rungs_below_it() {
        let count = SNAPSHOT_STRIDE * 3;
        let t0: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        let mut app = app_with_rungs(count, t0);
        let before = app.snapshots.len();
        assert!(before >= 2, "expected several rungs, got {before}");

        // A sidecar entry timestamped just before the edge: out of order, but
        // it cannot disturb anything earlier than itself.
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![Record::Entry {
                source: Source::Sub("late".into()),
                entry: crate::provider::claude::wire::parse_line(&format!(
                    r#"{{"type":"assistant","uuid":"u","parentUuid":null,"timestamp":"{}","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"b1","name":"Bash","input":{{}}}}]}}}}"#,
                    (t0 + chrono::Duration::seconds(count as i64 - 2)).to_rfc3339()
                ))
                .unwrap(),
            }
            .statement()
            .unwrap()],
        });

        assert!(
            app.snapshots.len() >= before - 1,
            "the untouched prefix lost its rungs: {} -> {}",
            before,
            app.snapshots.len()
        );
        assert!(
            app.snapshots
                .iter()
                .all(|s| s.generation == app.timeline.generation),
            "a surviving rung kept a stale generation stamp"
        );

        // And a rung is only worth keeping if restoring it is still correct.
        let target = SNAPSHOT_STRIDE + 5;
        let total = app.timeline.items.len();
        app.seek_to_fraction(target as f64 / total as f64);
        let laddered = app.session.clone();
        let landed = app.timeline.folded;

        app.snapshots.clear();
        app.seek_to_fraction(1.0);
        app.snapshots.clear();
        app.seek_to_fraction(target as f64 / total as f64);
        assert_eq!(landed, app.timeline.folded, "both paths land on one item");
        assert!(
            laddered == app.session,
            "a surviving rung diverged from a full rebuild"
        );
    }

    #[test]
    fn seek_forward_then_back_rebuilds_as_of_then() {
        use crate::tailer::ReplayItem;
        let t1: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        let t2: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:10.000Z".parse().unwrap();
        let items = vec![
            ReplayItem::at(Some(t1), meta_update("sub1").facts),
            ReplayItem::at(Some(t2), meta_update("sub2").facts),
        ];

        let mut app = App::new("s".into(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items,
            speed: 8.0,
            info: Default::default(),
        });
        // ReplayLoaded folds the first moment only: sub1 present, sub2 not due.
        assert!(app.session.agent("sub1").is_some());
        assert!(app.session.agent("sub2").is_none());

        // Seek to the end: forward fold brings sub2 in. Select it... then sub1.
        app.seek(t2);
        assert!(app.session.agent("sub2").is_some());
        app.select_agent(Some("sub1".into()));

        // Seek back before sub2 existed: rebuild must drop it AND keep selection.
        app.seek("2026-06-05T10:00:05.000Z".parse().unwrap());
        assert!(app.session.agent("sub1").is_some());
        assert!(
            app.session.agent("sub2").is_none(),
            "backward seek must rebuild to the earlier state (sub2 gone)"
        );
        assert_eq!(
            app.selected_agent_id().as_deref(),
            Some("sub1"),
            "selection must survive the rebuild"
        );
        assert!(
            !app.timeline.follow_head,
            "seeking into the past unpins follow"
        );
    }

    #[test]
    fn seek_prompt_steps_between_eras() {
        use crate::tailer::ReplayItem;
        let ts = |s: &str| s.parse::<chrono::DateTime<chrono::Utc>>().unwrap();
        let prompt = |uuid: &str, t: &str, text: &str| {
            let line = format!(
                r#"{{"type":"user","uuid":"{uuid}","parentUuid":null,"origin":{{"kind":"human"}},"timestamp":"{t}","message":{{"role":"user","content":"{text}"}}}}"#
            );
            ReplayItem::new(
                Record::Entry {
                    source: Source::Main,
                    entry: crate::provider::claude::wire::parse_line(&line).unwrap(),
                }
                .statement()
                .unwrap(),
            )
        };
        let items = vec![
            prompt("p1", "2026-06-05T10:00:00.000Z", "first"),
            prompt("p2", "2026-06-05T11:00:00.000Z", "second"),
            prompt("p3", "2026-06-05T12:00:00.000Z", "third"),
            // Trailing activity after the last era.
            ReplayItem::at(
                Some(ts("2026-06-05T13:00:00.000Z")),
                meta_update("sub1").facts,
            ),
        ];
        let mut app = App::new("s".into(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items,
            speed: 8.0,
            info: Default::default(),
        });
        assert_eq!(app.timeline.cursor, Some(ts("2026-06-05T10:00:00.000Z")));

        // ']' from the start lands on the NEXT era boundary — including ones
        // the playhead hasn't folded yet (the model can't know them).
        app.seek_prompt(true);
        assert_eq!(app.timeline.cursor, Some(ts("2026-06-05T11:00:00.000Z")));
        app.seek_prompt(true);
        assert_eq!(app.timeline.cursor, Some(ts("2026-06-05T12:00:00.000Z")));

        // Past the last prompt → re-pins to the edge.
        app.seek_prompt(true);
        assert_eq!(app.timeline.cursor, Some(ts("2026-06-05T13:00:00.000Z")));
        assert!(app.timeline.follow_head);

        // '[' steps back to the previous era — strictly before the cursor, so
        // sitting exactly on a boundary steps to the era before it.
        app.seek_prompt(false);
        assert_eq!(app.timeline.cursor, Some(ts("2026-06-05T12:00:00.000Z")));
        app.seek_prompt(false);
        assert_eq!(app.timeline.cursor, Some(ts("2026-06-05T11:00:00.000Z")));
        app.seek_prompt(false);
        assert_eq!(app.timeline.cursor, Some(ts("2026-06-05T10:00:00.000Z")));

        // Before the first prompt → clamps to the start (a no-op here).
        app.seek_prompt(false);
        assert_eq!(app.timeline.cursor, Some(ts("2026-06-05T10:00:00.000Z")));
    }

    #[test]
    fn go_live_repins_to_edge_and_folds_to_end() {
        use crate::tailer::ReplayItem;
        let t1: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        let t2: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:10.000Z".parse().unwrap();
        let items = vec![
            ReplayItem::at(Some(t1), meta_update("sub1").facts),
            ReplayItem::at(Some(t2), meta_update("sub2").facts),
        ];
        let mut app = App::new("s".into(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items,
            speed: 8.0,
            info: Default::default(),
        });

        // Scrub back into history, then "go live": re-pins and folds to the edge.
        app.seek(t2);
        app.seek("2026-06-05T10:00:05.000Z".parse().unwrap());
        assert!(!app.timeline.follow_head);

        app.go_live();
        assert!(app.timeline.follow_head, "go live re-pins to the edge");
        assert!(
            app.session.agent("sub2").is_some(),
            "go live folds forward to the head"
        );
    }

    #[test]
    fn transport_is_emergent_not_a_mode() {
        use crate::tailer::ReplayItem;
        let t1: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        let t2: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:10.000Z".parse().unwrap();
        let mut app = App::new("s".into(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: vec![
                ReplayItem::at(Some(t1), meta_update("sub1").facts),
                ReplayItem::at(Some(t2), meta_update("sub2").facts),
            ],
            speed: 8.0,
            info: Default::default(),
        });

        // Paced, playing forward, not yet at the edge → Playing.
        assert_eq!(app.transport(), Transport::Playing);

        // At the edge with no fresh append → Idle (a finished/quiet session),
        // NOT "Live" just because it was opened as replay.
        app.seek(t2);
        assert_eq!(app.transport(), Transport::Idle);

        // A genuine append lands at the edge (the file resumed): even a paced
        // replay now reads Live — "live" is following + fresh, not a mode.
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![meta_update("sub3")],
        });
        assert_eq!(app.transport(), Transport::Live);
        assert!(
            app.session.agent("sub3").is_some(),
            "a resumed replay folds the new append"
        );

        // Scrub back off the edge → History.
        app.seek(t1);
        assert_eq!(app.transport(), Transport::History);
    }

    #[test]
    fn space_pauses_and_resumes_from_the_current_cursor() {
        use crate::tailer::ReplayItem;
        let t1: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        let t2: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:10.000Z".parse().unwrap();
        let mut app = App::new("s".into(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: vec![
                ReplayItem::at(Some(t1), meta_update("sub1").facts),
                ReplayItem::at(Some(t2), meta_update("sub2").facts),
            ],
            speed: 8.0,
            info: Default::default(),
        });

        // Scrub into the past → stopped (History).
        app.seek("2026-06-05T10:00:05.000Z".parse().unwrap());
        assert!(!app.timeline.follow_head);

        // Space resumes from HERE (re-engages follow_head, does not jump to edge).
        app.toggle_play_pause();
        assert!(app.timeline.follow_head);
        assert!(!app.is_paused);
        assert_eq!(app.transport(), Transport::Playing);
        assert!(
            app.session.agent("sub2").is_none(),
            "resume plays forward from the cursor, not a jump to the live edge"
        );

        // Space again freezes in place.
        app.toggle_play_pause();
        assert!(app.is_paused);
        assert_eq!(app.transport(), Transport::Paused);
    }

    #[test]
    fn seeking_to_the_edge_clears_pause() {
        use crate::tailer::ReplayItem;
        let t1: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        let t2: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:10.000Z".parse().unwrap();
        let mut app = App::new("s".into(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: vec![
                ReplayItem::at(Some(t1), meta_update("sub1").facts),
                ReplayItem::at(Some(t2), meta_update("sub2").facts),
            ],
            speed: 8.0,
            info: Default::default(),
        });

        // Paused, then drag to the live edge: pause must clear so the playhead
        // rides the edge instead of freezing there.
        app.is_paused = true;
        app.seek(t2);
        assert!(app.timeline.follow_head, "landed at the edge");
        assert!(!app.is_paused, "seeking to the edge clears pause");
        assert_ne!(app.transport(), Transport::Paused, "not frozen at live");
        assert!(
            app.session.agent("sub2").is_some(),
            "seek to the edge folds the whole stream"
        );
    }

    #[test]
    fn transport_paused_outranks_parked_in_the_past() {
        use crate::tailer::ReplayItem;
        let t1: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:00.000Z".parse().unwrap();
        let t2: chrono::DateTime<chrono::Utc> = "2026-06-05T10:00:10.000Z".parse().unwrap();
        let mut app = App::new("s".into(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: vec![
                ReplayItem::at(Some(t1), meta_update("sub1").facts),
                ReplayItem::at(Some(t2), meta_update("sub2").facts),
            ],
            speed: 8.0,
            info: Default::default(),
        });

        // Scrubbed back off the edge → History.
        app.seek(t1);
        assert_eq!(app.transport(), Transport::History);

        // A deliberate pause outranks "parked in the past" — `transport` checks
        // `is_paused` before `follow_head`, so a paused-and-scrubbed view reads
        // Paused, not History. (Pause parks the cursor, so both flags are set.)
        app.is_paused = true;
        assert_eq!(app.transport(), Transport::Paused);
    }

    #[test]
    fn pausing_at_the_live_edge_buffers_appends_instead_of_snapping() {
        let e = |ts: &str| {
            Record::Entry {
            source: Source::Main,
            entry: crate::provider::claude::wire::parse_line(&format!(
                r#"{{"type":"user","uuid":"u","timestamp":"{ts}","message":{{"role":"user","content":"x"}}}}"#
            ))
            .unwrap(),
        }
        .statement()
        .unwrap()
        };
        let mut app = App::new("s".into(), Mode::Live);
        app.handle_ui_event(UiEvent::SessionReset {
            session_id: "s".into(),
        });
        // Ride the live edge at t0.
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![e("2026-06-05T10:00:00.000Z")],
        });
        assert!(app.timeline.follow_head, "live session follows the edge");
        let cursor_at_pause = app.timeline.cursor;
        let folded_at_pause = app.timeline.folded;

        // Pause AT the edge → parks (drops follow_head) so the view holds still.
        app.toggle_play_pause();
        assert!(app.is_paused);
        assert!(
            !app.timeline.follow_head,
            "pause parks the cursor at the edge"
        );
        assert_eq!(app.transport(), Transport::Paused);

        // A new live event lands while paused: it must BUFFER (extend the
        // timeline) without moving the parked cursor or folding into the view —
        // the whole point of live-pause. (Previously it snapped the playhead.)
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![e("2026-06-05T10:00:10.000Z")],
        });
        assert_eq!(
            app.timeline.cursor, cursor_at_pause,
            "paused view holds still"
        );
        assert_eq!(
            app.timeline.folded, folded_at_pause,
            "the new event is buffered, not folded"
        );
        assert!(
            app.timeline.items.len() > app.timeline.folded,
            "and it IS buffered behind the edge"
        );
        assert_eq!(app.transport(), Transport::Paused);

        // Resume → re-engages the edge and catches up through the buffered gap.
        app.toggle_play_pause();
        assert!(app.timeline.follow_head);
        assert!(!app.is_paused);
        app.tick_timeline(std::time::Duration::from_secs(30));
        assert_eq!(
            app.timeline.folded,
            app.timeline.items.len(),
            "resume catches up through what buffered while paused"
        );
    }

    #[test]
    fn out_of_order_live_batch_folds_all_and_keeps_items_sorted() {
        let mut app = App::new("s".into(), Mode::Live);
        app.handle_ui_event(UiEvent::SessionReset {
            session_id: "s".into(),
        });
        let e = |ts: &str| {
            Record::Entry {
            source: Source::Main,
            entry: crate::provider::claude::wire::parse_line(&format!(
                r#"{{"type":"user","uuid":"u","timestamp":"{ts}","message":{{"role":"user","content":"x"}}}}"#
            ))
            .unwrap(),
        }
        .statement()
        .unwrap()
        };
        // An out-of-order batch (file-grouped arrival / backfilled block).
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![
                e("2026-06-05T10:00:05.000Z"),
                e("2026-06-05T10:00:01.000Z"),
                e("2026-06-05T10:00:03.000Z"),
            ],
        });

        // Following the live edge → the whole (order-independent) batch is folded.
        assert!(app.timeline.folded > 0);
        assert_eq!(app.timeline.folded, app.timeline.items.len());
        // And `items` is ts-sorted, so a later backward seek folds the right prefix.
        let got: Vec<_> = app.timeline.items.iter().filter_map(|i| i.ts()).collect();
        let mut want = got.clone();
        want.sort();
        assert_eq!(got, want);
    }

    // Folding done subagents. The omp fixture played to its end has `main`
    // with `Reviewer` and `__sidecar` done and `Helper` stopped.

    /// The rows as plain strings: an agent's id, or `fold:<parent>[members]`.
    fn row_keys(app: &App) -> Vec<String> {
        let folds = app.folds();
        view::rows(&app.session, &folds)
            .into_iter()
            .map(|row| match row {
                Row::Agent { id, .. } => id.to_string(),
                Row::Folded {
                    parent, members, ..
                } => format!("fold:{parent}[{}]", members.join(",")),
            })
            .collect()
    }

    /// The graph holds exactly one node per row: each shown agent, and one
    /// card per fold.
    fn assert_graph_matches_rows(app: &App) {
        let folds = app.folds();
        let want: std::collections::BTreeSet<String> = view::rows(&app.session, &folds)
            .into_iter()
            .map(|row| match row {
                Row::Agent { id, .. } => id.to_string(),
                Row::Folded { parent, .. } => fold_card_id(parent),
            })
            .collect();
        let have: std::collections::BTreeSet<String> =
            app.flow.nodes().map(|n| n.id.clone()).collect();
        assert_eq!(have, want);
    }

    fn fold_fixture() -> Option<App> {
        #[cfg(feature = "native")]
        let app = crate::ui::snapshots::fixture_app();
        #[cfg(not(feature = "native"))]
        let app = None;
        app
    }

    #[test]
    fn done_siblings_fold_in_rows() {
        let Some(app) = fold_fixture() else {
            return;
        };
        assert_eq!(
            row_keys(&app),
            ["main", "fold:main[Reviewer,__sidecar]", "Helper"]
        );
        assert_graph_matches_rows(&app);
    }

    /// The selection never decides what folds, so a lane click or a seek
    /// leaves the graph and the Now rows in agreement.
    #[test]
    fn done_siblings_fold_graph_matches_now() {
        let Some(mut app) = fold_fixture() else {
            return;
        };
        app.set_view(View::Lanes);
        app.select_agent(Some("Reviewer".into()));
        app.reconcile_folds();
        assert_eq!(app.selected_agent_id().as_deref(), Some("Reviewer"));
        assert!(row_keys(&app).contains(&"fold:main[Reviewer,__sidecar]".to_string()));
        assert_graph_matches_rows(&app);

        app.seek_to_fraction(0.0);
        app.reconcile_folds();
        assert_graph_matches_rows(&app);
    }

    /// A live subagent called done only because it went quiet can resume,
    /// so it never folds.
    #[test]
    fn done_siblings_fold_skips_live_heuristic_done() {
        let old = chrono::Utc::now() - chrono::Duration::seconds(600);
        let agent = |id: &str, parent: Option<&str>| Fact {
            agent: Some(id.into()),
            ts: Some(old),
            kind: FactKind::Agent {
                kind: if parent.is_some() {
                    crate::fact::AgentKind::Subagent
                } else {
                    crate::fact::AgentKind::Main
                },
                parent: parent.map(str::to_string),
                agent_type: Some(id.into()),
                description: None,
                spawned_by: None,
                interactive: parent.is_none(),
            },
        };
        let mut app = App::new("s".into(), Mode::Live);
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![Statement {
                at: Some(old),
                facts: vec![
                    agent(session::MAIN_ID, None),
                    agent("a", Some(session::MAIN_ID)),
                    agent("b", Some(session::MAIN_ID)),
                ],
            }],
        });
        for id in ["a", "b"] {
            assert_eq!(
                app.session.agent(id).unwrap().status,
                session::AgentStatus::Done,
                "{id} went quiet"
            );
        }
        assert_eq!(row_keys(&app), [session::MAIN_ID, "a", "b"]);
        assert_graph_matches_rows(&app);
    }

    /// `settled` is derived, not latched: back to the start and to the end
    /// again, the fold returns.
    #[test]
    fn done_siblings_fold_survives_seek_back_and_end() {
        let Some(mut app) = fold_fixture() else {
            return;
        };
        app.seek_to_fraction(0.0);
        app.go_live();
        app.tick_timeline(std::time::Duration::ZERO);
        assert!(row_keys(&app).contains(&"fold:main[Reviewer,__sidecar]".to_string()));
        assert_graph_matches_rows(&app);
    }

    /// main → {A, B} done, A → {C1, C2} done, replayed to its end.
    fn nested_tree_app() -> App {
        let at = chrono::DateTime::from_timestamp(1_780_000_000, 0).unwrap();
        let fact = |id: &str, kind: FactKind| Fact {
            agent: Some(id.into()),
            ts: Some(at),
            kind,
        };
        let agent = |id: &str, parent: &str| {
            fact(
                id,
                FactKind::Agent {
                    kind: crate::fact::AgentKind::Subagent,
                    parent: Some(parent.into()),
                    agent_type: Some(id.into()),
                    description: None,
                    spawned_by: None,
                    interactive: false,
                },
            )
        };
        let main = session::MAIN_ID;
        let mut facts = vec![fact(
            main,
            FactKind::Agent {
                kind: crate::fact::AgentKind::Main,
                parent: None,
                agent_type: Some("main".into()),
                description: None,
                spawned_by: None,
                interactive: true,
            },
        )];
        for (id, parent) in [("A", main), ("B", main), ("C1", "A"), ("C2", "A")] {
            facts.push(agent(id, parent));
            facts.push(fact(id, FactKind::Ended(crate::fact::AgentStatus::Done)));
        }
        let mut app = App::new("s".into(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: vec![crate::tailer::ReplayItem::new(Statement {
                at: Some(at),
                facts,
            })],
            speed: 8.0,
            info: Default::default(),
        });
        app.go_live();
        app.tick_timeline(std::time::Duration::ZERO);
        app
    }

    /// The hidden A starts no fold of its own, a selected C1 shows as main's
    /// fold, and with both main and A unfolded `esc` on C1 closes A's fold
    /// only — whatever order the expanded parents sit in, so each round uses
    /// a fresh app (and a fresh hash seed).
    #[test]
    fn done_siblings_fold_nested_tree() {
        let main = session::MAIN_ID;
        for _ in 0..32 {
            let mut app = nested_tree_app();
            assert_eq!(row_keys(&app), [main, "fold:main[A,B]"]);
            assert_graph_matches_rows(&app);
            app.selection.agent = Some("C1".into());
            app.normalize_fold_selection();
            assert_eq!(app.selection.folded.as_deref(), Some(main));
            assert_eq!(app.selection.agent, None);

            app.expand_fold();
            assert_eq!(row_keys(&app), [main, "A", "fold:A[C1,C2]", "B"]);
            assert_graph_matches_rows(&app);

            app.select_fold("A".into());
            app.expand_fold();
            assert_eq!(app.selected_agent_id().as_deref(), Some("C1"));
            assert!(app.collapse_fold());
            assert_eq!(app.selection.folded.as_deref(), Some("A"));
            assert!(app.expanded_folds.contains(main));
            assert!(!app.expanded_folds.contains("A"));
        }
    }

    /// A subagent open full-size that then folds away closes its detail:
    /// otherwise an invisible detail keeps `j`/`k` scrolling nothing.
    #[test]
    fn folding_the_detailed_agent_closes_the_detail() {
        let Some(mut app) = fold_fixture() else {
            return;
        };
        let before_end = app.timeline.head_ts().unwrap() - chrono::Duration::seconds(1);
        app.seek(before_end);
        app.select_agent(Some("Reviewer".into()));
        app.selection.detail = true;
        app.go_live();
        app.tick_timeline(std::time::Duration::ZERO);
        app.normalize_fold_selection();
        assert_eq!(app.selection.folded.as_deref(), Some(session::MAIN_ID));
        assert!(!app.selection.detail);
    }

    /// Siblings already done for good when a live session is first read fold
    /// before any of them reached the canvas; the card still lands below its
    /// parent, not on top of it.
    #[test]
    fn a_fold_formed_on_first_load_sits_below_its_parent() {
        let at = chrono::Utc::now() - chrono::Duration::seconds(5);
        let fact = |id: &str, kind: FactKind| Fact {
            agent: Some(id.into()),
            ts: Some(at),
            kind,
        };
        let agent = |id: &str, main: bool| {
            fact(
                id,
                FactKind::Agent {
                    kind: if main {
                        crate::fact::AgentKind::Main
                    } else {
                        crate::fact::AgentKind::Subagent
                    },
                    parent: (!main).then(|| session::MAIN_ID.to_string()),
                    agent_type: Some(id.into()),
                    description: None,
                    spawned_by: None,
                    interactive: main,
                },
            )
        };
        let mut facts = vec![agent(session::MAIN_ID, true)];
        for id in ["a", "b"] {
            facts.push(agent(id, false));
            facts.push(fact(id, FactKind::Ended(crate::fact::AgentStatus::Done)));
        }
        let mut app = App::new("s".into(), Mode::Live);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: vec![crate::tailer::ReplayItem::new(Statement {
                at: Some(at),
                facts,
            })],
            speed: 8.0,
            info: Default::default(),
        });
        assert_eq!(row_keys(&app), [session::MAIN_ID, "fold:main[a,b]"]);
        let main = app.flow.node(session::MAIN_ID).unwrap();
        let card = app.flow.node(&fold_card_id(session::MAIN_ID)).unwrap();
        assert!(
            card.position.y >= main.position.y + main.height,
            "{:?} under {:?}",
            card.position,
            main.position
        );
    }

    #[test]
    fn done_siblings_fold_reset_on_session_switch() {
        let Some(mut app) = fold_fixture() else {
            return;
        };
        assert!(!app.parked_positions.is_empty(), "the members' places");
        app.expanded_folds.insert(session::MAIN_ID.into());
        app.select_fold(session::MAIN_ID.into());
        app.handle_ui_event(UiEvent::SessionReset {
            session_id: "other".into(),
        });
        assert!(app.expanded_folds.is_empty());
        assert!(app.parked_positions.is_empty());
        assert_eq!(app.selection.folded, None);
    }

    /// Paused at the end, the file grows: the replay is no longer settled,
    /// and nothing but the loop's reconcile re-syncs the graph.
    #[test]
    fn done_siblings_fold_follows_growth_while_paused_at_end() {
        let Some(mut app) = fold_fixture() else {
            return;
        };
        app.toggle_play_pause();
        let later = app.timeline.head_ts().unwrap() + chrono::Duration::seconds(10);
        app.handle_ui_event(UiEvent::Batch {
            session_id: app.current_session_id.clone(),
            statements: vec![Statement {
                at: Some(later),
                facts: vec![Fact {
                    agent: Some(session::MAIN_ID.into()),
                    ts: Some(later),
                    kind: FactKind::Activity,
                }],
            }],
        });
        assert!(!app.settled());
        assert!(app.reconcile_folds());
        assert_graph_matches_rows(&app);
    }

    /// A card the user dragged keeps its place while a fold takes it off the
    /// canvas and a seek brings it back; the fold's card sits where its first
    /// member stood.
    #[test]
    fn fold_keeps_dragged_positions_across_seek() {
        use rataflow::types::Position;
        let Some(mut app) = fold_fixture() else {
            return;
        };
        assert_eq!(app.camera, Camera::Overview);
        let before_end = app.timeline.head_ts().unwrap() - chrono::Duration::seconds(1);
        app.seek(before_end);
        assert!(
            app.flow.node("Reviewer").is_some(),
            "unfolded before the end"
        );
        let dragged = Position::new(300.0, 120.0);
        let aside = Position::new(-50.0, 80.0);
        app.flow.set_node_position("Reviewer", dragged);
        app.flow.set_node_position("Helper", aside);

        app.go_live();
        app.tick_timeline(std::time::Duration::ZERO);
        assert!(app.flow.node("Reviewer").is_none(), "folded at the end");
        let card = app.flow.node(&fold_card_id(session::MAIN_ID)).unwrap();
        assert_eq!(card.position, dragged);

        app.seek(before_end);
        assert_eq!(app.flow.node("Reviewer").unwrap().position, dragged);
        assert_eq!(app.flow.node("Helper").unwrap().position, aside);
    }
}
