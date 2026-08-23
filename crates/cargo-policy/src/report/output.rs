use core::fmt;
use std::io::{IsTerminal as _, Write as _};

use crate::args::ColorChoice;

pub fn stderr_line(arguments: fmt::Arguments<'_>) {
    let _ = writeln!(std::io::stderr(), "{arguments}");
}

pub fn stderr_write(arguments: fmt::Arguments<'_>) {
    let _ = write!(std::io::stderr(), "{arguments}");
}

pub fn color_enabled(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => std::io::stderr().is_terminal(),
    }
}

pub fn severity_color(severity: &str) -> &'static str {
    if severity == "warning" { "33" } else { "31" }
}

pub fn paint(text: &str, code: &str, enabled: bool) -> String {
    if enabled {
        format!("\u{1b}[{code}m{text}\u{1b}[0m")
    } else {
        text.to_owned()
    }
}
