//! Windows supervision policy. Native handles and FFI stay in platform/windows.
use super::{Outcome, child::SupervisedChild, finish_lease};
use crate::{
    Error, Result,
    loader::LaunchPlan,
    platform::windows::{ConsoleHandler, Job, resume},
    store::Lease,
};
use std::{os::windows::process::CommandExt, process::Command};

pub(super) fn launch(plan: LaunchPlan, mut lease: Option<Lease>) -> Result<Outcome> {
    // No KILL_ON_JOB_CLOSE: normal termination of cjr must not kill a daemon.
    let job = Job::new()?;
    let _handler = ConsoleHandler::install()?;
    let mut command = Command::new(&plan.binary);
    command
        .args(&plan.args)
        .envs(plan.environment.iter().map(|(k, v)| (k, v)))
        .creation_flags(windows_sys::Win32::System::Threading::CREATE_SUSPENDED);
    if let Some(lease) = lease.as_mut() {
        lease.mark_started();
    }
    let child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            finish_lease(lease, true);
            return Err(Error::at(&plan.binary, e));
        }
    };
    let mut child = SupervisedChild::new(child);
    job.assign(child.inner())?;
    resume(child.inner())?;
    let status = child.wait()?;
    finish_lease(lease, job.is_empty().unwrap_or(false));
    Ok(Outcome::from(status))
}
