use core::time::Duration;

use super::output::stderr_line;

pub(crate) fn started(name: &str) {
    stderr_line(format_args!("axiom: checking {name}..."));
}

pub(crate) fn finished(name: &str, elapsed: Duration) {
    let elapsed = if elapsed.as_secs() == 0 {
        format!("{}ms", elapsed.as_millis())
    } else {
        format!("{:.1}s", elapsed.as_secs_f64())
    };
    stderr_line(format_args!("axiom: finished {name} in {elapsed}"));
}

pub(crate) fn failed(name: &str) {
    stderr_line(format_args!("axiom: {name} failed"));
}
