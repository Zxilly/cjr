//! Ownership of a spawned target until the supervisor has collected its status.
use crate::Result;
use std::process::{Child, ExitStatus};

pub(super) struct SupervisedChild {
    child: Child,
    reaped: bool,
}
impl SupervisedChild {
    pub(super) fn new(child: Child) -> Self {
        Self {
            child,
            reaped: false,
        }
    }
    #[cfg(windows)]
    pub(super) fn inner(&self) -> &Child {
        &self.child
    }
    #[cfg(unix)]
    pub(super) fn id(&self) -> u32 {
        self.child.id()
    }
    #[cfg(windows)]
    pub(super) fn wait(&mut self) -> Result<ExitStatus> {
        let status = self.child.wait()?;
        self.reaped = true;
        Ok(status)
    }
    #[cfg(unix)]
    pub(super) fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        let status = self.child.try_wait()?;
        self.reaped = status.is_some();
        Ok(status)
    }
}
impl Drop for SupervisedChild {
    fn drop(&mut self) {
        if !self.reaped {
            // On an internal supervision failure, do not leave a suspended or
            // unsupervised direct child. Leases remain conservative about descendants.
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
