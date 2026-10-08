//! Whether the frame on screen is stale.
//!
//! The terminal loop ticks every 16 ms so input and motion stay smooth, but a
//! quiet session would otherwise repaint the same cells sixty times a second.
//! [`RedrawGate`] lets a tick draw only when something visible can have
//! changed: an event arrived ([`RedrawGate::mark`]), the app is in motion
//! ([`App::animating`]), or a time-driven part of the picture moved on — what
//! [`FrameStamp`] captures.

use chrono::{DateTime, Utc};
use web_time::{Duration, Instant};

use super::{App, Transport};

/// Marching ants keep moving this long after the last append, then hold
/// still: a session that has gone quiet stops costing frames.
const ANTS_MOVE_FOR: Duration = Duration::from_secs(30);

/// Every part of a frame that changes with time alone, as of one tick. Two
/// equal stamps paint the same cells, so the second frame can be skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameStamp {
    /// The status badge and scrubber tag (`Live` turns `Idle` on its own).
    transport: Transport,
    /// The marching-ants phase while any edge is animated and ants move.
    ants: Option<u64>,
    /// Which chips show, their fade bands, and their duration labels.
    chips: u64,
}

impl App {
    /// The stamp of a frame drawn at `now`. `wall` is the time chips measure a
    /// pending tool against (the timeline's `now_reference`).
    pub fn frame_stamp(&self, now: Instant, wall: Option<DateTime<Utc>>) -> FrameStamp {
        FrameStamp {
            transport: self.transport_at(now),
            ants: self.ants_phase(now),
            chips: self.chips.frame_key(&self.session, wall),
        }
    }

    /// Whether every tick shows something new: a camera glide, or a playhead
    /// playing forward through the timeline.
    pub fn animating(&self, now: Instant) -> bool {
        self.camera_glide.is_some() || self.transport_at(now) == Transport::Playing
    }

    /// The phase rataflow draws animated edges at — `None` when no edge is
    /// animated or the ants have been still for [`ANTS_MOVE_FOR`].
    fn ants_phase(&self, now: Instant) -> Option<u64> {
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
    use crate::state::session::MAIN_ID;
    use crate::state::{CameraGlide, Mode};
    use crate::tailer::{ReplayItem, UiEvent};

    const TICK: Duration = Duration::from_millis(16);

    fn main_says(at: DateTime<Utc>, kinds: Vec<FactKind>) -> Statement {
        let born = FactKind::Agent {
            kind: AgentKind::Main,
            parent: None,
            agent_type: Some("claude".into()),
            description: None,
            spawned_by: None,
            interactive: true,
        };
        let facts = std::iter::once(born)
            .chain(kinds)
            .map(|kind| Fact {
                agent: Some(MAIN_ID.to_string()),
                ts: Some(at),
                kind,
            })
            .collect();
        Statement {
            at: Some(at),
            facts,
        }
    }

    /// A finished session opened at its edge: nothing moves on its own.
    fn finished() -> App {
        let at: DateTime<Utc> = "2026-06-05T10:00:00Z".parse().unwrap();
        let mut app = App::new("s".into(), Mode::Live);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: vec![ReplayItem::new(main_says(at, vec![FactKind::Activity]))],
            speed: 8.0,
            info: Default::default(),
        });
        app.go_live();
        app.tick_timeline(Duration::ZERO);
        app
    }

    /// A live session whose last append just arrived, with `kinds` stated by
    /// main `ago` before `wall`.
    fn live(wall: DateTime<Utc>, ago: chrono::Duration, kinds: Vec<FactKind>) -> App {
        let mut app = App::new("s".into(), Mode::Live);
        app.handle_ui_event(UiEvent::Batch {
            session_id: "s".into(),
            statements: vec![main_says(wall - ago, kinds)],
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
            let moving = app.tick_auto_pan(TICK);
            app.tick_animation(TICK);
            if app.tick_camera(TICK) {
                gate.mark();
            }
            app.tick_timeline(TICK);
            let wall = wall.map(|w| w + chrono::Duration::milliseconds(16 * i64::from(k)));
            drawn += u32::from(gate.due(app, start + TICK * k, wall, moving));
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

    #[test]
    fn draw_every_tick_while_gliding() {
        let mut app = finished();
        let mut gate = RedrawGate::default();
        let start = Instant::now();
        gate.due(&app, start, None, false);
        app.camera_glide = Some(CameraGlide {
            from: (0.0, 0.0),
            to: (50.0, 0.0),
            t: 0.0,
        });
        // 0.5 s of glide is 31 whole ticks plus the landing.
        assert_eq!(run(&mut app, &mut gate, start, None, 32), 32);
        assert!(app.camera_glide.is_none());
        assert_eq!(run(&mut app, &mut gate, start, None, 30), 0);
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
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
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
        let mut app = live(wall, chrono::Duration::zero(), vec![FactKind::Activity]);
        let batch = Instant::now();
        let mut gate = RedrawGate::default();
        assert!(gate.due(&app, batch, None, false));
        assert_eq!(app.transport_at(batch), Transport::Live);

        // Ticks spanning the moment the append stops being fresh: one draw,
        // for the badge turning Idle.
        let before = batch + Duration::from_millis(9_900);
        let drawn = run(&mut app, &mut gate, before, None, 25);
        assert_eq!(drawn, 1);
        assert_eq!(app.transport_at(before + TICK * 25), Transport::Idle);
    }

    /// A tool still in flight shows its running duration, which has to keep
    /// ticking on screen even though nothing else changes.
    #[test]
    fn pending_chip_duration_redraws() {
        let wall = Utc::now();
        let pending = FactKind::ToolStart {
            call: "t1".into(),
            name: "bash".into(),
            summary: None,
        };
        let mut app = live(wall, chrono::Duration::seconds(60), vec![pending]);
        let mut gate = RedrawGate::default();
        let start = Instant::now();
        assert!(gate.due(&app, start, Some(wall), false));
        let drawn = run(&mut app, &mut gate, start, Some(wall), 63);
        assert!((1..=2).contains(&drawn), "{drawn} draws in one second");
    }

    /// A main agent that is Running but has nothing in flight and no moving
    /// edges has nothing that changes with time.
    #[test]
    fn running_without_pending_draws_at_most_once_per_second() {
        let wall = Utc::now();
        let mut app = live(
            wall,
            chrono::Duration::seconds(40),
            vec![FactKind::Activity],
        );
        let mut gate = RedrawGate::default();
        let start = Instant::now();
        gate.due(&app, start, Some(wall), false);
        assert!(run(&mut app, &mut gate, start, Some(wall), 63) <= 1);
    }
}
