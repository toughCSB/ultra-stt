use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use super::ContextSignalSource;
use crate::app_detector::types::{
    BrowserAccessStatus, ContextSignals, FocusIdentity, TargetAppGuard,
};

pub struct WindowsContextSource;

pub(crate) fn capture_selected_text() -> Option<String> {
    use windows_sys::Win32::UI::WindowsAndMessaging as wm;
    unsafe {
        let window = wm::GetForegroundWindow();
        if window.is_null() {
            return None;
        }
        let mut process_id = 0;
        wm::GetWindowThreadProcessId(window, &mut process_id);
        if process_id == 0 {
            return None;
        }
        let text = super::uia_worker::selected_text(process_id, window as usize as u64)?;
        (wm::GetForegroundWindow() == window).then_some(text)
    }
}

pub(super) unsafe fn uia_selected_text(process_id: u32, window: u64) -> Option<String> {
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};
    use windows_sys::Win32::UI::WindowsAndMessaging as wm;
    let expected_window = window as usize as _;
    if wm::GetForegroundWindow() != expected_window {
        return None;
    }
    let automation: IUIAutomation =
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
    let focused = automation.GetFocusedElement().ok()?;
    if focused.CurrentProcessId().ok()? != process_id as i32
        || focused.CurrentIsPassword().ok()?.as_bool()
    {
        return Some(String::new());
    }
    let text = uia_element_selection(&automation, &focused, process_id)?;
    let current = automation.GetFocusedElement().ok()?;
    if wm::GetForegroundWindow() != expected_window
        || !automation
            .CompareElements(&focused, &current)
            .ok()?
            .as_bool()
    {
        return None;
    }
    Some(text)
}

unsafe fn uia_element_selection(
    automation: &windows::Win32::UI::Accessibility::IUIAutomation,
    focused: &windows::Win32::UI::Accessibility::IUIAutomationElement,
    process_id: u32,
) -> Option<String> {
    use windows::Win32::UI::Accessibility::{IUIAutomationTextPattern, UIA_TextPatternId};
    let walker = automation.ControlViewWalker().ok()?;
    let mut element = focused.clone();
    // 브라우저 등은 부모 요소에서 선택 영역의 TextPattern을 제공할 수 있다.
    // 다른 프로세스나 비밀번호 입력란으로는 조회를 확장하지 않는다.
    for _ in 0..8 {
        if element.CurrentProcessId().ok()? != process_id as i32
            || element.CurrentIsPassword().ok()?.as_bool()
        {
            return Some(String::new());
        }
        if let Ok(window) = element.CurrentNativeWindowHandle() {
            if is_native_edit_control(window.0 as usize as u64) {
                return native_edit_selection(window.0 as _, process_id);
            }
        }
        if let Ok(pattern) =
            element.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
        {
            let ranges = pattern.GetSelection().ok()?;
            let count = ranges.Length().ok()?;
            if count == 0 {
                return Some(String::new());
            }
            if !(1..=16).contains(&count) {
                return None;
            }
            let mut selection = String::new();
            for index in 0..count {
                let range = ranges.GetElement(index).ok()?;
                let text = range.GetText(16_385).ok()?.to_string();
                if text.is_empty() {
                    continue;
                }
                if !selection.is_empty() {
                    selection.push('\n');
                }
                selection.push_str(&text);
                if selection.encode_utf16().count() > 16_384 {
                    return None;
                }
            }
            return Some(selection);
        }
        element = walker.GetParentElement(&element).ok()?;
    }
    None
}

