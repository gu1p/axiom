//! Primary Axiom command-line executable.

use std::process::ExitCode;

fn main() -> ExitCode {
    cargo_policy::run(std::env::args_os())
}
