//! Input routing.
//!
//! One keymap for every view (the table in the help overlay and the README):
//! `tab`/`1 2 3` switch views, `j`/`k` move the shared selection (or scroll
//! an open detail), `enter`/`esc` open and close, `space`/`[ ]`/`G` drive the
//! timeline. Keys that only mean something on one view (lane cursor, graph
//! pan and zoom) act there and nowhere else. The graph is read-only, so only
//! navigation and viewport keys ever reach rataflow (a whitelist, not a
//! fall-through).
//!
//! The mouse uses where the last frame put things ([`App::hit`]): tabs
//! switch views, rows select, the lane axis seeks, and on the graph canvas
//! everything goes to rataflow.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use rataflow::EventResponse;

use crate::state::view::{contains, fold_of_row_key};
use crate::state::{App, Camera, View};

/// Rows moved per PageUp/PageDown in the detail's tool-call list.
const PAGE_SCROLL: i32 = 10;

/// Route one crossterm event. Returns `true` if the app should quit.
pub fn handle_event(event: &Event, app: &mut App) -> bool {
    match event {
        Event::Key(key) => handle_key(key, app),
        Event::Mouse(mouse) => {
            handle_mouse(mouse, app);
            false
        }
        _ => false,
    }
}

fn handle_mouse(mouse: &MouseEvent, app: &mut App) {
    let (x, y) = (mouse.column, mouse.row);
    let left_press = matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left));
    let left_drag = matches!(mouse.kind, MouseEventKind::Drag(MouseButton::Left));

    if left_press && let Some(view) = app.hit.tab_at(x, y) {
        app.set_view(view);
        return;
    }
    // The lane axis seeks: queue the time under the pointer, applied once per
    // frame, so a drag burst costs one rebuild.
    if (left_press || left_drag)
        && let Some(axis) = &app.hit.lane_axis
        && contains(axis.area, x, y)
    {
        let col = usize::from(x - axis.area.x).min(axis.columns.len().saturating_sub(1));
        if let Some(t) = axis.columns.get(col) {
            app.pending_seek = Some(*t);
        }
        return;
    }
    if left_press && let Some(key) = app.hit.row_at(x, y) {
        let key = key.to_string();
        match fold_of_row_key(&key) {
            Some(parent) => app.select_fold(parent.to_string()),
            None => app.select_agent(Some(key)),
        }
        return;
    }
    let on_canvas = app.hit.canvas.is_some_and(|r| contains(r, x, y));
    if app.view == View::Graph && on_canvas {
        let response = app.flow.handle_mouse_event(*mouse);
        app.process_flow_events(response.into_events());
        return;
    }
    match mouse.kind {
        MouseEventKind::ScrollDown => step(app, 1),
        MouseEventKind::ScrollUp => step(app, -1),
        _ => {}
    }
}

/// `j`/`k`: scroll an open detail, otherwise move the selection.
fn step(app: &mut App, delta: isize) {
    if app.selection.detail {
        scroll_detail(app, delta as i32);
    } else {
        app.select_step(delta);
    }
}

