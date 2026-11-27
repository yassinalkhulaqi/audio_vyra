//! Diagnostic CLI for AURIVO.
//!
//! The CLI reads the same JSON files the desktop app writes, which makes it
//! useful for support requests: a user can attach a preset or session dump and
//! a maintainer can inspect it without launching the GUI.

mod commands;
mod format;
mod preset;
mod state;
mod wav;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match commands::run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("aurivo: {message}");
            ExitCode::FAILURE
        }
    }
}
