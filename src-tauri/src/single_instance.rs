// Session-local mutex prevents duplicate workers; an auto-reset event queues activation
// even if the second launch arrives before the first dashboard has been created.
#[cfg(windows)]
pub struct Instance {
    mutex: windows_sys::Win32::Foundation::HANDLE,
    activation: windows_sys::Win32::Foundation::HANDLE,
}
#[cfg(windows)]
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.activation);
            windows_sys::Win32::Foundation::CloseHandle(self.mutex);
        }
    }
}
#[cfg(windows)]
const EVENT: &str = "Local\\NOVA.Desktop.Activate";
#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
#[cfg(windows)]
pub fn acquire() -> Result<Option<Instance>, String> {
    use windows_sys::Win32::{Foundation::*, System::Threading::*};
    unsafe {
        let activation = CreateEventW(std::ptr::null(), 0, 0, wide(EVENT).as_ptr());
        if activation.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mutex = CreateMutexW(
            std::ptr::null(),
            0,
            wide("Local\\NOVA.Desktop.Singleton").as_ptr(),
        );
        if mutex.is_null() {
            let error = std::io::Error::last_os_error().to_string();
            CloseHandle(activation);
            return Err(error);
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            if !std::env::args_os().any(|arg| arg == "--background") {
                SetEvent(activation);
            }
            CloseHandle(activation);
            CloseHandle(mutex);
            return Ok(None);
        }
        Ok(Some(Instance { mutex, activation }))
    }
}
#[cfg(windows)]
pub fn listen(app: &tauri::AppHandle) -> Result<(), String> {
    use windows_sys::Win32::{Foundation::*, System::Threading::*};
    let handle = unsafe { CreateEventW(std::ptr::null(), 0, 0, wide(EVENT).as_ptr()) };
    if handle.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let raw = handle as usize;
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("nova-activation".into())
        .spawn(move || {
            let handle = raw as HANDLE;
            loop {
                if unsafe { WaitForSingleObject(handle, INFINITE) } != WAIT_OBJECT_0 {
                    break;
                }
                let dashboard = app.clone();
                if app
                    .run_on_main_thread(move || crate::tray::show_dashboard(&dashboard))
                    .is_err()
                {
                    break;
                }
                crate::local_log::event("runtime", "duplicate_launch_activated");
            }
            unsafe {
                CloseHandle(handle);
            }
        });
    if let Err(error) = spawned {
        unsafe {
            CloseHandle(handle);
        }
        return Err(error.to_string());
    }
    Ok(())
}
#[cfg(not(windows))]
pub struct Instance;
#[cfg(not(windows))]
pub fn acquire() -> Result<Option<Instance>, String> {
    Ok(Some(Instance))
}
#[cfg(not(windows))]
pub fn listen(_: &tauri::AppHandle) -> Result<(), String> {
    Ok(())
}
