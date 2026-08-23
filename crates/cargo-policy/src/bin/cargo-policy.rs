//! Cargo subcommand compatibility executable for Axiom.

use std::process::ExitCode;

fn main() -> ExitCode {
    cargo_policy::run(std::env::args_os())
}
