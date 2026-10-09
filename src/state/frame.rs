//! Whether the frame on screen is stale.
//!
//! The terminal loop ticks every 16 ms so input and motion stay smooth, but a
//! quiet session would otherwise repaint the same cells sixty times a second.
//! [`RedrawGate`] lets a tick draw only when something visible can have
//! changed: an event arrived ([`RedrawGate::mark`]: input, a tailer batch, a
//! resize, a view switch, a status flip), the app is in motion
//! ([`App::animating`]), or a time-driven part of the picture moved on — what
//! [`FrameStamp`] captures.
//!
//! Every element that changes with time alone has a field here, at the
//! granularity it is drawn at; leaving one out makes the screen look frozen.

use chrono::{DateTime, Utc};
use web_time::{Duration, Instant};

use super::{App, Transport, View};

/// Marching ants keep moving this long after the last append, then hold
/// still: a session that has gone quiet stops costing frames.
const ANTS_MOVE_FOR: Duration = Duration::from_secs(30);

/// Every part of a frame that changes with time alone, as of one tick. Two
/// equal stamps paint the same cells, so the second frame can be skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameStamp {
    /// The status badge (`Live` turns `Idle` on its own).
    transport: Transport,
    view: View,
    /// The marching-ants phase, while the graph is shown, an edge is
    /// animated, and ants move.
    ants: Option<u64>,
    /// Whole seconds every in-flight tool has run, summed: each row's timer
    /// ticks once a second, and every one only grows, so the sum changes
    /// exactly when some timer's text does.
    running: i64,
    /// How many in-flight tools are slow (the hint bar count; see
    /// [`SessionModel::slow_tools`](super::session::SessionModel::slow_tools)).
    slow: usize,
    /// The top bar's elapsed time, in whole seconds (also the spinner phase).
    elapsed: Option<i64>,
    /// The lane view's live edge, in columns, while it rides the edge.
    lane_edge: Option<i64>,
    /// Whether a snackbar is up.
    snackbar: bool,
    /// Whether a status flash is up.
    flash: bool,
}

impl App {
    /// The stamp of a frame drawn at `now`. `wall` is the time on-screen
    /// timers measure against (the timeline's `now_reference`).
    pub fn frame_stamp(&self, now: Instant, wall: Option<DateTime<Utc>>) -> FrameStamp {
        let mut running = 0i64;
        let mut slow = 0usize;
        if let Some(wall) = wall {
            for (_, call) in self.session.in_flight() {
                if let Some(start) = call.ts {
                    let ran = (wall - start).max(chrono::Duration::zero());
                    running = running.saturating_add(ran.num_seconds());
                }
            }
            slow = self.session.slow_tools(wall);
        }
        let lane_edge = match (&self.hit.lane_axis, wall) {
            (Some(axis), Some(wall)) if self.view == View::Lanes && self.timeline.follow_head => {
                Some(wall.timestamp_millis() / axis.ms_per_col.max(1))
            }
            _ => None,
        };
        FrameStamp {
            transport: self.transport_at(now),
            view: self.view,
            ants: self.ants_phase(now),
            running,
            slow,
            elapsed: self.session.elapsed(wall).map(|d| d.num_seconds()),
            lane_edge,
            snackbar: self.snackbar.as_ref().is_some_and(|s| s.until > now),
            flash: self.flash.as_ref().is_some_and(|f| f.until > now),
        }
    }

    /// Whether every tick shows something new: a camera glide on the graph,
    /// or a playhead playing forward through the timeline.
    pub fn animating(&self, now: Instant) -> bool {
        (self.view == View::Graph && self.camera_glide.is_some())
            || self.transport_at(now) == Transport::Playing
    }

    /// Whether the picture moves on its own between ticks — playback, a
    /// glide, marching ants, a status flash — so the loop needs its full
    /// frame rate. Otherwise the only time-driven changes are whole-second
    /// timers and multi-second deadlines, and a slow tick shows them all.
    pub fn in_motion(&self, now: Instant) -> bool {
        self.animating(now)
            || self.ants_phase(now).is_some()
            || self.flash.as_ref().is_some_and(|f| f.until > now)
    }

