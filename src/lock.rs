use crate::{Error, Result};
use std::{
    fs::{File, OpenOptions},
    path::Path,
    time::{Duration, Instant},
};

/// The same whole-file flock / first-byte LockFileEx protocol used by cjv.
/// Closing the handle releases the lock, including on process termination.
pub struct Lock(File);
impl Lock {
    pub fn acquire(path: &Path, timeout: Duration, create: bool) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(create)
            .truncate(false)
            .open(path)
            .map_err(|e| Error::at(path, e))?;
        let start = Instant::now();
        use backon::{BlockingRetryable, ConstantBuilder};
        (|| crate::platform::try_lock(&file))
            .retry(
                ConstantBuilder::default()
                    .with_delay(Duration::from_millis(50))
                    .without_max_times(),
            )
            .when(|e| busy(e) && start.elapsed() < timeout)
            .call()
            .map_err(|e| {
                if busy(&e) {
                    Error::new("CacheBusy", format!("{} is locked", path.display()))
                } else {
                    Error::at(path, e)
                }
            })?;
        Ok(Self(file))
    }
    pub fn file(&self) -> &File {
        &self.0
    }
}

fn busy(e: &std::io::Error) -> bool {
    #[cfg(windows)]
    {
        e.raw_os_error() == Some(33)
    }
    #[cfg(unix)]
    {
        e.raw_os_error() == Some(libc::EWOULDBLOCK) || e.raw_os_error() == Some(libc::EAGAIN)
    }
}
