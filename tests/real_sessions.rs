//! Every session file on this machine, read the way the app reads it.
//!
//! Opt-in, because it reads the user's own transcripts:
//!
//! ```text
//! AG_REAL_SESSIONS=1 cargo test --release --test real_sessions -- --nocapture
//! ```
//!
//! Each `.jsonl` under the Claude Code, Codex and omp session roots (or under
//! `AG_REAL_SESSIONS_ROOT` instead, when set) is opened as a session, streamed
//! through its provider line by line, and finished. A file that cannot join a
//! session — a Codex child whose root rollout was archived, an internal
//! subagent with no parent link — is parsed on its own instead and counted as
//! `standalone`. A file fails when no provider recognizes it, when parsing
//! panics, or when a non-empty file states nothing at all. The report names
//! only paths and the kind of failure, never content.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use agents_graph::provider::{
    FileRole, Provider, ReadMode, SessionFile, Target, open, provider_of,
};
use agents_graph::tailer::read_lines;

/// Every `.jsonl` file under `dir`, at any depth.
fn jsonl_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            jsonl_under(&path, out);
        } else if path.extension().is_some_and(|e| e == "jsonl") {
            out.push(path);
        }
    }
}

fn session_files() -> Vec<PathBuf> {
    let roots: Vec<PathBuf> = match std::env::var_os("AG_REAL_SESSIONS_ROOT") {
        Some(root) => vec![PathBuf::from(root)],
        None => {
            let home = PathBuf::from(std::env::var_os("HOME").expect("HOME"));
            vec![
                home.join(".claude/projects"),
                home.join(".codex/sessions"),
                home.join(".omp/agent/sessions"),
            ]
        }
    };
    let mut files = Vec::new();
    for root in roots {
        jsonl_under(&root, &mut files);
    }
    files.sort();
    files
}

enum Outcome {
    Clean,
    Standalone,
    Failed(String),
}

/// The provider and file entry to read `path` with on its own, for a file
/// that does not open as part of a session.
fn standalone(path: &Path) -> Option<(Provider, SessionFile)> {
    use std::io::BufRead;
    let mut first = String::new();
    std::io::BufReader::new(std::fs::File::open(path).ok()?)
        .read_line(&mut first)
        .ok()?;
    let provider = provider_of(&first)?;
    let file = provider.session_file(path).unwrap_or_else(|| SessionFile {
        provider,
        path: path.to_path_buf(),
        session: String::new(),
        role: FileRole::Root,
        read: ReadMode::Tail,
        project_key: String::new(),
        modified: std::time::SystemTime::UNIX_EPOCH,
    });
    Some((provider, file))
}

fn check(path: &Path) -> Outcome {
    let (provider, file, alone) = match open(&Target::Path(path.to_path_buf()), None) {
        Ok(session) => match session.every_file().find(|f| f.path == path) {
            Some(file) => (session.provider, file.clone(), false),
            None => return Outcome::Failed("not part of the session it opens".into()),
        },
        Err(e) => match standalone(path) {
            Some((provider, file)) => (provider, file, true),
            None => return Outcome::Failed(format!("open: {e}")),
        },
    };
    let mut stream = provider.stream_for(&file);
    let mut stated = 0usize;
    let read = catch_unwind(AssertUnwindSafe(|| {
        let state = read_lines(path, &mut |line| {
            stated += usize::from(stream.push(line).is_some());
        });
        stated += usize::from(stream.finish().is_some());
        state
    }));
    match read {
        Err(_) => Outcome::Failed("panic".into()),
        Ok(Err(e)) => Outcome::Failed(format!("read: {}", e.kind())),
        Ok(Ok(state)) if state.offset > 0 && stated == 0 => {
            Outcome::Failed("no records read".into())
        }
        Ok(Ok(_)) if alone => Outcome::Standalone,
        Ok(Ok(_)) => Outcome::Clean,
    }
}

#[test]
fn every_local_session_file_parses() {
    if std::env::var_os("AG_REAL_SESSIONS").is_none() {
        eprintln!("real_sessions: skipped (set AG_REAL_SESSIONS=1)");
        return;
    }
    let files = session_files();
    let mut standalone = 0usize;
    let mut failures: Vec<(&Path, String)> = Vec::new();
    for path in &files {
        match check(path) {
            Outcome::Clean => {}
            Outcome::Standalone => standalone += 1,
            Outcome::Failed(why) => failures.push((path, why)),
        }
    }
    for (path, why) in &failures {
        println!("failed: {} ({why})", path.display());
    }
    println!(
        "real_sessions: total={} failed={} standalone={standalone}",
        files.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{} file(s) failed", failures.len());
}