    /// The phase rataflow draws animated edges at — `None` off the graph view,
    /// when no edge is animated, or once the ants have been still for
    /// [`ANTS_MOVE_FOR`].
    fn ants_phase(&self, now: Instant) -> Option<u64> {
        if self.view != View::Graph {
            return None;
        }
        let moving = self
            .last_batch_at
            .is_some_and(|t| now.saturating_duration_since(t) < ANTS_MOVE_FOR);
        (moving && self.flow.edges().iter().any(|e| e.animated))
            .then(|| self.ants_ms / self.flow.animation_speed_ms.max(1))
    }
}

/// Decides, once per tick, whether to draw.
#[derive(Debug, Default)]
pub struct RedrawGate {
    drawn: Option<FrameStamp>,
    dirty: bool,
}

impl RedrawGate {
    /// Something the stamp does not cover changed: input, a tailer event, a
    /// resize, a status flip. The next tick draws.
    pub fn mark(&mut self) {
        self.dirty = true;
    }

    /// Whether this tick draws. `moving` is motion the app does not track
    /// itself: an auto-pan or drag in the flow.
    pub fn due(
        &mut self,
        app: &App,
        now: Instant,
        wall: Option<DateTime<Utc>>,
        moving: bool,
    ) -> bool {
        let stamp = app.frame_stamp(now, wall);
        let due = self.dirty || moving || app.animating(now) || self.drawn.as_ref() != Some(&stamp);
        if due {
            self.dirty = false;
            self.drawn = Some(stamp);
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fact::{AgentKind, Fact, FactKind, Statement};
    use crate::state::session::{AgentStatus, MAIN_ID};
    use crate::state::{CameraGlide, LaneAxis, Mode};
    use crate::tailer::{ReplayItem, UiEvent};

    const TICK: Duration = Duration::from_millis(16);

    fn born(id: &str, parent: Option<&str>) -> FactKind {
        FactKind::Agent {
            kind: if parent.is_some() {
                AgentKind::Subagent
            } else {
                AgentKind::Main
            },
            parent: parent.map(str::to_string),
            agent_type: Some(id.into()),
            description: None,
            spawned_by: None,
            interactive: parent.is_none(),
        }
    }

    fn says(id: &str, at: DateTime<Utc>, kinds: Vec<FactKind>) -> Statement {
        let parent = (id != MAIN_ID).then_some(MAIN_ID);
        let facts = std::iter::once(born(id, parent))
            .chain(kinds)
            .map(|kind| Fact {
                agent: Some(id.to_string()),
                ts: Some(at),
                kind,
            })
            .collect();
        Statement {
            at: Some(at),
            facts,
        }
    }

    fn pending(call: &str) -> FactKind {
        FactKind::ToolStart {
            call: call.into(),
            name: "bash".into(),
            summary: None,
            intent: None,
        }
    }

    /// A finished session opened at its edge: nothing moves on its own.
    fn finished() -> App {
        let at: DateTime<Utc> = "2026-06-05T10:00:00Z".parse().unwrap();
        let mut app = App::new("s".into(), Mode::Live);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: vec![ReplayItem::new(says(MAIN_ID, at, vec![FactKind::Activity]))],
            speed: 8.0,
            info: Default::default(),
        });
        app.go_live();
        app.tick_timeline(Duration::ZERO);
        app
    }

