use crate::process::BootIdentity;
pub(crate) fn boot_identity() -> Option<BootIdentity> {
    #[cfg(target_os = "linux")]
    {
        let host = std::fs::read_to_string("/etc/machine-id").ok()?;
        let session = std::fs::read_to_string("/proc/sys/kernel/random/boot_id").ok()?;
        Some(BootIdentity {
            host: crate::util::digest(host.trim().as_bytes()),
            session: session.trim().into(),
        })
    }
    #[cfg(target_os = "macos")]
    {
        fn sysctl(name: &[u8]) -> Option<String> {
            let mut b = [0u8; 256];
            let mut n = b.len();
            // SAFETY: NUL-terminated static name, writable byte buffer and valid size pointer; no retained pointers.
            if unsafe {
                libc::sysctlbyname(
                    name.as_ptr().cast(),
                    b.as_mut_ptr().cast(),
                    &mut n,
                    std::ptr::null_mut(),
                    0,
                )
            } != 0
            {
                return None;
            }
            let c = std::ffi::CStr::from_bytes_until_nul(&b[..n]).ok()?;
            Some(c.to_string_lossy().into_owned())
        }
        let mut host = [0u8; 16];
        let timeout = libc::timespec {
            tv_sec: 1,
            tv_nsec: 0,
        };
        // SAFETY: gethostuuid writes exactly 16 bytes; timeout is a valid immutable timespec.
        if unsafe { libc::gethostuuid(host.as_mut_ptr(), &timeout) } != 0 {
            return None;
        }
        let session = sysctl(b"kern.bootsessionuuid\0")?;
        Some(BootIdentity {
            host: crate::util::digest(&host),
            session,
        })
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
pub(crate) fn glibc_version() -> Option<String> {
    unsafe extern "C" {
        fn gnu_get_libc_version() -> *const libc::c_char;
    }
    // SAFETY: glibc returns a process-lifetime, NUL-terminated static version string.
    let value = unsafe { std::ffi::CStr::from_ptr(gnu_get_libc_version()) };
    Some(value.to_string_lossy().into_owned())
}
