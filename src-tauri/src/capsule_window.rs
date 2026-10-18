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

#[cfg(test)]
mod target_display_tests {
    use super::{target_display_point, DisplayRect};

    #[test]
    fn chooses_external_display_by_largest_window_overlap_not_primary() {
        let displays = [
            DisplayRect {
                x: 0.0,
                y: 0.0,
                width: 1512.0,
                height: 982.0,
            },
            DisplayRect {
                x: -2560.0,
                y: -400.0,
                width: 2560.0,
                height: 1440.0,
            },
        ];
        let window = DisplayRect {
            x: -800.0,
            y: 100.0,
            width: 1000.0,
            height: 700.0,
        };
        assert_eq!(
            target_display_point(window, &displays),
            Some((-1280.0, 320.0))
        );
        let moved = DisplayRect { x: 50.0, ..window };
        assert_eq!(target_display_point(moved, &displays), Some((756.0, 491.0)));
    }

    #[test]
    fn missing_disconnected_and_invalid_windows_use_fallback() {
        let display = DisplayRect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        };
        assert_eq!(target_display_point(display, &[]), None);
        assert_eq!(
            target_display_point(DisplayRect::default(), &[display]),
            None
        );
        assert_eq!(
            target_display_point(
                DisplayRect {
                    x: 5000.0,
                    ..display
                },
                &[display]
            ),
            None
        );
        assert_eq!(
            target_display_point(
                DisplayRect {
                    x: f64::NAN,
                    ..display
                },
                &[display]
            ),
            None
        );
    }
}
use std::sync::Mutex;

use tauri::Manager;

#[derive(Default)]
pub(crate) struct RecordingMonitor(Mutex<Option<tauri::Monitor>>);

pub(crate) fn capture_recording_monitor(app: &tauri::AppHandle) {
    let monitor =
        foreground_monitor_point().and_then(|(x, y)| app.monitor_from_point(x, y).ok().flatten());
    if let Some(state) = app.try_state::<RecordingMonitor>() {
        *state.0.lock().unwrap_or_else(|error| error.into_inner()) = monitor;
    }
}

#[tauri::command]
pub(crate) fn get_recording_monitor(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, RecordingMonitor>,
) -> Result<Option<tauri::Monitor>, String> {
    if window.label() != "capsule" {
        return Err("Only the recording capsule can request its target display".into());
    }
    Ok(state
        .0
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone())
}

#[cfg(target_os = "windows")]
fn foreground_monitor_point() -> Option<(f64, f64)> {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect},
    };
    let mut bounds = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        let window = GetForegroundWindow();
        if window.is_null() || GetWindowRect(window, &mut bounds) == 0 {
            return None;
        }
    }
    (bounds.right > bounds.left && bounds.bottom > bounds.top).then_some((
        (f64::from(bounds.left) + f64::from(bounds.right)) / 2.0,
        (f64::from(bounds.top) + f64::from(bounds.bottom)) / 2.0,
    ))
}

#[cfg(target_os = "macos")]
fn foreground_monitor_point() -> Option<(f64, f64)> {
    macos_target_display::foreground_monitor_point()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn foreground_monitor_point() -> Option<(f64, f64)> {
    None
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy, Debug, Default)]
struct DisplayRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[cfg(any(target_os = "macos", test))]
fn target_display_point(window: DisplayRect, displays: &[DisplayRect]) -> Option<(f64, f64)> {
    if ![window.x, window.y, window.width, window.height]
        .iter()
        .all(|value| value.is_finite())
        || window.width <= 0.0
        || window.height <= 0.0
    {
        return None;
    }
    displays
        .iter()
        .filter_map(|display| {
            let width =
                (window.x + window.width).min(display.x + display.width) - window.x.max(display.x);
            let height = (window.y + window.height).min(display.y + display.height)
                - window.y.max(display.y);
            let area = width.max(0.0) * height.max(0.0);
            (area.is_finite() && area > 0.0).then_some((display, area))
        })
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(display, _)| {
            (
                display.x + display.width / 2.0,
                display.y + display.height / 2.0,
            )
        })
}

#[cfg(target_os = "macos")]
mod macos_target_display {
    use std::ffi::c_void;

    use objc2::{
        msg_send,
        rc::autoreleasepool,
        runtime::{AnyClass, AnyObject},
    };

    use super::{target_display_point, DisplayRect};