unsafe fn native_edit_selection(
    window: windows_sys::Win32::Foundation::HWND,
    process_id: u32,
) -> Option<String> {
    use windows_sys::Win32::UI::WindowsAndMessaging as wm;
    let mut owner = 0;
    wm::GetWindowThreadProcessId(window, &mut owner);
    if owner != process_id
        || wm::GetWindowLongPtrW(window, wm::GWL_STYLE) & wm::ES_PASSWORD as isize != 0
    {
        return None;
    }
    let read_message = |message, wparam, lparam| {
        let mut result = 0usize;
        (wm::SendMessageTimeoutW(
            window,
            message,
            wparam,
            lparam,
            wm::SMTO_ABORTIFHUNG,
            40,
            &mut result,
        ) != 0)
            .then_some(result)
    };
    let mut start = 0u32;
    let mut end = 0u32;
    read_message(
        0x00B0,
        (&mut start as *mut u32) as usize,
        (&mut end as *mut u32) as isize,
    )?;
    if start == end {
        return Some(String::new());
    }
    let length = read_message(wm::WM_GETTEXTLENGTH, 0, 0)?;
    if length > 16_384 || start > end || end as usize > length {
        return None;
    }
    let mut buffer = vec![0u16; length + 1];
    let copied = read_message(wm::WM_GETTEXT, buffer.len(), buffer.as_mut_ptr() as isize)?;
    if copied != length {
        return None;
    }
    let text = String::from_utf16(&buffer[start as usize..end as usize]).ok()?;
    Some(text)
}

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
    let focus = super::uia_worker::focused_runtime_id(process_id)
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
    let class = String::from_utf16_lossy(&class_name[..len as usize]).to_ascii_lowercase();
    // WinForms는 기본 Edit/RichEdit의 윈도우 클래스를 이 이름으로 감싼다.
    // 동일한 선택 조회 메시지를 지원하므로 복사 키 없이 읽을 수 있다.
    matches!(class.as_str(), "edit" | "richedit20w" | "richedit50w")
        || class.starts_with("windowsforms10.edit.")
        || class.starts_with("windowsforms10.richedit20w.")
        || class.starts_with("windowsforms10.richedit50w.")
}

pub(super) unsafe fn uia_runtime_id(process_id: u32) -> Option<Vec<i32>> {
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
        let length = last.checked_sub(first)?.checked_add(1)?;
        if !(1..=64).contains(&length) {
            return None;
        }
        let mut id = Vec::with_capacity(length as usize);
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

#[cfg(test)]
mod selection_tests {
    use super::*;

    #[test]
    fn native_edit_selection_is_read_without_copy_keys_or_clipboard() {
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED,
        };
        use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};
        use windows_sys::Win32::UI::WindowsAndMessaging as wm;

        let (sender, receiver) = std::sync::mpsc::channel();
        let host = std::thread::spawn(move || unsafe {
            let class: Vec<u16> = "EDIT\0".encode_utf16().collect();
            let text: Vec<u16> = "hello world\0".encode_utf16().collect();
            let window = wm::CreateWindowExW(
                0,
                class.as_ptr(),
                text.as_ptr(),
                wm::WS_OVERLAPPEDWINDOW,
                0,
                0,
                200,
                100,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            sender.send(window as usize).unwrap();
            if window.is_null() {
                return;
            }
            let mut message = std::mem::zeroed();
            while wm::GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                if message.message == wm::WM_APP {
                    break;
                }
                wm::TranslateMessage(&message);
                wm::DispatchMessageW(&message);
            }
            wm::DestroyWindow(window);
        });
        let window = receiver.recv().unwrap();
        assert_ne!(window, 0, "native test Edit must be created");
        let result = std::thread::spawn(move || unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).unwrap();
            let element = automation
                .ElementFromHandle(windows::Win32::Foundation::HWND(window as *mut _))
                .unwrap();
            let hwnd = window as windows_sys::Win32::Foundation::HWND;
            wm::SendMessageW(hwnd, 0x00B1, 6, 11); // EM_SETSEL은 테스트 전용 창에만 보낸다.
            let selected = uia_element_selection(&automation, &element, std::process::id());
            wm::SendMessageW(hwnd, 0x00B1, 0, 0);
            let empty = uia_element_selection(&automation, &element, std::process::id());
            assert_eq!(native_edit_selection(hwnd, u32::MAX), None);
            let unicode: Vec<u16> = "안녕 세계\0".encode_utf16().collect();
            wm::SendMessageW(hwnd, wm::WM_SETTEXT, 0, unicode.as_ptr() as isize);
            wm::SendMessageW(hwnd, 0x00B1, 3, 5);
            assert_eq!(
                native_edit_selection(hwnd, std::process::id()).as_deref(),
                Some("세계")
            );
            let style = wm::GetWindowLongPtrW(hwnd, wm::GWL_STYLE);
            wm::SetWindowLongPtrW(hwnd, wm::GWL_STYLE, style | wm::ES_PASSWORD as isize);
            assert_eq!(native_edit_selection(hwnd, std::process::id()), None);
            wm::SetWindowLongPtrW(hwnd, wm::GWL_STYLE, style);
            drop(element);
            drop(automation);
            CoUninitialize();
            (selected, empty)
        })
        .join();
        unsafe {
            wm::PostMessageW(window as _, wm::WM_APP, 0, 0);
        }
        host.join().unwrap();
        let (selected, empty) = result.unwrap();
        assert_eq!(selected.as_deref(), Some("world"));
        assert_eq!(empty, Some(String::new()));
    }
}
