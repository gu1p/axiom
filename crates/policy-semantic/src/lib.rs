#![doc(hidden)]

pub mod graph;
pub mod protocol;
pub mod source_path;

#[cfg(feature = "frontend")]
mod cli;
#[cfg(feature = "frontend")]
mod config;
#[cfg(feature = "frontend")]
mod diagnostics;
#[cfg(feature = "frontend")]
mod toolchain;

#[cfg(feature = "frontend")]
pub fn run_frontend(arguments: &[String]) -> std::process::ExitCode {
    use std::io::Write as _;

    match cli::run(arguments.to_owned()) {
        Ok(exit_code) => exit_code,
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::BrokenPipe)
                || error
                    .downcast_ref::<serde_json::Error>()
                    .is_some_and(|error| {
                        error.io_error_kind() == Some(std::io::ErrorKind::BrokenPipe)
                    }) =>
        {
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            if let Err(output_error) = cli::write_error(arguments, &error) {
                let _ = writeln!(
                    std::io::stderr(),
                    "axiom semantic analyzer: {error:#}: {output_error:#}"
                );
            }
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(feature = "frontend")]
pub fn run_rustc_probe(arguments: &[String]) -> Option<std::process::ExitCode> {
    toolchain::run_rustc_probe(arguments)
}

/// Invalidates Axiom's managed semantic cache after an interrupted analysis.
#[cfg(feature = "frontend")]
#[doc(hidden)]
pub fn invalidate_managed_cache(workspace_root: &std::path::Path) -> Result<(), String> {
    cli::invalidate_managed_cache(workspace_root).map_err(|error| format!("{error:#}"))
}

/// Validates a normalized semantic configuration without running compiler analysis.
#[cfg(feature = "frontend")]
#[doc(hidden)]
pub fn validate_config(
    workspace_root: &std::path::Path,
    config_path: &std::path::Path,
) -> Result<(), String> {
    config::Config::load(workspace_root, Some(config_path))
        .map(|_| ())
        .map_err(|error| format!("{error:#}"))
}