    #[repr(C)]
    #[derive(Default)]
    struct Point {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Default)]
    struct Size {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Default)]
    struct Rect {
        origin: Point,
        size: Size,
    }

    impl From<Rect> for DisplayRect {
        fn from(rect: Rect) -> Self {
            Self {
                x: rect.origin.x,
                y: rect.origin.y,
                width: rect.size.width,
                height: rect.size.height,
            }
        }
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGWindowListCopyWindowInfo(options: u32, relative_to: u32) -> *mut AnyObject;
        static kCGWindowOwnerPID: *const c_void;
        static kCGWindowBounds: *const c_void;
        static kCGWindowLayer: *const c_void;
        fn CGRectMakeWithDictionaryRepresentation(
            dictionary: *const c_void,
            rect: *mut Rect,
        ) -> bool;
        fn CGGetActiveDisplayList(maximum: u32, displays: *mut u32, count: *mut u32) -> i32;
        fn CGDisplayBounds(display: u32) -> Rect;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(value: *const c_void);
        fn CFGetTypeID(value: *const c_void) -> usize;
        fn CFStringCreateWithCString(
            allocator: *const c_void,
            text: *const i8,
            encoding: u32,
        ) -> *const c_void;
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateApplication(pid: i32) -> *const c_void;
        fn AXUIElementGetTypeID() -> usize;
        fn AXValueGetTypeID() -> usize;
        fn AXUIElementSetMessagingTimeout(element: *const c_void, timeout: f32) -> i32;
        fn AXUIElementCopyAttributeValue(
            element: *const c_void,
            attribute: *const c_void,
            value: *mut *const c_void,
        ) -> i32;
        fn AXValueGetValue(value: *const c_void, kind: u32, output: *mut c_void) -> u8;
    }

    struct OwnedCf(*const c_void);

    impl OwnedCf {
        fn new(value: *const c_void) -> Option<Self> {
            (!value.is_null()).then_some(Self(value))
        }
    }

    impl Drop for OwnedCf {
        fn drop(&mut self) {
            unsafe { CFRelease(self.0) };
        }
    }

    unsafe fn copy_attribute(element: *const c_void, name: &std::ffi::CStr) -> Option<OwnedCf> {
        let attribute = OwnedCf::new(CFStringCreateWithCString(
            std::ptr::null(),
            name.as_ptr(),
            0x08000100,
        ))?;
        let mut value = std::ptr::null();
        let status = AXUIElementCopyAttributeValue(element, attribute.0, &mut value);
        let owned = OwnedCf::new(value);
        if status == 0 {
            owned
        } else {
            None
        }
    }

    unsafe fn focused_window_rect(pid: i32) -> Option<DisplayRect> {
        let application = OwnedCf::new(AXUIElementCreateApplication(pid))?;
        if AXUIElementSetMessagingTimeout(application.0, 0.03) != 0 {
            return None;
        }
        let window = copy_attribute(application.0, c"AXFocusedWindow")?;
        if CFGetTypeID(window.0) != AXUIElementGetTypeID()
            || AXUIElementSetMessagingTimeout(window.0, 0.03) != 0
        {
            return None;
        }
        let position = copy_attribute(window.0, c"AXPosition")?;
        let size = copy_attribute(window.0, c"AXSize")?;
        if CFGetTypeID(position.0) != AXValueGetTypeID()
            || CFGetTypeID(size.0) != AXValueGetTypeID()
        {
            return None;
        }
        let mut rect = Rect::default();
        if AXValueGetValue(position.0, 1, (&mut rect.origin as *mut Point).cast()) == 0
            || AXValueGetValue(size.0, 2, (&mut rect.size as *mut Size).cast()) == 0
        {
            return None;
        }
        Some(rect.into())
    }

    unsafe fn point_for_window(rect: DisplayRect) -> Option<(f64, f64)> {
        let mut display_ids = [0u32; 32];
        let mut display_count = 0;
        if CGGetActiveDisplayList(32, display_ids.as_mut_ptr(), &mut display_count) != 0 {
            return None;
        }
        let displays = display_ids[..(display_count as usize).min(display_ids.len())]
            .iter()
            .map(|id| DisplayRect::from(CGDisplayBounds(*id)))
            .collect::<Vec<_>>();
        target_display_point(rect, &displays)
    }

    pub(super) fn foreground_monitor_point() -> Option<(f64, f64)> {
        let workspace_class = AnyClass::get("NSWorkspace")?;
        autoreleasepool(|_| unsafe {
            let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
            if workspace.is_null() {
                return None;
            }
            let application: *mut AnyObject = msg_send![workspace, frontmostApplication];
            if application.is_null() {
                return None;
            }
            let pid: i32 = msg_send![application, processIdentifier];
            if pid <= 0 {
                return None;
            }

            if crate::pipeline::is_accessibility_trusted() {
                if let Some(rect) = focused_window_rect(pid) {
                    return point_for_window(rect);
                }
            }

            let windows = CGWindowListCopyWindowInfo(1 | 16, 0);
            if windows.is_null() {
                return None;
            }
            let result = (|| {
                let count: usize = msg_send![windows, count];
                for index in 0..count {
                    let window: *mut AnyObject = msg_send![windows, objectAtIndex: index];
                    let owner: *mut AnyObject = msg_send![window, objectForKey: kCGWindowOwnerPID];
                    if owner.is_null() {
                        continue;
                    }
                    let owner_pid: i32 = msg_send![owner, intValue];
                    if owner_pid != pid {
                        continue;
                    }
                    let layer: *mut AnyObject = msg_send![window, objectForKey: kCGWindowLayer];
                    if layer.is_null() {
                        continue;
                    }
                    let layer_number: i32 = msg_send![layer, intValue];
                    if layer_number != 0 {
                        continue;
                    }
                    let dictionary: *const c_void =
                        msg_send![window, objectForKey: kCGWindowBounds];
                    if dictionary.is_null() {
                        continue;
                    }
                    let mut rect = Rect::default();
                    if !CGRectMakeWithDictionaryRepresentation(dictionary, &mut rect)
                        || rect.size.width < 1.0
                        || rect.size.height < 1.0
                    {
                        continue;
                    }
                    return point_for_window(rect.into());
                }
                None
            })();
            CFRelease(windows.cast());
            result
        })
    }
}