/// Handle a single key event, returning `true` to quit.
fn handle_key(key: &KeyEvent, app: &mut App) -> bool {
    // Some terminals emit both Press and Release; act on Press only.
    if matches!(key.kind, KeyEventKind::Release) {
        return false;
    }
    // No key here takes Alt. A terminal's late reply to the start-up colour
    // query (`ESC ] 11;…`) parses as Alt+`]`; letting it through would step
    // the timeline. Ctrl+Alt is AltGr on Windows, which types `[` and `]` on
    // many keyboard layouts, so that combination still counts as the key.
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    if alt && !key.modifiers.contains(KeyModifiers::CONTROL) {
        return false;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let graph = app.view == View::Graph;

    match key.code {
        KeyCode::Char('q' | 'Q') => {
            app.should_quit = true;
            return true;
        }
        KeyCode::Char('c' | 'C') if ctrl => {
            app.should_quit = true;
            return true;
        }

        // Views.
        KeyCode::Tab => app.set_view(app.view.cycle(1)),
        KeyCode::BackTab => app.set_view(app.view.cycle(-1)),
        KeyCode::Char('1') => app.set_view(View::Now),
        KeyCode::Char('2') => app.set_view(View::Lanes),
        KeyCode::Char('3') => app.set_view(View::Graph),

        // Selection, shared by every view.
        KeyCode::Char('j') | KeyCode::Down => step(app, 1),
        KeyCode::Char('k') | KeyCode::Up => step(app, -1),
        KeyCode::PageDown => scroll_detail(app, PAGE_SCROLL),
        KeyCode::PageUp => scroll_detail(app, -PAGE_SCROLL),
        KeyCode::Enter => enter(app),
        KeyCode::Esc => escape(app),

        // Timeline.
        KeyCode::Char(' ') => app.toggle_play_pause(),
        KeyCode::Char('[') => app.seek_prompt(false),
        KeyCode::Char(']') => app.seek_prompt(true),
        KeyCode::End | KeyCode::Char('g' | 'G') => {
            let was_live = app.timeline.follow_head && !app.is_paused;
            app.go_live();
            if !was_live {
                app.snack("Back to live");
            }
        }
        KeyCode::Left | KeyCode::Right if app.view == View::Lanes => {
            lane_cursor(app, if key.code == KeyCode::Left { -1 } else { 1 });
        }
        KeyCode::Char('z' | 'Z') => {
            app.fold_gaps = !app.fold_gaps;
            app.snack(if app.fold_gaps {
                "Idle gaps folded"
            } else {
                "Idle gaps shown at full length"
            });
        }
        KeyCode::Char('s' | 'S') => {
            app.timeline.compress_gaps = !app.timeline.compress_gaps;
            app.snack(if app.timeline.compress_gaps {
                "Replay skips idle gaps"
            } else {
                "Replay in real time"
            });
        }

        // Camera and layout (the graph's, but `f` also makes the selection
        // follow activity on every view).
        KeyCode::Char('f' | 'F') => {
            app.camera = Camera::Follow;
            app.track_activity();
        }
        KeyCode::Char('o' | 'O') => {
            app.camera = Camera::Overview;
            app.camera_glide = None;
            app.flow.request_fit_view();
        }
        KeyCode::Char('r' | 'R') => app.relayout_now(),

        // Overlays.
        KeyCode::Char('?') => app.show_help = !app.show_help,
        KeyCode::Char('i' | 'I') => app.show_info = !app.show_info,

        // Graph-only: spatial selection, pan, zoom, center.
        KeyCode::Left | KeyCode::Right if graph => to_flow(app, *key),
        KeyCode::Char('h' | 'l' | 'c') if graph => to_flow(app, *key),
        // Vertical pan moved to shift (`j`/`k` select everywhere); rataflow
        // pans on the lowercase keys, so forward those.
        KeyCode::Char(c @ ('H' | 'J' | 'K' | 'L')) if graph => {
            let lower = KeyEvent::new(KeyCode::Char(c.to_ascii_lowercase()), KeyModifiers::NONE);
            to_flow(app, lower);
        }
        KeyCode::Char('+' | '=' | '-' | '_' | '0') if graph => {
            let response = app.flow.handle_controls_key_event(*key);
            app.process_flow_events(response.into_events());
        }
        _ => {}
    }
    false
}

/// Hand a navigation key to rataflow and react to what it did.
fn to_flow(app: &mut App, key: KeyEvent) {
    let response: EventResponse = app.flow.handle_key_event(key);
    app.process_flow_events(response.into_events());
}

/// `enter`: select the root when nothing is selected; unfold a selected
/// fold; expand the Now row; then open the full detail.
fn enter(app: &mut App) {
    if app.expand_fold() {
        return;
    }
    let Some(id) = app.selected_agent_id() else {
        app.select_step(0);
        return;
    };
    if app.session.agent(&id).is_none() {
        return;
    }
    if app.view == View::Now && !app.selection.expanded {
        app.selection.expanded = true;
    } else {
        app.selection.detail = true;
    }
}

/// `esc`: close the innermost thing open, then fold an unfolded agent back.
/// Following, the selection is the camera's narration, so it stays.
fn escape(app: &mut App) {
    if app.show_help {
        app.show_help = false;
    } else if app.show_info {
        app.show_info = false;
    } else if app.selection.detail {
        app.selection.detail = false;
    } else if app.selection.expanded {
        app.selection.expanded = false;
    } else if !app.collapse_fold() && app.camera != Camera::Follow {
        app.select_agent(None);
    }
}

/// Scroll the detail's tool-call list by `delta` rows. Scrolling up detaches
/// the tail; the renderer owns the upper clamp and re-attaches at the bottom.
fn scroll_detail(app: &mut App, delta: i32) {
    if app.selection.agent.is_none() {
        return;
    }
    if delta < 0 {
        app.selection.follow = false;
    }
    app.selection.scroll = (i32::from(app.selection.scroll) + delta).max(0) as u16;
}

/// Move the lane cursor one column, by the axis as last drawn (folded gaps
/// make columns uneven in time). Off the live edge the axis ends at the
/// cursor, so a step forward has no drawn column to land on; it moves one
/// column's worth of time instead, and a step past the head re-pins live.
fn lane_cursor(app: &mut App, delta: isize) {
    let Some(axis) = &app.hit.lane_axis else {
        return;
    };
    let Some(cursor) = app.timeline.cursor else {
        return;
    };
    let target = if delta < 0 {
        // The start of the last column strictly before the cursor.
        match axis.columns.partition_point(|c| *c < cursor) {
            0 => return,
            before => axis.columns[before - 1],
        }
    } else {
        let after = axis.columns.partition_point(|c| *c <= cursor);
        match axis.columns.get(after) {
            Some(next) => *next,
            None => cursor + chrono::Duration::milliseconds(axis.ms_per_col.max(1)),
        }
    };
    app.pending_seek = None;
    match app.timeline.head_ts() {
        Some(head) if target >= head => app.go_live(),
        _ => app.seek(target),
    }
}

/// Drain and react to the flow events produced by an input handler call.
pub fn process_flow_events(app: &mut App, events: impl Iterator<Item = rataflow::FlowEvent>) {
    app.process_flow_events(events);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fact::{AgentKind, Fact, FactKind, Statement};
    use crate::state::session::MAIN_ID;
    use crate::state::{LaneAxis, Mode};
    use crate::tailer::{ReplayItem, UiEvent};
    use chrono::{DateTime, Utc};
    use rataflow::FlowEvent;
    use ratatui::layout::Rect;

    fn at(s: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_780_000_000 + s, 0).unwrap()
    }

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn press(app: &mut App, code: KeyCode) -> bool {
        handle_event(&key(code), app)
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
        Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })
    }

    fn agent(id: &str, parent: Option<&str>, ts: i64) -> Fact {
        Fact {
            agent: Some(id.into()),
            ts: Some(at(ts)),
            kind: FactKind::Agent {
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
            },
        }
    }

    /// A replay of main + two subagents, a prompt every 10 s and one tool
    /// call by main from 2 s to 18 s, folded to the end.
    fn session_app() -> App {
        let statement = |facts: Vec<Fact>, s: i64| {
            ReplayItem::new(Statement {
                at: Some(at(s)),
                facts,
            })
        };
        let prompt = |s: i64| Fact {
            agent: Some(MAIN_ID.into()),
            ts: Some(at(s)),
            kind: FactKind::Prompt(format!("prompt at {s}")),
        };
        let main = |s: i64, kind: FactKind| Fact {
            agent: Some(MAIN_ID.into()),
            ts: Some(at(s)),
            kind,
        };
        let start = FactKind::ToolStart {
            call: "c1".into(),
            name: "bash".into(),
            summary: None,
            intent: None,
        };
        let end = FactKind::ToolEnd {
            call: "c1".into(),
            outcome: crate::fact::Outcome::Ok,
        };
        let mut app = App::new("s".into(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: vec![
                statement(vec![agent(MAIN_ID, None, 0), prompt(0)], 0),
                statement(vec![main(2, start)], 2),
                statement(vec![agent("a", Some(MAIN_ID), 5)], 5),
                statement(vec![prompt(10)], 10),
                statement(vec![agent("b", Some(MAIN_ID), 15)], 15),
                statement(vec![main(18, end)], 18),
                statement(vec![prompt(20)], 20),
            ],
            speed: 8.0,
            info: Default::default(),
        });
        app.go_live();
        app.tick_timeline(std::time::Duration::ZERO);
        app
    }

    /// Every row of the keymap table in the README and help overlay, pressed
    /// on a fresh app, does what the table says.
    #[test]
    fn keymap_matches_table() {
        type Check = fn(&App) -> bool;
        let table: Vec<(&str, Vec<KeyCode>, Check)> = vec![
            ("tab cycles views", vec![KeyCode::Tab], |a| {
                a.view == View::Lanes
            }),
            ("shift-tab cycles back", vec![KeyCode::BackTab], |a| {
                a.view == View::Graph
            }),
            ("2 picks lanes", vec![KeyCode::Char('2')], |a| {
                a.view == View::Lanes
            }),
            ("3 picks graph", vec![KeyCode::Char('3')], |a| {
                a.view == View::Graph
            }),
            (
                "1 picks now",
                vec![KeyCode::Char('3'), KeyCode::Char('1')],
                |a| a.view == View::Now,
            ),
            ("j selects the root first", vec![KeyCode::Char('j')], |a| {
                a.selected_agent_id().as_deref() == Some(MAIN_ID)
            }),
            (
                "j moves down the tree",
                vec![KeyCode::Char('j'), KeyCode::Char('j')],
                |a| a.selected_agent_id().as_deref() == Some("a"),
            ),
            (
                "↓ is j",
                vec![KeyCode::Down, KeyCode::Down, KeyCode::Down],
                |a| a.selected_agent_id().as_deref() == Some("b"),
            ),
            (
                "k moves up",
                vec![KeyCode::Char('j'), KeyCode::Char('j'), KeyCode::Char('k')],
                |a| a.selected_agent_id().as_deref() == Some(MAIN_ID),
            ),
            (
                "enter expands the now row",
                vec![KeyCode::Char('j'), KeyCode::Enter],
                |a| a.selection.expanded && !a.selection.detail,
            ),
            (
                "enter twice opens the detail",
                vec![KeyCode::Char('j'), KeyCode::Enter, KeyCode::Enter],
                |a| a.selection.detail,
            ),
            (
                "j scrolls inside the detail",
                vec![
                    KeyCode::Char('j'),
                    KeyCode::Enter,
                    KeyCode::Enter,
                    KeyCode::Char('j'),
                ],
                |a| a.selection.scroll == 1 && a.selected_agent_id().as_deref() == Some(MAIN_ID),
            ),
            (
                "esc closes the detail",
                vec![
                    KeyCode::Char('j'),
                    KeyCode::Enter,
                    KeyCode::Enter,
                    KeyCode::Esc,
                ],
                |a| !a.selection.detail && a.selection.expanded,
            ),
            (
                "esc then deselects",
                vec![KeyCode::Char('j'), KeyCode::Esc],
                |a| a.selected_agent_id().is_none(),
            ),
            ("space pauses", vec![KeyCode::Char(' ')], |a| a.is_paused),
            ("[ steps back a prompt", vec![KeyCode::Char('[')], |a| {
                a.timeline.cursor == Some(at(10))
            }),
            (
                "] after [ steps forward",
                vec![KeyCode::Char('['), KeyCode::Char('['), KeyCode::Char(']')],
                |a| a.timeline.cursor == Some(at(10)),
            ),
            (
                "G goes live",
                vec![KeyCode::Char('['), KeyCode::Char('G')],
                |a| a.timeline.follow_head && a.snackbar.is_some(),
            ),
            ("z toggles gap folding", vec![KeyCode::Char('z')], |a| {
                !a.fold_gaps
            }),
            ("f follows activity", vec![KeyCode::Char('f')], |a| {
                a.camera == Camera::Follow && a.selected_agent_id().is_some()
            }),
            (
                "o frames everything",
                vec![KeyCode::Char('f'), KeyCode::Char('o')],
                |a| a.camera == Camera::Overview,
            ),
            ("? opens help", vec![KeyCode::Char('?')], |a| a.show_help),
            (
                "esc closes help first",
                vec![KeyCode::Char('j'), KeyCode::Char('?'), KeyCode::Esc],
                |a| !a.show_help && a.selected_agent_id().is_some(),
            ),
            ("i opens session info", vec![KeyCode::Char('i')], |a| {
                a.show_info
            }),
            ("s toggles replay pacing", vec![KeyCode::Char('s')], |a| {
                !a.timeline.compress_gaps
            }),
        ];
        for (what, keys, check) in table {
            // The table is written from the Now view (tab order, `enter`).
            let mut app = session_app();
            app.set_view(View::Now);
            for code in keys {
                assert!(!press(&mut app, code), "{what}: no key here quits");
            }
            assert!(check(&app), "{what}");
        }
        let mut app = session_app();
        assert!(press(&mut app, KeyCode::Char('q')), "q quits");
        let mut app = session_app();
        let ctrl_c = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(handle_event(&ctrl_c, &mut app), "ctrl-c quits");
    }

    #[test]
    fn alt_modified_keys_are_ignored_but_altgr_keys_work() {
        let mut app = session_app();
        let before = app.timeline.cursor;
        let with = |c, m| Event::Key(KeyEvent::new(KeyCode::Char(c), m));
        assert!(!handle_event(&with('[', KeyModifiers::ALT), &mut app));
        assert!(!handle_event(&with(']', KeyModifiers::ALT), &mut app));
        assert_eq!(
            app.timeline.cursor, before,
            "a late OSC reply must not seek"
        );
        // Windows reports AltGr as Ctrl+Alt; `[` typed that way still steps.
        let altgr = KeyModifiers::CONTROL | KeyModifiers::ALT;
        handle_event(&with('[', altgr), &mut app);
        assert_eq!(app.timeline.cursor, Some(at(10)));
    }

    #[test]
    fn graph_keys_only_act_on_the_graph() {
        let mut app = session_app();
        app.set_view(View::Now);
        let zoom = app.flow.viewport.zoom;
        press(&mut app, KeyCode::Char('+'));
        assert_eq!(app.flow.viewport.zoom, zoom, "zoom is the graph's");
        press(&mut app, KeyCode::Char('3'));
        press(&mut app, KeyCode::Char('+'));
        assert!(app.flow.viewport.zoom > zoom);
    }

    #[test]
    fn lane_arrows_move_the_cursor_by_columns() {
        use crate::ui::seed::theme::{Depth, Mode as ThemeMode, Theme};
        let theme = Theme::new(ThemeMode::Dark, Depth::TrueColor);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        let mut app = session_app();
        press(&mut app, KeyCode::Char('2'));
        // Each press acts on the axis the previous frame drew, as in the loop.
        let mut step = |app: &mut App, code| {
            terminal.draw(|f| crate::ui::draw(f, app, &theme)).unwrap();
            press(app, code);
            app.timeline.cursor.unwrap()
        };
        let head = app.timeline.cursor.unwrap();
        let back1 = step(&mut app, KeyCode::Left);
        let back2 = step(&mut app, KeyCode::Left);
        assert!(back1 < head && back2 < back1, "← steps back");
        let col = app.hit.lane_axis.as_ref().unwrap().ms_per_col;
        let fwd = step(&mut app, KeyCode::Right);
        assert!(fwd > back2, "→ steps forward: {back2} → {fwd}");
        assert!((fwd - back2).num_milliseconds() <= col, "by one column");
        // Stepping forward past the head re-pins to the live edge.
        for _ in 0..200 {
            step(&mut app, KeyCode::Right);
        }
        assert!(app.timeline.follow_head);
        assert_eq!(app.timeline.cursor, Some(head));
    }

    #[test]
    fn axis_clicks_and_drags_queue_one_seek() {
        let mut app = session_app();
        press(&mut app, KeyCode::Char('2'));
        app.hit.lane_axis = Some(LaneAxis {
            area: Rect::new(10, 3, 5, 4),
            columns: vec![at(0), at(5), at(10), at(15), at(20)],
            ms_per_col: 5_000,
        });
        let down = mouse(MouseEventKind::Down(MouseButton::Left), 11, 4);
        handle_event(&down, &mut app);
        handle_event(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 10, 4),
            &mut app,
        );
        assert_eq!(app.pending_seek, Some(at(0)), "the latest target wins");
        app.tick_timeline(std::time::Duration::ZERO);
        assert_eq!(app.pending_seek, None);
        assert_eq!(app.timeline.cursor, Some(at(0)));
    }

    #[test]
    fn clicks_hit_tabs_and_rows() {
        let mut app = session_app();
        app.hit.tabs = vec![(Rect::new(50, 0, 7, 1), View::Graph)];
        app.hit.rows = vec![(Rect::new(0, 4, 40, 1), "b".into())];
        handle_event(
            &mouse(MouseEventKind::Down(MouseButton::Left), 4, 4),
            &mut app,
        );
        assert_eq!(app.selected_agent_id().as_deref(), Some("b"));
        handle_event(
            &mouse(MouseEventKind::Down(MouseButton::Left), 52, 0),
            &mut app,
        );
        assert_eq!(app.view, View::Graph);
    }

    #[test]
    fn viewport_change_hands_camera_to_user() {
        let mut app = App::new("s".into(), Mode::Live);
        assert_eq!(app.camera, Camera::Overview);
        process_flow_events(
            &mut app,
            vec![FlowEvent::ViewportChanged {
                x: 1.0,
                y: 2.0,
                zoom: 1.5,
            }]
            .into_iter(),
        );
        assert_eq!(app.camera, Camera::Manual);
    }

    /// Moving the selection on the graph leaves the camera to the app's own
    /// glide: rataflow's reveal is off, and the newly selected node is queued
    /// for a smooth center instead.
    #[test]
    fn selection_nav_leaves_the_camera_to_our_glide() {
        use ratatui::widgets::Widget;
        let mut app = session_app();
        press(&mut app, KeyCode::Char('3'));
        // Put "a" far from the root so it is off-screen when the root is.
        let far = app.flow.node("a").unwrap().position;
        app.flow.set_node_position("a", (far.x + 500.0, far.y));
        let area = Rect::new(0, 0, 60, 20);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        (&mut app.flow).render(area, &mut buf);
        app.flow.zoom_to(5.0);
        let root = app.flow.node(MAIN_ID).unwrap().position;
        app.flow.center_on((root.x + 5.0, root.y + 2.5));
        press(&mut app, KeyCode::Char('j'));
        app.pending_center = None;
        let before = (app.flow.viewport.x, app.flow.viewport.y);

        press(&mut app, KeyCode::Char('j'));

        assert_eq!((app.flow.viewport.x, app.flow.viewport.y), before);
        assert_eq!(app.pending_center.as_deref(), Some("a"));
        assert_eq!(
            app.flow.selected_nodes().next().map(|n| n.id.as_str()),
            Some("a"),
            "the graph's own selection mirrors the app's"
        );
    }

    #[test]
    fn destructive_and_stateful_library_keys_are_inert() {
        let mut app = session_app();
        press(&mut app, KeyCode::Char('3'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Delete);
        assert!(app.flow.node(MAIN_ID).is_some(), "Delete must be inert");
        press(&mut app, KeyCode::Char('m'));
        press(&mut app, KeyCode::Backspace);
        assert!(app.flow.node(MAIN_ID).is_some());
        assert!(!app.flow.locked, "no key locks the viewport");
    }

    #[test]
    fn dragging_a_node_drops_follow_to_manual() {
        let mut app = App::new("s".into(), Mode::Live);
        app.camera = Camera::Follow;
        process_flow_events(
            &mut app,
            vec![FlowEvent::NodeDragged {
                node_id: "a".into(),
            }]
            .into_iter(),
        );
        assert_eq!(app.camera, Camera::Manual, "node drag must drop Follow");
    }

    #[test]
    fn selecting_on_the_graph_in_overview_stays_in_overview() {
        let mut app = App::new("s".into(), Mode::Live);
        process_flow_events(
            &mut app,
            vec![FlowEvent::SelectionChanged {
                node_ids: vec!["a".into()],
                edge_ids: vec![],
            }]
            .into_iter(),
        );
        assert_eq!(app.camera, Camera::Overview);
        assert_eq!(app.selected_agent_id().as_deref(), Some("a"));
    }

    #[test]
    fn user_selection_drops_follow_to_manual() {
        let mut app = session_app();
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(app.camera, Camera::Follow);
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.camera, Camera::Manual, "a user selection drops Follow");
        assert!(app.camera_glide.is_none());
    }

    #[test]
    fn deselection_clears_a_pending_center() {
        let mut app = App::new("s".into(), Mode::Live);
        app.pending_center = Some("a".into());
        process_flow_events(
            &mut app,
            vec![FlowEvent::SelectionChanged {
                node_ids: vec![],
                edge_ids: vec![],
            }]
            .into_iter(),
        );
        assert!(app.pending_center.is_none());
    }

    #[test]
    fn r_key_tidies_layout_on_demand() {
        let mut app = App::new("s".into(), Mode::Live);
        app.layout_dirty = true;
        assert!(!press(&mut app, KeyCode::Char('r')));
        assert!(!app.layout_dirty);
    }

    #[test]
    fn camera_keys_name_destinations() {
        let mut app = App::new("s".into(), Mode::Live);
        app.camera = Camera::Manual;
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(app.camera, Camera::Follow);
        app.layout_dirty = true;
        press(&mut app, KeyCode::Char('o'));
        assert_eq!(app.camera, Camera::Overview);
        assert!(app.layout_dirty, "o frames, it does not relayout");
    }

    /// One frame of `app` at 120 × 40, as the loop draws it between keys.
    fn draw_frame(app: &mut App) -> Vec<String> {
        use crate::ui::seed::theme::{Depth, Mode as ThemeMode, Theme};
        let theme = Theme::new(ThemeMode::Dark, Depth::TrueColor);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| crate::ui::draw(f, app, &theme)).unwrap();
        let buf = terminal.backend().buffer();
        (0..buf.area.height)
            .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect())
            .collect()
    }

    /// `enter` on a fold unfolds it onto its first member; `esc` folds it
    /// back with the fold selected.
    #[test]
    fn done_siblings_fold_again_on_esc() {
        let Some(mut app) = crate::ui::snapshots::fixture_app() else {
            return;
        };
        app.select_fold(MAIN_ID.into());
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.selected_agent_id().as_deref(), Some("Reviewer"));
        assert!(app.folds().by_parent.is_empty(), "unfolded");
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.selection.folded.as_deref(), Some(MAIN_ID));
        assert_eq!(app.selected_agent_id(), None);
        assert_eq!(
            app.folds().by_parent.get(MAIN_ID).map(Vec::len),
            Some(2),
            "folded again"
        );
    }

    #[test]
    fn done_siblings_fold_row_click_selects_the_fold() {
        let Some(mut app) = crate::ui::snapshots::fixture_app() else {
            return;
        };
        app.set_view(View::Now);
        draw_frame(&mut app);
        let key = crate::state::view::fold_row_key(MAIN_ID);
        let (rect, _) = *app
            .hit
            .rows
            .iter()
            .find(|(_, k)| *k == key)
            .expect("the fold has a row");
        handle_event(
            &mouse(MouseEventKind::Down(MouseButton::Left), rect.x, rect.y),
            &mut app,
        );
        assert_eq!(app.selection.folded.as_deref(), Some(MAIN_ID));
        assert_eq!(app.selected_agent_id(), None);
    }

    /// The lanes draw every agent, so `j` walks every one of them, folded
    /// members included, and the frame marks each in turn.
    #[test]
    fn lanes_j_reaches_every_lane() {
        let Some(mut app) = crate::ui::snapshots::fixture_app() else {
            return;
        };
        app.set_view(View::Lanes);
        for (id, name) in [
            (MAIN_ID, "omp"),
            ("Reviewer", "reviewer"),
            ("Helper", "scout"),
            ("__sidecar", "subagent"),
        ] {
            press(&mut app, KeyCode::Char('j'));
            let frame = draw_frame(&mut app);
            assert_eq!(app.selected_agent_id().as_deref(), Some(id));
            let marked: Vec<&String> = frame.iter().filter(|l| l.contains('▶')).collect();
            assert!(
                marked.len() == 1 && marked[0].contains(name),
                "{name}: {marked:#?}"
            );
        }
    }
}
