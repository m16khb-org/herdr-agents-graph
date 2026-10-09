//! Snapshot goldens of the three views in both themes at two sizes.
//!
//! Input: the omp fixture replayed to its end, so the timeline's "now" is the
//! playhead and every time on screen is a recorded one; clock times print in
//! UTC. Each golden (`assets/ui/<view>-<theme>-<WxH>.txt`) holds the frame's
//! text and its style runs named by SEED token (see `seed::snapshot`).
//! Regenerate with `UPDATE_GOLDEN=1`.

use std::path::PathBuf;

use chrono::FixedOffset;

use crate::provider::Target;
use crate::state::{App, Mode, View};
use crate::tailer::UiEvent;
use crate::ui::seed::snapshot::serialize;
use crate::ui::seed::theme::{Depth, Mode as ThemeMode, Theme};

const OMP_FIXTURE: &str = "2026-01-01T00-00-00-000Z_00000000-0000-7000-0000-000000000001.jsonl";

/// The omp fixture replayed to its end and settled. `None` outside a git
/// checkout (the published crate ships no fixtures).
pub(super) fn fixture_app() -> Option<App> {
    let dir = crate::provider::harness::fixture_dir("omp")?;
    let session = crate::provider::open(&Target::Path(dir.join(OMP_FIXTURE)), None).unwrap();
    let (items, info, _) = crate::tailer::build_replay(&session);
    let mut app = App::new(session.id.clone(), Mode::Replay);
    app.utc_offset = FixedOffset::east_opt(0).unwrap();
    app.handle_ui_event(UiEvent::ReplayLoaded {
        session_id: session.id,
        items,
        speed: 8.0,
        info,
    });
    app.go_live();
    app.tick_timeline(std::time::Duration::ZERO);
    Some(app)
}

fn golden_path(view: &str, theme: &str, w: u16, h: u16) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/ui")
        .join(format!("{view}-{theme}-{w}x{h}.txt"))
}

/// Draw `view` of the fixture in `mode` at `w` × `h` and compare it with its
/// golden (or rewrite the golden under `UPDATE_GOLDEN=1`).
fn check(view: View, mode: ThemeMode, w: u16, h: u16) {
    let Some(mut app) = fixture_app() else {
        return;
    };
    app.set_view(view);
    // The root selected: the Now view opens its side detail at 120 columns,
    // and every view shows a selection.
    app.select_step(0);
    let theme = Theme::new(mode, Depth::TrueColor);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
    // Two frames: the graph centers a selection only once it knows its size.
    for _ in 0..2 {
        terminal
            .draw(|f| crate::ui::draw(f, &mut app, &theme))
            .unwrap();
    }
    let actual = serialize(terminal.backend().buffer(), &theme);

    let name = match view {
        View::Now => "now",
        View::Lanes => "lanes",
        View::Graph => "graph",
    };
    let tone = match mode {
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    };
    let path = golden_path(name, tone, w, h);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("no golden at {}; run with UPDATE_GOLDEN=1", path.display()))
        .replace("\r\n", "\n");
    if expected != actual {
        let first = expected
            .lines()
            .zip(actual.lines())
            .position(|(e, a)| e != a)
            .unwrap_or(expected.lines().count().min(actual.lines().count()));
        panic!(
            "{} differs at line {}:\n  expected: {:?}\n  actual:   {:?}\nRun with UPDATE_GOLDEN=1 if the change is intended.",
            path.display(),
            first + 1,
            expected.lines().nth(first).unwrap_or(""),
            actual.lines().nth(first).unwrap_or("")
        );
    }
}

macro_rules! snapshots {
    ($($test:ident: $view:expr, $mode:expr, $w:expr, $h:expr;)*) => {
        $(
            #[test]
            fn $test() {
                check($view, $mode, $w, $h);
            }
        )*
    };
}

snapshots! {
    now_light_80x24: View::Now, ThemeMode::Light, 80, 24;
    now_light_120x40: View::Now, ThemeMode::Light, 120, 40;
    now_dark_80x24: View::Now, ThemeMode::Dark, 80, 24;
    now_dark_120x40: View::Now, ThemeMode::Dark, 120, 40;
    lanes_light_80x24: View::Lanes, ThemeMode::Light, 80, 24;
    lanes_light_120x40: View::Lanes, ThemeMode::Light, 120, 40;
    lanes_dark_80x24: View::Lanes, ThemeMode::Dark, 80, 24;
    lanes_dark_120x40: View::Lanes, ThemeMode::Dark, 120, 40;
    graph_light_80x24: View::Graph, ThemeMode::Light, 80, 24;
    graph_light_120x40: View::Graph, ThemeMode::Light, 120, 40;
    graph_dark_80x24: View::Graph, ThemeMode::Dark, 80, 24;
    graph_dark_120x40: View::Graph, ThemeMode::Dark, 120, 40;
}
