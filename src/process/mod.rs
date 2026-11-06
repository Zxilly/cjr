mod child;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
use crate::{Result, loader::LaunchPlan, store::Lease};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootIdentity {
    pub host: String,
    pub session: String,
}
pub fn boot_identity() -> Option<BootIdentity> {
    crate::platform::boot_identity()
}
/// Supervision returns status; only the CLI entry point terminates the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}
impl From<std::process::ExitStatus> for Outcome {
    fn from(status: std::process::ExitStatus) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = status.signal() {
                return Self::Signaled(signal);
            }
        }
        Self::Exited(status.code().unwrap_or(1))
    }
}
impl Outcome {
    pub fn terminate(self) -> ! {
        match self {
            Self::Exited(code) => std::process::exit(code),
            Self::Signaled(signal) => {
                #[cfg(unix)]
                {
                    let _ = signal_hook::low_level::emulate_default_handler(signal);
                }
                std::process::exit(128 + signal)
            }
        }
    }
}
fn finish_lease(lease: Option<Lease>, gone: bool) {
    if let Some(lease) = lease
        && let Err(error) = lease.finish(gone)
    {
        crate::diagnostics::emit(format_args!(
            "cjr: retaining usage record after cleanup failure: {error}"
        ));
    }
}
pub fn launch(plan: LaunchPlan, lease: Option<Lease>) -> Result<Outcome> {
    // Signal handlers, console handlers and subreaper state are process-wide.
    static SUPERVISOR: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _exclusive = SUPERVISOR.try_lock().map_err(|_| {
        crate::Error::new(
            "SupervisorBusy",
            "only one supervisor may run in this process",
        )
    })?;
    crate::loader::recheck(&plan)?;
    #[cfg(windows)]
    {
        windows::launch(plan, lease)
    }
    #[cfg(unix)]
    {
        unix::launch(plan, lease)
    }
}
