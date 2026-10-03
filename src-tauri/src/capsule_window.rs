#[tauri::command]
pub fn raise_capsule(window: tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "capsule" {
        return Err("Only the recording capsule can request this operation".into());
    }
    if !window.is_visible().map_err(|error| error.to_string())? {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    return raise_windows(window.hwnd().map_err(|error| error.to_string())?.0)
        .map_err(|error| error.to_string());

    #[cfg(target_os = "macos")]
    return window
        .with_webview(|platform| unsafe {
            use objc2::{
                msg_send,
                runtime::{AnyObject, Bool},
            };
            let native = platform.ns_window().cast::<AnyObject>();
            let visible: Bool = msg_send![native, isVisible];
            if !visible.as_bool() {
                return;
            }
            let level = CGWindowLevelForKey(9) as isize;
            let _: () = msg_send![native, setLevel: level];
            let _: () = msg_send![native, orderFrontRegardless];
        })
        .map_err(|error| error.to_string());

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    window
        .set_always_on_top(true)
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowLevelForKey(key: i32) -> i32;
}

#[cfg(target_os = "windows")]
fn raise_windows(handle: windows_sys::Win32::Foundation::HWND) -> std::io::Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsWindowVisible, SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER,
        SWP_NOSIZE,
    };
    unsafe {
        if IsWindowVisible(handle) == 0 {
            return Ok(());
        }
        if SetWindowPos(
            handle,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOOWNERZORDER,
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::raise_windows;
    use windows_sys::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GetForegroundWindow, GetWindow, GetWindowLongW,
            IsWindowVisible, SetWindowPos, ShowWindow, GWL_EXSTYLE, GW_HWNDPREV, HWND_TOPMOST,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
        },
    };

    struct TestWindow(HWND);

    impl TestWindow {
        fn new() -> Self {
            let handle = unsafe {
                CreateWindowExW(
                    WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                    windows_sys::core::w!("STATIC"),
                    windows_sys::core::w!("Ultra STT stacking regression test"),
                    WS_POPUP,
                    20,
                    20,
                    80,
                    80,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null(),
                )
            };
            assert!(!handle.is_null());
            Self(handle)
        }
    }

    impl Drop for TestWindow {
        fn drop(&mut self) {
            unsafe { DestroyWindow(self.0) };
        }
    }

    fn is_above(front: HWND, back: HWND) -> bool {
        let mut current = back;
        for _ in 0..1000 {
            current = unsafe { GetWindow(current, GW_HWNDPREV) };
            if current == front {
                return true;
            }
            if current.is_null() {
                return false;
            }
        }
        false
    }

    #[test]
    fn restores_native_z_order_without_activating_or_reopening_hidden_windows() {
        let capsule = TestWindow::new();
        let covering = TestWindow::new();
        let foreground = unsafe { GetForegroundWindow() };
        raise_windows(capsule.0).unwrap();
        assert_eq!(unsafe { IsWindowVisible(capsule.0) }, 0);

        for _ in 0..3 {
            unsafe {
                ShowWindow(capsule.0, SW_SHOWNOACTIVATE);
                ShowWindow(covering.0, SW_SHOWNOACTIVATE);
                assert_ne!(
                    SetWindowPos(
                        covering.0,
                        HWND_TOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
                    ),
                    0
                );
            }
            assert!(is_above(covering.0, capsule.0));
            raise_windows(capsule.0).unwrap();
            assert!(is_above(capsule.0, covering.0));
            assert_ne!(
                unsafe { GetWindowLongW(capsule.0, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST,
                0
            );
            assert_eq!(unsafe { GetForegroundWindow() }, foreground);
            unsafe { ShowWindow(capsule.0, SW_HIDE) };
            raise_windows(capsule.0).unwrap();
            assert_eq!(unsafe { IsWindowVisible(capsule.0) }, 0);
        }
    }
}
