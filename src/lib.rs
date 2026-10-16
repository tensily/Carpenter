//! carpenter — an agent-driven CLI that builds Python/Jupyter learning material.
//!
//! SQLite is the source of truth; notebooks render from it. No embedded LLM:
//! an external agent (opencode) is the tutor; carpenter is deterministic storage,
//! rendering, and execution.

#![deny(missing_docs)]
// In a `--features dev` build the doc/example/scenario gates relax (adr/016).
// `deny` is overridable by a later `allow` (only `forbid` is sticky), so the
// `cfg_attr` MUST come after the `deny` for the override to take effect.
#![cfg_attr(feature = "dev", allow(missing_docs))]

pub mod app;
pub mod commands;
pub mod core;
pub mod manual;
pub mod models;
#[cfg(feature = "tui")]
pub mod tui;
