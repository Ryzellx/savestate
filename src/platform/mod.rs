pub fn platform_name() -> &'static str {
    #[cfg(windows)]
    {
        "Windows"
    }
    #[cfg(target_os = "linux")]
    {
        "Linux"
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        "Unsupported"
    }
}

/// Re-attach a GUI-subsystem Windows process to the console of the
/// terminal that launched it, so CLI invocations still print output.
///
/// Called only when command-line arguments are present. When the process
/// was started from Explorer there is no parent console, so a fresh one is
/// allocated instead; when neither works (a service, for example), the
/// function silently does nothing and the CLI output is simply lost.
#[cfg(windows)]
pub fn attach_parent_console() {
    use windows::Win32::System::Console::{
        AllocConsole, AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS,
        STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
    };
    unsafe {
        let attached = AttachConsole(ATTACH_PARENT_PROCESS).is_ok() || AllocConsole().is_ok();
        if !attached {
            return;
        }
        // Rust's std print macros write through the Win32 standard handles,
        // so point them at the console we just joined. The CRT descriptor
        // shuffle below keeps any C-level output working too.
        if let Ok(handle) = GetStdHandle(STD_OUTPUT_HANDLE) {
            let _ = SetStdHandle(STD_OUTPUT_HANDLE, handle);
            let fd = libc::open_osfhandle(handle.0 as isize, libc::O_TEXT);
            if fd >= 0 {
                libc::dup2(fd, 1);
            }
        }
        if let Ok(handle) = GetStdHandle(STD_ERROR_HANDLE) {
            let _ = SetStdHandle(STD_ERROR_HANDLE, handle);
            let fd = libc::open_osfhandle(handle.0 as isize, libc::O_TEXT);
            if fd >= 0 {
                libc::dup2(fd, 2);
            }
        }
    }
}

pub fn known_limitations() -> &'static [&'static str] {
    &[
        "Xbox / Microsoft Store games: save locations are sandboxed and not reliably discoverable from metadata; register them manually with a custom save path.",
        "EA App and Ubisoft Connect: no public local manifest format is parsed in this release; games from those launchers can be added manually.",
        "Cloud-only saves (Steam Cloud with no local files) have nothing local to back up until the game writes files to disk.",
    ]
}
