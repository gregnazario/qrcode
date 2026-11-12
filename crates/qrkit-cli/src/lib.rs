//! Library entry for the `qrkit` CLI: argument parsing/mapping and terminal
//! preview. Kept behind a lib target so the logic is integration-testable;
//! the `qrkit` binary in `main.rs` is a thin shell over these modules.

pub mod args;
pub mod preview;
