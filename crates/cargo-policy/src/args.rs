use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "axiom", bin_name = "axiom", version = crate::VERSION)]
#[command(about = "Enforce executable engineering policies for Rust workspaces")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check the workspace against configured policies.
    Check(CheckOptions),
    /// Create a default policy.toml without overwriting existing configuration.
    Init(InitOptions),
}

#[derive(Debug, Args)]
pub struct CheckOptions {
    /// Path to the Cargo manifest used to discover the workspace.
    #[arg(long)]
    pub manifest_path: Option<Utf8PathBuf>,
    /// Policy file to use instead of <workspace>/policy.toml.
    #[arg(long)]
    pub config: Option<Utf8PathBuf>,
    /// Diagnostic output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,
    /// Colorize human diagnostics.
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,
}

#[derive(Debug, Args)]
pub struct InitOptions {
    /// Path to the Cargo manifest used to discover the workspace.
    #[arg(long)]
    pub manifest_path: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}
