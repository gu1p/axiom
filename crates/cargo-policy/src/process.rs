#[cfg(unix)]
use core::time::Duration;
use std::process::{Child, Command};

use policy_core::AnalysisError;

#[cfg(unix)]
pub(crate) fn configure_group(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;

    command.process_group(0);
}

#[cfg(not(unix))]
pub(crate) fn configure_group(_command: &mut Command) {}

#[cfg(unix)]
pub(crate) fn terminate_group(child: &mut Child) -> Result<(), AnalysisError> {
    use rustix::process::{Pid, Signal, kill_process_group};

    let pid = Pid::from_child(child);
    let _ = kill_process_group(pid, Signal::TERM);
    for _ in 0..5 {
        let _ = child
            .try_wait()
            .map_err(|error| AnalysisError::new(format!("could not stop process: {error}")))?;
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = kill_process_group(pid, Signal::KILL);
    child
        .wait()
        .map(|_| ())
        .map_err(|error| AnalysisError::new(format!("could not stop process: {error}")))
}

#[cfg(not(unix))]
pub(crate) fn terminate_group(child: &mut Child) -> Result<(), AnalysisError> {
    child
        .kill()
        .and_then(|()| child.wait().map(|_| ()))
        .map_err(|error| AnalysisError::new(format!("could not stop process: {error}")))
}
