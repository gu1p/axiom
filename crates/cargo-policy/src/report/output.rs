use core::fmt;
use std::io::Write as _;

pub fn stderr_line(arguments: fmt::Arguments<'_>) {
    let _ = writeln!(std::io::stderr(), "{arguments}");
}

pub fn stderr_write(arguments: fmt::Arguments<'_>) {
    let _ = write!(std::io::stderr(), "{arguments}");
}
