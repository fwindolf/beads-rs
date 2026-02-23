//! Formula engine for the beads system.
//!
//! Formulas are high-level workflow templates that compile down to proto beads.
//! They support variable definitions with defaults, step definitions that become
//! issue hierarchies, conditions for optional steps, and dependencies between steps.

pub mod engine;
pub mod parser;
pub mod types;
