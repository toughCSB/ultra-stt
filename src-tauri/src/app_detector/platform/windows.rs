use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::sync::mpsc::{self, SyncSender};
use std::sync::OnceLock;
use std::time::Duration;

use super::ContextSignalSource;
use crate::app_detector::types::{
    BrowserAccessStatus, ContextSignals, FocusIdentity, TargetAppGuard,
};

pub struct WindowsContextSource;

pub(crate) fn restore_target_application(target: &TargetAppGuard) -> bool {
    let Some(process_id) = target.process_id else {
        return false;
    };
    let Some(window) = target.target_window else {
        return false;
    };
    unsafe {
        restore_process_window(
            process_id,
            window as usize as windows_sys::Win32::Foundation::HWND,
        )
    }
}
unsafe fn restore_process_window(
    process_id: u32,
    window: windows_sys::Win32::Foundation::HWND,
) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging as wm;
    if window.is_null() || wm::IsWindow(window) == 0 || wm::IsWindowVisible(window) == 0 {
        return false;
    }
    let mut actual_pid = 0;
    wm::GetWindowThreadProcessId(window, &mut actual_pid);
    if actual_pid != process_id || wm::GetAncestor(window, wm::GA_ROOT) != window {
        return false;
    }
    wm::ShowWindow(window, wm::SW_RESTORE);
    wm::SetForegroundWindow(window) != 0
}

impl ContextSignalSource for WindowsContextSource {
    fn requires_focus_guard(&self) -> bool {
        true
    }

    fn collect(&self) -> Option<ContextSignals> {
        unsafe { collect_foreground_context() }
    }

    fn current_target_guard(&self) -> Option<TargetAppGuard> {
        unsafe { foreground_target_guard() }
    }
}

unsafe fn foreground_target_guard() -> Option<TargetAppGuard> {
    use windows_sys::Win32::UI::WindowsAndMessaging as wm;
    let window = wm::GetForegroundWindow();
    if window.is_null() {
        return None;
    }
    let mut process_id = 0;
    let thread_id = wm::GetWindowThreadProcessId(window, &mut process_id);
    if process_id == 0 || thread_id == 0 {
        return None;
    }
    let focus_window = focused_child_window(window, thread_id);
    let alias = process_name(process_id);
    let focus = uia_focused_runtime_id(process_id)
        .map(FocusIdentity::WindowsUiAutomation)
        .or_else(|| {
            focus_window
                .filter(|hwnd| is_native_edit_control(*hwnd))
                .map(FocusIdentity::WindowsControl)
        });
    // UIA may have blocked on a provider while the user changed windows/fields.
    let still_same = wm::GetForegroundWindow() == window
        && focused_child_window(window, thread_id) == focus_window;
    Some(TargetAppGuard {
        process_id: Some(process_id),
        native_identity: alias,
        target_window: Some(window as usize as u64),
        target_focus: if still_same && focus_window.is_some() {
            focus
        } else {
            None
        },
        focus_required: true,
    })
}

unsafe fn focused_child_window(
    window: windows_sys::Win32::Foundation::HWND,
    thread_id: u32,
) -> Option<u64> {
    use windows_sys::Win32::UI::WindowsAndMessaging as wm;
    let mut info = wm::GUITHREADINFO {
        cbSize: std::mem::size_of::<wm::GUITHREADINFO>() as u32,
        ..std::mem::zeroed()
    };
    if wm::GetGUIThreadInfo(thread_id, &mut info) == 0
        || info.hwndFocus.is_null()
        || wm::GetAncestor(info.hwndFocus, wm::GA_ROOT) != window
    {
        return None;
    }
    Some(info.hwndFocus as usize as u64)
}

unsafe fn is_native_edit_control(hwnd: u64) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetClassNameW;
    let mut class_name = [0u16; 64];
    let len = GetClassNameW(
        hwnd as usize as _,
        class_name.as_mut_ptr(),
        class_name.len() as i32,
    );
    if len <= 0 {
        return false;
    }
    let class = String::from_utf16_lossy(&class_name[..len as usize]);
    matches!(
        class.to_ascii_lowercase().as_str(),
        "edit" | "richedit20w" | "richedit50w"
    )
}

type UiRequest = (u32, mpsc::Sender<Option<Vec<i32>>>);

