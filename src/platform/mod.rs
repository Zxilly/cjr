//! Audited native operations absent from the standard library and safe dependencies.
//! No binary parsing, network, cache policy or application orchestration belongs here.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
pub(crate) mod windows;

#[cfg(unix)]
pub(crate) use unix::{boot_identity, glibc_version};
#[cfg(windows)]
pub(crate) use windows::boot_identity;

pub(crate) fn try_lock(file: &std::fs::File) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        rustix::fs::flock(file, rustix::fs::FlockOperation::NonBlockingLockExclusive)
            .map_err(Into::into)
    }
    #[cfg(windows)]
    {
        windows::try_lock(file)
    }
}
