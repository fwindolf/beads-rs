//! Configuration management for the beads system.
//!
//! This crate handles loading and saving `.beads/config.yaml` files,
//! discovering `.beads/` directories in the filesystem, and providing
//! typed access to beads configuration values.
//!
//! Ported from the Go `internal/config` and `internal/beads` packages.

pub mod beads_dir;
pub mod config;