fn uia_focused_runtime_id(process_id: u32) -> Option<Vec<i32>> {
    static WORKER: OnceLock<SyncSender<UiRequest>> = OnceLock::new();
    let worker = WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::sync_channel::<UiRequest>(1);
        std::thread::Builder::new()
            .name("target-focus-uia".into())
            .spawn(move || {
                use windows::Win32::System::Com::{
                    CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED,
                };
                let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
                for (pid, reply) in rx {
                    let result = if initialized {
                        unsafe { uia_runtime_id(pid) }
                    } else {
                        None
                    };
                    let _ = reply.send(result);
                }
                if initialized {
                    unsafe {
                        CoUninitialize();
                    }
                }
            })
            .expect("UI Automation worker thread");
        tx
    });
    let (reply_tx, reply_rx) = mpsc::channel();
    worker.try_send((process_id, reply_tx)).ok()?;
    reply_rx
        .recv_timeout(Duration::from_millis(150))
        .ok()
        .flatten()
}

unsafe fn uia_runtime_id(process_id: u32) -> Option<Vec<i32>> {
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::System::Ole::{
        SafeArrayDestroy, SafeArrayGetDim, SafeArrayGetElement, SafeArrayGetLBound,
        SafeArrayGetUBound, SafeArrayGetVartype,
    };
    use windows::Win32::System::Variant::VT_I4;
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};
    let automation: IUIAutomation =
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
    let element = automation.GetFocusedElement().ok()?;
    if element.CurrentProcessId().ok()? != process_id as i32 {
        return None;
    }
    let array = element.GetRuntimeId().ok()?;
    if array.is_null() {
        return None;
    }
    let result = (|| {
        if SafeArrayGetDim(array) != 1 || SafeArrayGetVartype(array).ok()? != VT_I4 {
            return None;
        }
        let first = SafeArrayGetLBound(array, 1).ok()?;
        let last = SafeArrayGetUBound(array, 1).ok()?;
        if last < first || last - first > 63 {
            return None;
        }
        let mut id = Vec::with_capacity((last - first + 1) as usize);
        for index in first..=last {
            let mut value = 0i32;
            SafeArrayGetElement(array, &index, (&mut value as *mut i32).cast()).ok()?;
            id.push(value);
        }
        Some(id)
    })();
    let _ = SafeArrayDestroy(array);
    result
}

unsafe fn collect_foreground_context() -> Option<ContextSignals> {
    let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
    if hwnd.is_null() {
        return None;
    }

    let mut title_buf = [0u16; 512];
    let title_len = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW(
        hwnd,
        title_buf.as_mut_ptr(),
        title_buf.len() as i32,
    );
    let window_title = (title_len > 0).then(|| {
        OsString::from_wide(&title_buf[..title_len as usize])
            .to_string_lossy()
            .to_string()
    });

    let mut process_id = 0u32;
    windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, &mut process_id);
    let process_alias = process_name(process_id);
    let supported_browser = process_alias.as_deref().is_some_and(is_supported_browser);

    Some(ContextSignals {
        process_id: (process_id != 0).then_some(process_id),
        native_identity: process_alias.clone(),
        target_window: Some(hwnd as usize as u64),
        target_focus: None,
        focus_required: true,
        process_alias,
        window_title,
        browser_host: None,
        is_supported_browser: supported_browser,
        browser_access_status: BrowserAccessStatus::for_unavailable_url_adapter(supported_browser),
        browser_target: None,
    })
}

unsafe fn process_name(process_id: u32) -> Option<String> {
    if process_id == 0 {
        return None;
    }
    let handle = windows_sys::Win32::System::Threading::OpenProcess(
        windows_sys::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION,
        0,
        process_id,
    );
    if handle.is_null() {
        return None;
    }

    let mut buffer = [0u16; 260];
    let mut size = buffer.len() as u32;
    let ok = windows_sys::Win32::System::Threading::QueryFullProcessImageNameW(
        handle,
        0,
        buffer.as_mut_ptr(),
        &mut size,
    );
    let _ = windows_sys::Win32::Foundation::CloseHandle(handle);
    if ok == 0 || size == 0 {
        return None;
    }

    let path = OsString::from_wide(&buffer[..size as usize])
        .to_string_lossy()
        .to_string();
    path.rsplit('\\').next().map(str::to_string)
}

fn is_supported_browser(process_name: &str) -> bool {
    matches!(
        process_name.to_ascii_lowercase().as_str(),
        "chrome.exe" | "msedge.exe" | "brave.exe" | "arc.exe"
    )
}
