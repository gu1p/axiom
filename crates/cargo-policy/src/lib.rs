//! Command-line frontend and orchestration for Axiom checks.

mod args;
mod check;
mod init;
mod report;
mod semantic;
mod tools;

use std::{ffi::OsString, process::ExitCode};

use clap::Parser as _;

const VERSION: &str = match option_env!("AXIOM_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

pub fn run(arguments: impl IntoIterator<Item = OsString>) -> ExitCode {
    let arguments: Vec<_> = arguments.into_iter().collect();
    if let Some(code) = semantic_entrypoint(&arguments) {
        return code;
    }
    let cli = args::Cli::parse_from(normalized_args(arguments));
    ExitCode::from(match cli.command {
        args::Command::Check(options) => check::run(&options),
        args::Command::Init(options) => init::run(&options),
    })
}

fn semantic_entrypoint(arguments: &[OsString]) -> Option<ExitCode> {
    let strings: Vec<String> = arguments
        .iter()
        .map(|argument| argument.clone().into_string())
        .collect::<Result<_, _>>()
        .ok()?;
    if let Some(code) = policy_semantic::run_rustc_probe(&strings) {
        return Some(code);
    }
    if strings
        .get(1)
        .is_none_or(|argument| argument != "__semantic")
    {
        return None;
    }
    let mut frontend = strings;
    frontend.remove(1);
    Some(policy_semantic::run_frontend(&frontend))
}

fn normalized_args(arguments: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut arguments: Vec<_> = arguments.into_iter().collect();
    if arguments
        .get(1)
        .is_some_and(|argument| argument == "policy")
    {
        arguments.remove(1);
    }
    arguments
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
