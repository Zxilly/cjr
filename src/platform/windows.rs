use crate::process::BootIdentity;
use std::fs::File;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE},
    System::{Console::*, Diagnostics::ToolHelp::*, JobObjects::*, Threading::*},
};

fn owned(handle: HANDLE) -> std::io::Result<OwnedHandle> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: only fresh owned handles from successful Win32 creation/open calls
    // enter this function. Sentinels were rejected; OwnedHandle closes exactly once.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

pub(crate) struct Job(OwnedHandle);
impl Job {
    pub(crate) fn new() -> std::io::Result<Self> {
        // SAFETY: null optional security/name pointers create an unnamed private job.
        owned(unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) }).map(Self)
    }
    pub(crate) fn assign(&self, child: &std::process::Child) -> std::io::Result<()> {
        // SAFETY: both handles remain alive for this synchronous call.
        if unsafe { AssignProcessToJobObject(self.0.as_raw_handle(), child.as_raw_handle()) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    pub(crate) fn is_empty(&self) -> std::io::Result<bool> {
        // SAFETY: the Win32 accounting POD permits zero initialization.
        let mut accounting: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: correctly sized and aligned output struct; the job handle is borrowed.
        let ok = unsafe {
            QueryInformationJobObject(
                self.0.as_raw_handle(),
                JobObjectBasicAccountingInformation,
                (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                std::mem::size_of_val(&accounting) as u32,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(accounting.ActiveProcesses == 0)
    }
}
pub(crate) fn resume(child: &std::process::Child) -> std::io::Result<()> {
    // SAFETY: snapshot has no borrowed inputs and returns an owned handle.
    let snapshot = owned(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) })?;
    // SAFETY: THREADENTRY32 is a Win32 POD; dwSize is initialized before use.
    let mut entry: THREADENTRY32 = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of_val(&entry) as u32;
    // SAFETY: entry is a correctly sized writable buffer; snapshot is live.
    let mut ok = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) };
    while ok != 0 {
        if entry.th32OwnerProcessID == child.id() {
            // SAFETY: opening an OS-provided ID returns a fresh owned handle; access is minimal.
            let thread =
                owned(unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) })?;
            // SAFETY: live thread handle with the required right; target was created suspended.
            if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
                return Err(std::io::Error::last_os_error());
            }
            return Ok(());
        }
        // SAFETY: same initialized snapshot and writable entry as Thread32First.
        ok = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) };
    }
    Err(std::io::Error::other(
        "could not locate the suspended target thread",
    ))
}
// SAFETY: invoked by Windows with a DWORD event. No allocation, locks, panics,
// borrowed data or calls into managed runtime occur on the callback thread.
unsafe extern "system" fn ctrl(kind: u32) -> i32 {
    if kind == CTRL_C_EVENT || kind == CTRL_BREAK_EVENT {
        1
    } else {
        0
    }
}
pub(crate) struct ConsoleHandler;
impl ConsoleHandler {
    pub(crate) fn install() -> std::io::Result<Self> {
        // SAFETY: callback has static lifetime and the documented system ABI.
        if unsafe { SetConsoleCtrlHandler(Some(ctrl), 1) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self)
    }
}
impl Drop for ConsoleHandler {
    fn drop(&mut self) {
        // SAFETY: unregister precisely the static callback registered by this guard.
        unsafe {
            SetConsoleCtrlHandler(Some(ctrl), 0);
        }
    }
}

pub(crate) fn boot_identity() -> Option<BootIdentity> {
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQuerySystemInformation(
            class: u32,
            buffer: *mut std::ffi::c_void,
            len: u32,
            returned: *mut u32,
        ) -> i32;
    }
    #[repr(C, align(8))]
    struct BootBuffer([u8; 64]);
    let mut buffer = BootBuffer([0; 64]);
    let b = &mut buffer.0;
    // SystemBootEnvironmentInformation begins with the per-boot GUID.
    // SAFETY: aligned writable buffer of the supplied size; class 90 writes
    // SYSTEM_BOOT_ENVIRONMENT_INFORMATION. No borrowed pointer survives this call.
    if unsafe {
        NtQuerySystemInformation(
            90,
            b.as_mut_ptr().cast(),
            b.len() as u32,
            std::ptr::null_mut(),
        )
    } < 0
    {
        return None;
    }
    use windows_sys::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6464KEY, RegGetValueW,
    };
    let key: Vec<u16> = "SOFTWARE\\Microsoft\\Cryptography\0"
        .encode_utf16()
        .collect();
    let name: Vec<u16> = "MachineGuid\0".encode_utf16().collect();
    let mut host = [0u16; 128];
    let mut size = std::mem::size_of_val(&host) as u32;
    // SAFETY: NUL-terminated key/value names, correctly sized UTF-16 output;
    // registry access is read-only and the output is decoded only after success.
    if unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY,
            std::ptr::null_mut(),
            host.as_mut_ptr().cast(),
            &mut size,
        )
    } != 0
    {
        return None;
    }
    let host = String::from_utf16(&host[..host.iter().position(|c| *c == 0)?]).ok()?;
    Some(BootIdentity {
        host: crate::util::digest(host.as_bytes()),
        session: crate::util::hex(&b[..16]),
    })
}

pub(crate) fn try_lock(file: &File) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Storage::FileSystem::{LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx},
        System::IO::OVERLAPPED,
    };
    // SAFETY: OVERLAPPED is a C POD; synchronous byte-range locking needs all-zero offsets.
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    // SAFETY: live synchronous file handle and valid zeroed OVERLAPPED; lock byte 0.
    if unsafe {
        LockFileEx(
            file.as_raw_handle(),
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            1,
            0,
            &mut overlapped,
        )
    } != 0
    {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
