//! Core (non-CLI) logic: error hierarchy, envelope output, filesystem/storage,
//! status derivation.

pub mod bugfile;
pub mod compare;
pub mod config;
pub mod confirm;
pub mod db;
pub mod error;
pub mod exec;
pub mod helper;
pub mod installs;
pub mod notebook;
pub mod output;
pub mod reconcile;
pub mod release;
pub mod skill;
pub mod status;
pub mod store;
pub mod time;
pub mod verify;

// Dev-build-only helpers (adr/016). Compiled solely under the `dev` feature so
// the surface never reaches a release binary or the generated howto/skill.
#[cfg(feature = "dev")]
pub mod dev;