    /// A live session whose last append just arrived.
    fn live(statements: Vec<Statement>) -> App {
        let mut app = App::new("s".into(), Mode::Live);
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements,
        });
        app.tick_timeline(Duration::ZERO);
        app
    }

    /// Run the loop's per-tick work `ticks` times from `start`, the wall
    /// clock advancing in step from `wall`; returns how many ticks drew.
    fn run(
        app: &mut App,
        gate: &mut RedrawGate,
        start: Instant,
        wall: Option<DateTime<Utc>>,
        ticks: u32,
    ) -> u32 {
        let mut drawn = 0;
        for k in 1..=ticks {
            let now = start + TICK * k;
            app.tick_clock(now);
            let moving = app.tick_auto_pan(TICK);
            app.tick_animation(TICK);
            if app.tick_camera(TICK) {
                gate.mark();
            }
            app.tick_timeline(TICK);
            let wall = wall.map(|w| w + chrono::Duration::milliseconds(16 * i64::from(k)));
            drawn += u32::from(gate.due(app, now, wall, moving));
        }
        drawn
    }

    #[test]
    fn draw_skipped_when_clean() {
        let mut app = finished();
        let mut gate = RedrawGate::default();
        let start = Instant::now();
        assert!(gate.due(&app, start, None, false), "the first frame draws");
        assert_eq!(run(&mut app, &mut gate, start, None, 120), 0);
    }

    /// A 50-cell pan, from its start.
    const GLIDE: CameraGlide = CameraGlide {
        from: (0.0, 0.0),
        to: (50.0, 0.0),
        t: 0.0,
    };

    /// A glide draws every tick where it shows — on the graph — and costs no
    /// frame anywhere else.
    #[test]
    fn a_glide_draws_every_tick_only_on_the_graph() {
        for (view, frames) in [(View::Graph, 19), (View::Now, 0), (View::Lanes, 0)] {
            let mut app = finished();
            app.set_view(view);
            let mut gate = RedrawGate::default();
            let start = Instant::now();
            gate.due(&app, start, None, false);
            app.camera_glide = Some(GLIDE);
            // A 0.3 s glide is 18 whole ticks plus the landing.
            assert_eq!(
                run(&mut app, &mut gate, start, None, 19),
                frames,
                "{view:?}"
            );
            assert!(app.camera_glide.is_none());
            assert_eq!(run(&mut app, &mut gate, start, None, 30), 0);
        }
    }

    #[cfg(feature = "native")]
    #[test]
    fn draw_every_tick_while_auto_panning() {
        use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        let mouse = |kind, column, row| {
            Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers: KeyModifiers::NONE,
            })
        };
        let mut app = finished();
        app.set_view(View::Graph);
        // The loop owns the theme; any one will do to give the canvas a size.
        let theme = crate::ui::seed::theme::Theme::new(
            crate::ui::seed::theme::Mode::Dark,
            crate::ui::seed::theme::Depth::TrueColor,
        );
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|f| crate::ui::draw(f, &mut app, &theme))
            .unwrap();
        let mut gate = RedrawGate::default();
        let start = Instant::now();
        gate.due(&app, start, None, false);

        // Pressing empty canvas and dragging pans the view.
        crate::handler::handle_event(
            &mouse(MouseEventKind::Down(MouseButton::Left), 70, 3),
            &mut app,
        );
        crate::handler::handle_event(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 66, 4),
            &mut app,
        );
        assert!(app.flow.is_dragging());
        assert_eq!(run(&mut app, &mut gate, start, None, 20), 20);

        crate::handler::handle_event(
            &mouse(MouseEventKind::Up(MouseButton::Left), 66, 4),
            &mut app,
        );
        gate.due(&app, start, None, false);
        assert_eq!(run(&mut app, &mut gate, start, None, 20), 0);
    }

    #[test]
    fn draw_once_when_live_expires() {
        let wall = Utc::now();
        let mut app = live(vec![says(MAIN_ID, wall, vec![FactKind::Activity])]);
        let batch = Instant::now();
        let mut gate = RedrawGate::default();
        assert!(gate.due(&app, batch, None, false));
        assert_eq!(app.transport_at(batch), Transport::Live);

        // Ticks spanning the moment the append stops being fresh: one draw,
        // for the badge turning Idle.
        let before = batch + Duration::from_millis(9_900);
        assert_eq!(run(&mut app, &mut gate, before, None, 25), 1);
        assert_eq!(app.transport_at(before + TICK * 25), Transport::Idle);
    }

    /// Every in-flight tool shows its running time, wherever its row is. A
    /// child's long tool under a quiet root must keep ticking on screen, once
    /// a second, with no selection anywhere.
    #[test]
    fn running_tool_elapsed_redraws_once_per_second() {
        let wall = Utc::now();
        let mut app = live(vec![
            says(
                MAIN_ID,
                wall - chrono::Duration::seconds(600),
                vec![FactKind::Activity],
            ),
            says(
                "task",
                wall - chrono::Duration::seconds(60),
                vec![pending("t1")],
            ),
        ]);
        app.status_tick();
        assert_ne!(
            app.session.agent(MAIN_ID).unwrap().status,
            AgentStatus::Running
        );
        assert!(app.selected_agent_id().is_none());
        let mut gate = RedrawGate::default();
        let start = Instant::now();
        assert!(gate.due(&app, start, Some(wall), false));
        let drawn = run(&mut app, &mut gate, start, Some(wall), 63);
        assert!((1..=2).contains(&drawn), "{drawn} draws in one second");
        let drawn = run(
            &mut app,
            &mut gate,
            start + TICK * 63,
            Some(wall + chrono::Duration::milliseconds(1008)),
            125,
        );
        assert!((2..=3).contains(&drawn), "{drawn} draws in two seconds");
    }

    #[test]
    fn pending_tool_count_redraws_at_30s() {
        let wall = Utc::now();
        let app = live(vec![says(
            "task",
            wall - chrono::Duration::milliseconds(29_500),
            vec![pending("t1")],
        )]);
        let now = Instant::now();
        let before = app.frame_stamp(now, Some(wall));
        let after = app.frame_stamp(now, Some(wall + chrono::Duration::seconds(1)));
        assert_eq!((before.slow, after.slow), (0, 1));
    }

    /// A main agent that is Running but has nothing in flight changes with
    /// time only through the top bar's elapsed seconds.
    #[test]
    fn running_without_pending_draws_at_most_once_per_second() {
        let wall = Utc::now();
        let mut app = live(vec![says(
            MAIN_ID,
            wall - chrono::Duration::seconds(40),
            vec![FactKind::Activity],
        )]);
        let mut gate = RedrawGate::default();
        let start = Instant::now();
        gate.due(&app, start, Some(wall), false);
        assert!(run(&mut app, &mut gate, start, Some(wall), 63) <= 1);
    }

    #[test]
    fn lane_live_edge_redraws_per_column() {
        let mut app = finished();
        app.set_view(View::Lanes);
        assert!(app.timeline.follow_head);
        let wall = Utc::now();
        app.hit.lane_axis = Some(LaneAxis {
            area: ratatui::layout::Rect::new(0, 0, 10, 4),
            columns: Vec::new(),
            ms_per_col: 500,
        });
        let mut gate = RedrawGate::default();
        let start = Instant::now();
        gate.due(&app, start, Some(wall), false);
        // One second of wall time is two 500 ms columns.
        let drawn = run(&mut app, &mut gate, start, Some(wall), 63);
        assert!((1..=3).contains(&drawn), "{drawn} draws");
        // Off the edge, the axis holds still.
        app.toggle_play_pause();
        gate.due(&app, start, Some(wall), false);
        assert_eq!(run(&mut app, &mut gate, start, Some(wall), 63), 0);
    }

    #[test]
    fn snackbar_expiry_redraws_once() {
        let mut app = finished();
        let start = app.clock;
        app.snack("Back to live");
        let mut gate = RedrawGate::default();
        assert!(gate.due(&app, start, None, false));
        let near_end = start + crate::state::SNACK_FOR - Duration::from_millis(80);
        assert_eq!(run(&mut app, &mut gate, near_end, None, 10), 1);
        assert!(app.snackbar.is_none(), "the clock retired it");
    }

    #[test]
    fn view_switch_marks_dirty() {
        let mut app = finished();
        let mut gate = RedrawGate::default();
        let start = Instant::now();
        gate.due(&app, start, None, false);
        assert!(!gate.due(&app, start, None, false));
        app.set_view(View::Lanes);
        assert!(
            gate.due(&app, start, None, false),
            "a new view is a new frame"
        );
        assert!(!gate.due(&app, start, None, false));
    }

    /// The loop ticks slowly unless the picture moves on its own.
    #[test]
    fn in_motion_only_while_something_moves() {
        let mut app = finished();
        let now = app.clock;
        assert!(!app.in_motion(now), "a finished session holds still");

        // A glide moves the picture only where it is drawn.
        app.camera_glide = Some(GLIDE);
        assert!(!app.in_motion(now));
        app.set_view(View::Graph);
        assert!(app.in_motion(now));
        app.camera_glide = None;
        app.set_view(View::Now);

        // A status flash, until it is over.
        app.flash = Some(crate::state::Flash {
            ids: vec![MAIN_ID.into()],
            until: now + crate::state::FLASH_FOR,
        });
        assert!(app.in_motion(now));
        assert!(!app.in_motion(now + crate::state::FLASH_FOR));
        app.flash = None;

        // Playback behind the edge.
        app.seek(app.timeline.start_ts().unwrap() - chrono::Duration::seconds(1));
        app.toggle_play_pause();
        assert_eq!(app.transport_at(now), Transport::Playing);
        assert!(app.in_motion(now));
    }
}
