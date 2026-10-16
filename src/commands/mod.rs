//! Command implementations.
//!
//! Holds ONLY command fns (`pub fn -> Result<Data, CarpenterError>`); helpers
//! live in `core/`. `build.rs` scans this module by signature, so a helper here
//! with the right signature would break the build.

pub mod bug;
pub mod build;
pub mod config;
pub mod course;
pub mod deregister;
pub mod feature;
pub mod goal;
pub mod howto;
pub mod install;
pub mod installs;
pub mod lesson;
pub mod link;
pub mod notes;
pub mod plan;
pub mod progress;
pub mod quiz;
pub mod register;
pub mod skip;
pub mod uninstall;
pub mod upgrade;
pub mod venv;

#[cfg(test)]
pub(crate) mod testutil;
