// Enables ANSI escape interpretation on legacy Windows consoles (cmd.exe,
// older PowerShell hosts) that don't default to it

#[cfg(windows)]
mod imp {
    use std::os::raw::{c_int, c_void};

    const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5; // -11 as u32, per WinBase.h.
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(nStdHandle: u32) -> *mut c_void;
        fn GetConsoleMode(hConsoleHandle: *mut c_void, lpMode: *mut u32) -> c_int;
        fn SetConsoleMode(hConsoleHandle: *mut c_void, dwMode: u32) -> c_int;
    }

    pub fn enable() {
        // Safety: GetStdHandle with a well-known standard handle constant
        // never fails in a way that produces an invalid pointer other than
        // INVALID_HANDLE_VALUE or NULL, both checked before dereference via
        // GetConsoleMode. No aliasing: mode is a local, exclusively borrowed
        // for the duration of the two calls.
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                return;
            }

            let mut mode: u32 = 0;
            if GetConsoleMode(handle, &mut mode) == 0 {
                return;
            }

            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn enable() {}
}

pub fn enable_ansi_support() {
    imp::enable();
}
