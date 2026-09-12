#[cfg(windows)]
pub struct Instance(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
#[cfg(windows)]
pub fn acquire() -> Result<Option<Instance>, String> {
    use windows_sys::Win32::{
        Foundation::*, System::Threading::CreateMutexW, UI::WindowsAndMessaging::*,
    };
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    unsafe {
        let handle = CreateMutexW(
            std::ptr::null(),
            0,
            wide("Local\\NOVA.Desktop.Singleton").as_ptr(),
        );
        if handle.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(handle);
            let window = FindWindowW(std::ptr::null(), wide("NOVA").as_ptr());
            if !window.is_null() {
                ShowWindow(window, SW_SHOW);
                SetForegroundWindow(window);
            }
            return Ok(None);
        }
        Ok(Some(Instance(handle)))
    }
}
#[cfg(not(windows))]
pub struct Instance;
#[cfg(not(windows))]
pub fn acquire() -> Result<Option<Instance>, String> {
    Ok(Some(Instance))
}
