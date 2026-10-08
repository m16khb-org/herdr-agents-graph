// The README is the crate's front page: this is an application, so what a
// reader needs first is what the plugin does, not the module list. What
// follows is the part that only concerns someone depending on the
// library rather than running the binary.
#![doc = include_str!("../README.md")]
//!
//! ## The library
//!
//! The portable core behind the `agents-graph` terminal app (`src/main.rs`).
//! With default features off it builds without tokio, crossterm, or any IO.
//!
//! Portable everywhere: the domain [`state`] (model + unified replay/live
//! [`timeline`](state::Timeline) + flow-graph projection), the [`ui`] rendering,
//! the [`fact`] vocabulary every transcript format is reduced to, and the
//! [`provider`] providers that do the reducing. The wire types and pure replay
//! assembly live in [`tailer`]; its live file-tailing + the terminal loop ([`tui`]) and input
//! ([`handler`]) are native-only (they pull tokio/crossterm/fs) and `cfg`-gated
//! behind the `native` feature.

pub mod fact;
pub mod provider;
pub mod state;
pub mod tailer;
pub mod ui;

// The native frontend: terminal loop + crossterm input.
//
// Frontend plumbing, with no stability promise. These are public because the
// `agents-graph` binary is a separate crate that links this one, not because they are an
// interface to build on: they may change shape in any release. The parts meant
// to be depended on are the domain and the vocabulary above.
#[cfg(feature = "native")]
pub mod handler;
#[cfg(feature = "native")]
pub mod tui;
