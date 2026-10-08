//! Where omp keeps a session on disk, and how to tell what a file is.
//!
//! omp writes `<agent-dir>/sessions/<project-key>/<ISO-ts>_<uuid>.jsonl` for a
//! root session and, beside it, a directory of the same stem holding one
//! sibling `.jsonl` per spawned child (see `super`):
//! `<project-key>/<ISO-ts>_<uuid>/<Name>.jsonl`. That directory also holds
//! `<n>.bash.log`/`<n>.read.log` (raw tool output), `<Name>.md`/`<Name>.json`
//! (a child's written result) and `.lock` files — none of them transcripts,
//! all skipped by extension.
//!
//! `project_key` follows omp 18.8.4's `getDefaultSessionDirName`
//! (`src/session/session-paths.ts`): every path is canonicalized with
//! `realpath` first, a `cwd` under the system temp dir is `-tmp-<rel>`, one
//! under `$HOME` is `-<rel>`, anything else is the whole path wrapped in `--`,
//! and every `/`, `\` and `:` becomes `-`.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::provider::{FileRole, Provider, ReadMode, Scope, SessionFile};

// ---------------------------------------------------------------------------
// Roots
// ---------------------------------------------------------------------------

/// omp's session-storage root: `$PI_CODING_AGENT_SESSION_DIR` names the
/// sessions directory directly; failing that, `$PI_CODING_AGENT_DIR` names its
/// parent (`sessions` appended); failing that, `$HOME/.omp/agent/sessions`.
/// omp aliases every `OMP_`-prefixed variable to its `PI_` counterpart, so an
/// override set either way lands here as the `PI_` name.
fn sessions_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("PI_CODING_AGENT_SESSION_DIR") {
        return Some(PathBuf::from(dir));
    }
    if let Ok(dir) = std::env::var("PI_CODING_AGENT_DIR") {
        return Some(PathBuf::from(dir).join("sessions"));
    }
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".omp")
            .join("agent")
            .join("sessions"),
    )
}

// ---------------------------------------------------------------------------
// project_key
// ---------------------------------------------------------------------------

/// The directory name omp files a session at `cwd` under.
pub fn project_key(cwd: &Path) -> String {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    project_key_under(
        &canonical(cwd),
        home.as_deref().map(canonical).as_deref(),
        &canonical(&std::env::temp_dir()),
    )
}

/// [`project_key`] over already-canonical paths. The temp root is checked
/// first because it is the more specific root wherever it nests inside home.
fn project_key_under(cwd: &Path, home: Option<&Path>, tmp: &Path) -> String {
    if let Some(rel) = relative_within(cwd, tmp) {
        return join_under("-tmp", &rel);
    }
    if let Some(rel) = home.and_then(|home| relative_within(cwd, home)) {
        return join_under("-", &rel);
    }
    let whole = cwd.to_string_lossy();
    format!(
        "--{}--",
        dashed(whole.strip_prefix(['/', '\\']).unwrap_or(&whole))
    )
}

/// omp's `resolveEquivalentPath`: `realpath`, or the path itself when it
/// cannot be resolved (a project directory that no longer exists).
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `path` relative to `base` when it is `base` itself (empty) or inside it.
fn relative_within(path: &Path, base: &Path) -> Option<String> {
    let rel = path.strip_prefix(base).ok()?;
    Some(rel.to_string_lossy().into_owned())
}

/// omp's `encodeRelativeSessionDirName`.
fn join_under(prefix: &str, rel: &str) -> String {
    let encoded = dashed(rel);
    if encoded.is_empty() {
        prefix.to_string()
    } else if prefix.ends_with('-') {
        format!("{prefix}{encoded}")
    } else {
        format!("{prefix}-{encoded}")
    }
}

fn dashed(s: &str) -> String {
    s.replace(['/', '\\', ':'], "-")
}

// ---------------------------------------------------------------------------
// What a path is
// ---------------------------------------------------------------------------

/// Whether `name` is shaped like a session's `<ISO-ts>_<uuid>` stem (e.g.
/// `2026-09-06T20-10-29-994Z_01a07858-...`) — the sibling-directory name a
/// spawned child's file sits under. Shape only, so [`classify_path`] never
/// touches the filesystem.
fn looks_like_session_stem(name: &str) -> bool {
    let Some((ts, uuid)) = name.split_once('_') else {
        return false;
    };
    ts.contains('T') && ts.ends_with('Z') && !uuid.is_empty()
}

/// What a path is, by where it sits in the project layout:
///
/// - `<key>/<ts>_<uuid>.jsonl` is the root;
/// - `<key>/<ts>_<uuid>/<name>.jsonl` is a spawned child.
pub fn session_file(path: &Path) -> Option<SessionFile> {
    classify_path(path, crate::provider::modified(path))
}

/// [`session_file`] with the modification time supplied by the caller.
pub fn classify_path(path: &Path, modified: SystemTime) -> Option<SessionFile> {
    if path.extension().is_none_or(|e| e != "jsonl") {
        return None;
    }
    let parent = path.parent()?;
    let parent_name = parent.file_name()?.to_str()?;
    if looks_like_session_stem(parent_name) {
        let key = parent.parent()?.file_name()?.to_str()?.to_string();
        return Some(SessionFile {
            provider: Provider::Omp,
            path: path.to_path_buf(),
            session: parent_name.to_string(),
            role: FileRole::Agent {
                parent: parent_name.to_string(),
            },
            read: ReadMode::Tail,
            project_key: key,
            modified,
        });
    }
    let session = path.file_stem()?.to_str()?.to_string();
    Some(SessionFile {
        provider: Provider::Omp,
        path: path.to_path_buf(),
        session,
        role: FileRole::Root,
        read: ReadMode::Tail,
        project_key: parent_name.to_string(),
        modified,
    })
}

