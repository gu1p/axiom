mod args;
mod check;
mod init;
mod report;

use std::{ffi::OsString, process::ExitCode};

use clap::Parser;

pub const VERSION: &str = match option_env!("AXIOM_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

pub fn run(arguments: impl IntoIterator<Item = OsString>) -> ExitCode {
    let cli = args::Cli::parse_from(normalized_args(arguments));
    ExitCode::from(match cli.command {
        args::Command::Check(options) => check::run(&options),
        args::Command::Init(options) => init::run(&options),
    })
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
mod tests {
    use std::ffi::OsString;

    use super::normalized_args;

    #[test]
    fn strips_cargo_subcommand_argument() {
        let args = ["cargo-policy", "policy", "check"].map(OsString::from);
        assert_eq!(normalized_args(args), ["cargo-policy", "check"]);
    }

    #[test]
    fn preserves_direct_arguments() {
        let args = ["axiom", "check"].map(OsString::from);
        assert_eq!(normalized_args(args.clone()), args);
    }
}
