//! Unix supervision through safe rustix and signal-hook APIs.
use super::{Outcome, child::SupervisedChild, finish_lease};
use crate::{Error, Result, loader::LaunchPlan, store::Lease};
use rustix::process::{Pid, Signal};
use signal_hook::{consts::signal::*, iterator::Signals};
use std::os::unix::process::CommandExt;
use std::process::Command;

fn set_foreground(group: Pid) -> std::io::Result<()> {
    use nix::sys::signal::{SigSet, SigmaskHow, Signal as NixSignal};
    // POSIX permits a background tcsetpgrp only with SIGTTOU blocked or ignored.
    // Catching it is insufficient. Block only on this thread and only for the
    // ioctl, preserving the prior mask on every exit path.
    let mask = SigSet::from(NixSignal::SIGTTOU);
    let previous = mask.thread_swap_mask(SigmaskHow::SIG_BLOCK)?;
    let restore = scopeguard::guard(previous, |mask| {
        if let Err(e) = mask.thread_set_mask() {
            crate::diagnostics::emit(format_args!("cjr: restoring signal mask failed: {e}"));
        }
    });
    let result = rustix::termios::tcsetpgrp(std::io::stdin(), group).map_err(std::io::Error::from);
    drop(restore);
    result
}

/// Restore terminal ownership on every return path, before unregistering signals.
struct Foreground {
    previous: Option<Pid>,
}
impl Foreground {
    fn transfer(child: Pid) -> Result<Self> {
        let input = std::io::stdin();
        let current = match rustix::termios::tcgetpgrp(&input) {
            Ok(p) => p,
            Err(rustix::io::Errno::NOTTY | rustix::io::Errno::BADF) => {
                return Ok(Self { previous: None });
            }
            Err(e) => return Err(std::io::Error::from(e).into()),
        };
        if current != rustix::process::getpgrp() {
            return Ok(Self { previous: None });
        }
        set_foreground(child)?;
        let guard = Self {
            previous: Some(current),
        };
        forward(child, Signal::CONT)?;
        Ok(guard)
    }
}
impl Drop for Foreground {
    fn drop(&mut self) {
        if let Some(previous) = self.previous
            && let Err(e) = set_foreground(previous)
        {
            crate::diagnostics::emit(format_args!(
                "cjr: restoring terminal foreground failed: {e}"
            ));
        }
    }
}
fn forward(group: Pid, signal: Signal) -> Result<()> {
    match rustix::process::kill_process_group(group, signal) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(e) => Err(std::io::Error::from(e).into()),
    }
}

#[cfg(target_os = "linux")]
struct Subreaper {
    previous: Option<Pid>,
    enabled: bool,
}
#[cfg(target_os = "linux")]
impl Subreaper {
    fn acquire() -> Self {
        match rustix::process::child_subreaper() {
            Ok(previous) => Self {
                previous,
                enabled: rustix::process::set_child_subreaper(Some(Pid::INIT)).is_ok(),
            },
            Err(_) => Self {
                previous: None,
                enabled: false,
            },
        }
    }
    fn descendants_gone(&self) -> bool {
        use rustix::process::{WaitId, WaitIdOptions, waitid};
        // Observe without reaping: do not consume another owner's child status.
        self.enabled
            && matches!(
                waitid(
                    WaitId::All,
                    WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT
                ),
                Err(rustix::io::Errno::CHILD)
            )
    }
}
#[cfg(target_os = "linux")]
impl Drop for Subreaper {
    fn drop(&mut self) {
        if self.enabled {
            let _ = rustix::process::set_child_subreaper(self.previous);
        }
    }
}

pub(super) fn launch(plan: LaunchPlan, mut lease: Option<Lease>) -> Result<Outcome> {
    // Distinct pending signals are preserved; SIGCHLD wakes the child-status check.
    let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP, SIGQUIT, SIGCHLD])?;
    #[cfg(target_os = "linux")]
    let reaper = Subreaper::acquire();
    let mut command = Command::new(&plan.binary);
    command
        .args(&plan.args)
        .envs(plan.environment.iter().map(|(k, v)| (k, v)))
        .process_group(0);
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
    let group = Pid::from_raw(child.id() as i32)
        .ok_or_else(|| Error::new("LaunchFailed", "invalid child process ID"))?;
    let _foreground = Foreground::transfer(group)?;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        for signal in signals.wait() {
            let signal = match signal {
                SIGINT => Signal::INT,
                SIGTERM => Signal::TERM,
                SIGHUP => Signal::HUP,
                SIGQUIT => Signal::QUIT,
                _ => continue,
            };
            forward(group, signal)?;
        }
    };
    #[cfg(target_os = "linux")]
    let gone = reaper.descendants_gone();
    #[cfg(not(target_os = "linux"))]
    let gone = false;
    finish_lease(lease, gone);
    Ok(Outcome::from(status))
}
