#[cfg(windows)]
mod windows_platform {
    use std::env;

    use anyhow::{Result, bail};
    use eframe::Frame;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::{
        Win32::{
            Foundation::{CloseHandle, GetLastError, HANDLE, HWND},
            Graphics::Dwm::{
                DWMNCRP_DISABLED, DWMNCRP_ENABLED, DWMWA_NCRENDERING_POLICY,
                DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DEFAULT, DWMWCP_ROUND,
                DwmSetWindowAttribute,
            },
            System::Threading::{
                CreateMutexW, GetCurrentProcess, HIGH_PRIORITY_CLASS, SetPriorityClass,
            },
            UI::{
                Shell::{IsUserAnAdmin, ShellExecuteW},
                WindowsAndMessaging::SW_SHOWNORMAL,
            },
        },
        core::{PCWSTR, w},
    };

    const MUTEX_NAME: &str = "Global\\CrosshairOverlaySingleInstance";

    fn spawn_popup_arg(arg: &str) {
        if let Ok(exe) = env::current_exe() {
            let exe_wide = widestring(exe.as_os_str().to_string_lossy().as_ref());
            let arg_wide = widestring(arg);
            unsafe {
                let _ = ShellExecuteW(
                    Some(HWND(std::ptr::null_mut())),
                    w!("open"),
                    PCWSTR(exe_wide.as_ptr()),
                    PCWSTR(arg_wide.as_ptr()),
                    PCWSTR::null(),
                    SW_SHOWNORMAL,
                );
            }
        }
    }

    pub struct SingleInstanceGuard {
        handle: HANDLE,
    }

    impl Drop for SingleInstanceGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }

    pub fn relaunch_as_admin_if_needed() -> Result<bool> {
        unsafe {
            if IsUserAnAdmin().as_bool() {
                return Ok(false);
            }
        }

        let exe = env::current_exe()?;
        let exe_wide = widestring(exe.as_os_str().to_string_lossy().as_ref());
        unsafe {
            let result = ShellExecuteW(
                Some(HWND(std::ptr::null_mut())),
                w!("runas"),
                PCWSTR(exe_wide.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            if (result.0 as usize) <= 32 {
                bail!("Administrator elevation was cancelled or failed");
            }
        }
        Ok(true)
    }

    pub fn acquire_single_instance() -> Result<Option<SingleInstanceGuard>> {
        let name = widestring(MUTEX_NAME);
        let handle = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr()))? };
        let already_exists =
            unsafe { GetLastError().0 } == windows::Win32::Foundation::ERROR_ALREADY_EXISTS.0;
        if already_exists {
            spawn_popup_arg("--already-running-popup");
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Ok(None);
        }

        Ok(Some(SingleInstanceGuard { handle }))
    }

    pub fn set_high_priority() {
        unsafe {
            let _ = SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);
        }
    }

    pub fn set_native_window_shadow(frame: &Frame, enabled: bool) {
        let Ok(window_handle) = frame.window_handle() else {
            return;
        };
        let hwnd = match window_handle.as_raw() {
            RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as *mut _),
            _ => return,
        };

        unsafe {
            let policy = if enabled {
                DWMNCRP_ENABLED
            } else {
                DWMNCRP_DISABLED
            };
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_NCRENDERING_POLICY,
                &policy as *const _ as *const _,
                std::mem::size_of_val(&policy) as u32,
            );

            let corner = if enabled {
                DWMWCP_ROUND
            } else {
                DWMWCP_DEFAULT
            };
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner as *const _ as *const _,
                std::mem::size_of_val(&corner) as u32,
            );
        }
    }

    fn widestring(value: &str) -> Vec<u16> {
        let mut wide: Vec<u16> = value.encode_utf16().collect();
        wide.push(0);
        wide
    }
}

#[cfg(windows)]
pub use windows_platform::*;

#[cfg(not(windows))]
mod fallback {
    use anyhow::Result;
    use eframe::Frame;

    pub struct SingleInstanceGuard;

    pub fn relaunch_as_admin_if_needed() -> Result<bool> {
        Ok(false)
    }

    pub fn acquire_single_instance() -> Result<Option<SingleInstanceGuard>> {
        Ok(Some(SingleInstanceGuard))
    }

    pub fn set_high_priority() {}

    pub fn set_native_window_shadow(_frame: &Frame, _enabled: bool) {}
}

#[cfg(not(windows))]
pub use fallback::*;
