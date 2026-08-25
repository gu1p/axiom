use core::time::Duration;

mod terminal;

use crate::args::ColorChoice;

use super::output::stderr_line;

pub(crate) fn started(name: &str, choice: ColorChoice) {
    if !terminal::interactive() {
        stderr_line(format_args!("axiom: checking {name}..."));
        return;
    }
    terminal::started(name, choice);
}

pub(crate) fn finished(name: &str, elapsed: Duration) {
    if !terminal::interactive() {
        stderr_line(format_args!(
            "axiom: finished {name} in {}",
            compact_elapsed(elapsed)
        ));
        return;
    }
    terminal::complete(name, Some(elapsed));
}

pub(crate) fn failed(name: &str) {
    if !terminal::interactive() {
        stderr_line(format_args!("axiom: {name} failed"));
        return;
    }
    terminal::complete(name, None);
}

pub(super) fn suspend<T>(operation: impl FnOnce() -> T) -> T {
    terminal::suspend(operation)
}

pub(super) fn clock(duration: Duration) -> String {
    let total = duration.as_secs();
    let minutes = total / 60;
    let seconds = total % 60;
    let tenths = duration.subsec_millis() / 100;
    format!("{minutes:02}:{seconds:02}.{tenths}")
}

pub(super) fn compact_elapsed(elapsed: Duration) -> String {
    if elapsed.as_secs() == 0 {
        format!("{}ms", elapsed.as_millis())
    } else {
        format!("{:.1}s", elapsed.as_secs_f64())
    }
}

#[cfg(test)]
#[path = "../tests/progress.rs"]
mod tests;