/// Every path that could be a root session file under the sessions root.
/// Only roots, like Claude: a spawned child is found from its root via
/// [`related_paths`].
pub fn all_paths(scope: &Scope) -> Vec<PathBuf> {
    let Some(root) = sessions_dir() else {
        return Vec::new();
    };
    let dirs: Vec<PathBuf> = match &scope.project {
        // A project scope narrows to the one directory its key names.
        Some(cwd) => vec![root.join(project_key(cwd))],
        None => std::fs::read_dir(&root)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .collect()
            })
            .unwrap_or_default(),
    };
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "jsonl") {
                continue;
            }
            if let Some(since) = scope.since
                && crate::provider::modified(&path) < since
            {
                continue;
            }
            if let Some(prefix) = &scope.id_prefix
                && !path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with(prefix.as_str()))
            {
                continue;
            }
            out.push(path);
        }
    }
    out.sort();
    out
}

/// Where the rest of a file's session is: the root beside the `<ts>_<uuid>`
/// directory, and every child `.jsonl` in it. A root with no spawned children
/// has no such directory, so only the root comes back.
pub fn related_paths(file: &SessionFile) -> Vec<PathBuf> {
    let session_dir = match &file.role {
        FileRole::Root => file.path.with_extension(""),
        FileRole::Agent { .. } => match file.path.parent() {
            Some(p) => p.to_path_buf(),
            None => return Vec::new(),
        },
        FileRole::Sidecar => return Vec::new(),
    };
    let root = session_dir.with_extension("jsonl");
    let mut out = vec![root];
    if let Ok(rd) = std::fs::read_dir(&session_dir) {
        let mut children: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
            .collect();
        children.sort();
        out.extend(children);
    }
    out.retain(|p| p.is_file());
    out
}

/// A parser for one of this session's tailed files: the interactive root, or
/// a spawned child labelled by its own file stem — the only name either side
/// of the spawn agrees on (see `super`).
pub fn stream_for(file: &SessionFile) -> super::Stream {
    match &file.role {
        FileRole::Agent { .. } => {
            let name = file
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("agent")
                .to_string();
            super::Stream::new_child(name)
        }
        // `related_paths` only ever returns root/child `.jsonl` paths, so a
        // Sidecar never reaches here; reading it as a root is the safe answer.
        FileRole::Root | FileRole::Sidecar => super::Stream::new_root(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each row is a real directory name under `~/.omp/agent/sessions`.
    #[test]
    fn project_key_matches_omp_session_paths() {
        let home = Some(Path::new("/Users/me"));
        let tmp = Path::new("/private/var/folders/mt/x/T");
        let key = |cwd: &str| project_key_under(Path::new(cwd), home, tmp);
        assert_eq!(
            key("/Users/me/Workspace/herdr-agents-graph.worktrees/1-agents-graph-mvp"),
            "-Workspace-herdr-agents-graph.worktrees-1-agents-graph-mvp"
        );
        assert_eq!(key("/Users/me"), "-");
        // `/tmp` canonicalizes to `/private/tmp`, outside both roots.
        assert_eq!(key("/private/tmp"), "--private-tmp--");
        assert_eq!(key("/private/var/folders/mt/x/T"), "-tmp");
        assert_eq!(key("/private/var/folders/mt/x/T/wt/a"), "-tmp-wt-a");
        // The temp root wins where it nests inside home.
        let nested_tmp = Path::new("/Users/me/AppData/Local/Temp");
        assert_eq!(
            project_key_under(
                Path::new("/Users/me/AppData/Local/Temp/x"),
                home,
                nested_tmp
            ),
            "-tmp-x"
        );
        // `:` is a separator too (Windows drive letters in the wrapped form).
        assert_eq!(key("/opt/a:b"), "--opt-a-b--");
    }

    #[test]
    fn classify_path_tells_a_root_from_a_spawned_child() {
        let root = Path::new("/x/-tmp/2026-09-06T20-10-29-994Z_01a07858.jsonl");
        let child = Path::new("/x/-tmp/2026-09-06T20-10-29-994Z_01a07858/TickReviewer.jsonl");

        let r = classify_path(root, SystemTime::UNIX_EPOCH).unwrap();
        assert_eq!(r.role, FileRole::Root);
        assert_eq!(r.session, "2026-09-06T20-10-29-994Z_01a07858");
        assert_eq!(r.project_key, "-tmp");

        let c = classify_path(child, SystemTime::UNIX_EPOCH).unwrap();
        assert_eq!(
            c.role,
            FileRole::Agent {
                parent: "2026-09-06T20-10-29-994Z_01a07858".to_string()
            }
        );
        assert_eq!(c.session, "2026-09-06T20-10-29-994Z_01a07858");
        assert_eq!(c.project_key, "-tmp");

        assert!(
            classify_path(Path::new("/x/-tmp/a_b/Reviewer.md"), SystemTime::UNIX_EPOCH).is_none()
        );
    }
}
