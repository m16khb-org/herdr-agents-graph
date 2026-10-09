//! Terminal lifecycle and the central event loop.
//!
//! A single-task UI loop that ticks every 16 ms while the picture moves
//! (marching ants, glides, the replay playhead) and every 200 ms otherwise,
//! but draws only when the frame is stale —
//! see [`RedrawGate`]. Installs mouse capture and a panic hook that also
//! disables mouse capture (ratatui's default hook restores the screen but not
//! mouse capture). Drains both channels after the `select!` so input never
//! lags.
//!
//! The theme is decided once, before the terminal enters raw mode: the
//! background query needs the terminal's answer, and its own reply must not
//! reach the keymap (see `drain_input`).

use std::io::stdout;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};

use crate::handler;
use crate::state::{App, RedrawGate};
use crate::tailer::{TailRequest, UiEvent};
use crate::ui;
use crate::ui::seed::theme::Theme;

/// Tick cadence while the picture moves on its own (playback, a glide,
/// marching ants, a drag). 16 ms ≈ 60 fps keeps that motion smooth; a tick
/// with nothing new on screen draws nothing.
const TICK: Duration = Duration::from_millis(16);

/// Tick cadence when nothing moves: what still changes with time alone is a
/// whole-second timer or a multi-second deadline, which five ticks a second
/// show on time. Input and tailer events never wait for a tick — they wake
/// the loop themselves.
const IDLE_TICK: Duration = Duration::from_millis(200);

/// Interval for re-deriving time-based agent status (see `App::status_tick`).
const STATUS_TICK: Duration = Duration::from_secs(1);

/// Run the TUI to completion.
///
/// Owns the terminal, spawns the crossterm `EventStream` reader into an
/// unbounded channel, ticks at 16 ms while something moves and at 200 ms
/// otherwise, routes UI events / input each iteration, and draws when the
/// frame is stale. Returns when the user quits.
pub async fn run(
    mut app: App,
    // Held for the run only to keep the request channel open: the tailer treats a
    // closed channel as "exit", so dropping this sender would kill tailing. No
    // input path sends on it (auto-switch is internal to the tailer).
    _tail_tx: mpsc::Sender<TailRequest>,
    mut ui_rx: mpsc::Receiver<UiEvent>,
) -> anyhow::Result<()> {
    let theme = Theme::detect();
    let mut terminal = ratatui::init();
    drain_input()?;
    execute!(stdout(), EnableMouseCapture)?;
    install_panic_hook();

    // Crossterm event reader → unbounded channel (input must never block the
    // tailer's bounded channel, and bursts of mouse motion must not be dropped).
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        use futures::StreamExt;
        let mut reader = crossterm::event::EventStream::new();
        while let Some(Ok(event)) = reader.next().await {
            if event_tx.send(event).is_err() {
                break;
            }
        }
    });

    let mut last_tick = Instant::now();
    let mut last_status_tick = Instant::now();

    let mut gate = RedrawGate::default();

    let result = loop {
        // Advance animation/auto-pan EVERY iteration, drawn or not, so motion
        // resumes from where it is rather than jumping.
        let now = Instant::now();
        app.tick_clock(now.into_std());
        let elapsed = now - last_tick;
        let panning = app.tick_auto_pan(elapsed);
        app.tick_animation(elapsed);
        if app.tick_camera(elapsed) {
            gate.mark();
        }
        // Advance the replay playhead (paces replay; no-op while following a
        // live edge, where folding happens as Batches arrive).
        app.tick_timeline(elapsed);
        last_tick = now;

        // Interactive liveness (main/forks) is time-derived: a quiet session
        // produces no batches, so running→idle transitions need their own
        // clock. ~1s granularity is plenty for a minutes-scale idle window.
        if now - last_status_tick >= STATUS_TICK {
            if app.status_tick() {
                gate.mark();
            }
            last_status_tick = now;
        }
        // Folds follow every input that moves them, including the ones that
        // never re-sync the graph themselves (a paused end whose file grows).
        if app.reconcile_folds() {
            gate.mark();
        }

        // Draw at the top of the loop, so state changes from the previous
        // iteration are reflected — but only when the frame is stale.
        let wall = app.timeline.now_reference();
        if gate.due(&app, now.into_std(), wall, panning)
            && let Err(e) = terminal.draw(|frame| ui::draw(frame, &mut app, &theme))
        {
            break Err(e.into());
        }

        let cadence = if panning || app.in_motion(now.into_std()) {
            TICK
        } else {
            IDLE_TICK
        };
        tokio::select! {
            _ = tokio::time::sleep_until(now + cadence) => {}
            Some(ev) = ui_rx.recv() => {
                app.handle_ui_event(ev);
                gate.mark();
            }
            Some(ev) = event_rx.recv() => {
                gate.mark();
                if handler::handle_event(&ev, &mut app) {
                    break Ok(());
                }
            }
        }

        // Drain both channels after the select so neither input nor tailer
        // batches lag behind a single per-frame wakeup.
        let mut quit = false;
        while let Ok(ev) = event_rx.try_recv() {
            gate.mark();
            if handler::handle_event(&ev, &mut app) {
                quit = true;
                break;
            }
        }
        while let Ok(ev) = ui_rx.try_recv() {
            app.handle_ui_event(ev);
            gate.mark();
        }

        if quit || app.should_quit {
            break Ok(());
        }
    };

    // Clean restore regardless of how the loop ended.
    let _ = execute!(stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

/// Throw away whatever input is already waiting, before the event reader
/// starts. A terminal that answers the background query late (after the
/// query gave up) leaves an OSC reply in the input; read in raw mode it
/// would arrive as keys — `]` among them, which steps the timeline.
/// Canonical mode would not deliver a reply without a newline, so this runs
/// after `ratatui::init` has entered raw mode.
fn drain_input() -> anyhow::Result<()> {
    while crossterm::event::poll(Duration::ZERO)? {
        let _ = crossterm::event::read()?;
    }
    Ok(())
}

/// Install a panic hook that disables mouse capture before delegating to
/// ratatui's screen-restoring hook, so a panic leaves the terminal usable.
///
/// Called after [`fn@ratatui::init`] (which installs the screen-restore hook), so
/// our layer wraps it: we disable mouse capture first, then chain to the prior
/// hook which leaves raw mode / the alternate screen.
pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(stdout(), DisableMouseCapture);
        prev(info);
    }));
}
