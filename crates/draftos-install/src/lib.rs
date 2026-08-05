//! DraftOS install engine.
//!
//! Turns an [`InstallRequest`] into an ordered, inspectable plan and runs it.
//! The design separates three concerns so the dangerous part stays small and
//! testable:
//!
//! - [`config`] — the request contract (serde), with secrets that never log.
//! - [`files`] — pure generators for the config files written into the system.
//! - [`plan`] — pure: request → ordered [`step::Step`]s (fully unit-tested).
//! - [`exec`] — runs a plan, or dry-runs it; refuses to execute outside a live
//!   environment.
//!
//! The front-end (the installer GUI) serializes an [`InstallRequest`] to JSON and
//! invokes the `draftos-install` binary (via pkexec) to do the privileged work.

pub mod config;
pub mod exec;
pub mod files;
pub mod plan;
pub mod step;

pub use config::InstallRequest;
pub use plan::plan;
pub use step::{Phase, Step};
