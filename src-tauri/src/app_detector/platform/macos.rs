use std::collections::VecDeque;
use std::ffi::{c_void, CStr};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use objc2::msg_send;
use objc2::rc::autoreleasepool;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use url::Url;

use super::ContextSignalSource;
use crate::app_detector::types::{
    BrowserAccessStatus, BrowserTarget, ContextSignals, FocusIdentity, TargetAppGuard,
};

pub struct MacOsContextSource;

#[derive(Clone, Copy)]
struct SupportedBrowser {
    script_name: &'static str,
    bundle_id: &'static str,
    target: BrowserTarget,
}

pub(crate) fn restore_target_application(target: &TargetAppGuard) -> bool {
    let Some(window_id) = target.target_window else {
        return false;
    };
    let Some(process_id) = target.process_id else {
        return false;
    };
    let Ok(process_id) = i32::try_from(process_id) else {
        return false;
    };
    let Some(class) = AnyClass::get("NSRunningApplication") else {
        return false;
    };
    autoreleasepool(|_| unsafe {
        let application: *mut AnyObject =
            msg_send![class, runningApplicationWithProcessIdentifier: process_id];
        if application.is_null() {
            return false;
        }
        if let Some(expected_identity) = target.native_identity.as_deref() {
            let bundle: *mut AnyObject = msg_send![application, bundleIdentifier];
            if ns_string(bundle).as_deref() != Some(expected_identity) {
                return false;
            }
        }
        let activated: Bool = msg_send![application, activateWithOptions: 1usize << 1];
        activated.as_bool() && raise_window(process_id, window_id)
    })
}

impl ContextSignalSource for MacOsContextSource {
    fn requires_focus_guard(&self) -> bool {
        true
    }

    fn collect(&self) -> Option<ContextSignals> {
        let (app_name, process_id, native_identity) = frontmost_app_identity()?;
        let window_title = focused_window_title(process_id);

        let browser = supported_browser(native_identity.as_deref(), &app_name);
        let (browser_host, browser_access_status) = match browser {
            Some(browser) => {
                let permission_status = browser_access_status_for_automation_permission(
                    automation_permission_status(browser.bundle_id, false),
                );
                if permission_status != BrowserAccessStatus::Available {
                    (None, permission_status)
                } else {
                    match read_browser_host(browser.script_name) {
                        Some(host) => (Some(host), BrowserAccessStatus::Available),
                        None => (None, BrowserAccessStatus::Unknown),
                    }
                }
            }
            None => (None, BrowserAccessStatus::NotApplicable),
        };

        Some(ContextSignals {
            process_id: Some(process_id),
            native_identity,
            target_window: None,
            target_focus: None,
            focus_required: true,
            process_alias: Some(app_name),
            window_title,
            browser_host,
            is_supported_browser: browser.is_some(),
            browser_access_status,
            browser_target: browser.map(|browser| browser.target),
        })
    }

    fn current_target_guard(&self) -> Option<TargetAppGuard> {
        let (_, process_id, native_identity) = frontmost_app_identity()?;
        let (target_window, target_focus) = focused_ax_identity(process_id)
            .map(|(window, element)| {
                (
                    Some(window),
                    Some(FocusIdentity::MacAccessibility { window, element }),
                )
            })
            .unwrap_or((None, None));
        if frontmost_app_identity().is_none_or(|(_, current_pid, current_identity)| {
            current_pid != process_id || current_identity != native_identity
        }) {
            return Some(TargetAppGuard::unavailable());
        }
        Some(TargetAppGuard {
            process_id: Some(process_id),
            native_identity,
            target_window,
            target_focus,
            focus_required: true,
        })
    }
}

fn focused_ax_identity(process_id: u32) -> Option<(u64, u64)> {
    if !crate::pipeline::is_accessibility_trusted() {
        return None;
    }
    unsafe {
        let application = AXUIElementCreateApplication(process_id as i32);
        if application.is_null() {
            return None;
        }
        let _ = AXUIElementSetMessagingTimeout(application, 0.15);
        let result = (|| {
            let window = copy_ax_attribute(application, b"AXFocusedWindow\0")?;
            let Some(element) = copy_ax_attribute(application, b"AXFocusedUIElement\0") else {
                CFRelease(window);
                return None;
            };
            let identity = if CFGetTypeID(window) == AXUIElementGetTypeID()
                && CFGetTypeID(element) == AXUIElementGetTypeID()
            {
                let mut window_pid = 0i32;
                let mut element_pid = 0i32;
                if AXUIElementGetPid(window, &mut window_pid) == 0
                    && AXUIElementGetPid(element, &mut element_pid) == 0
                    && window_pid == process_id as i32
                    && element_pid == process_id as i32
                {
                    Some((intern_ax_element(window), intern_ax_element(element)))
                } else {
                    None
                }
            } else {
                None
            };
            CFRelease(window);
            CFRelease(element);
            identity
        })();
        CFRelease(application);
        result
    }
}

// AXUIElementRef values are retained until evicted. Only the opaque tokens cross
// the target guard boundary; equality is checked with CFEqual, not CFHash.
struct RetainedAxElement {
    token: u64,
    ptr: usize,
}

impl Drop for RetainedAxElement {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.ptr as *const c_void);
        }
    }
}

fn ax_registry() -> &'static Mutex<VecDeque<RetainedAxElement>> {
    static REGISTRY: OnceLock<Mutex<VecDeque<RetainedAxElement>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(VecDeque::new()))
}

unsafe fn intern_ax_element(element: *const c_void) -> u64 {
    static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
    let mut refs = ax_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(existing) = refs
        .iter()
        .find(|entry| CFEqual(entry.ptr as *const c_void, element) != 0)
    {
        return existing.token;
    }
    let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    CFRetain(element);
    refs.push_back(RetainedAxElement {
        token,
        ptr: element as usize,
    });
    if refs.len() > 128 {
        refs.pop_front();
    }
    token
}

unsafe fn ax_token_matches(token: u64, element: *const c_void) -> bool {
    let refs = ax_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    refs.iter()
        .find(|entry| entry.token == token)
        .is_some_and(|entry| CFEqual(entry.ptr as *const c_void, element) != 0)
}

unsafe fn raise_window(process_id: i32, expected_token: u64) -> bool {
    let application = AXUIElementCreateApplication(process_id);
    if application.is_null() {
        return false;
    }
    let _ = AXUIElementSetMessagingTimeout(application, 0.15);
    let windows = copy_ax_attribute(application, b"AXWindows\0");
    CFRelease(application);
    let Some(windows) = windows else {
        return false;
    };
    let mut raised = false;
    if CFGetTypeID(windows) == CFArrayGetTypeID() {
        let count = CFArrayGetCount(windows).min(64);
        for index in 0..count {
            let window = CFArrayGetValueAtIndex(windows, index);
            if !window.is_null()
                && CFGetTypeID(window) == AXUIElementGetTypeID()
                && ax_token_matches(expected_token, window)
            {
                let action =
                    CFStringCreateWithCString(std::ptr::null(), c"AXRaise".as_ptr(), 0x08000100);
                if !action.is_null() {
                    raised = AXUIElementPerformAction(window, action) == 0;
                    CFRelease(action);
                }
                break;
            }
        }
    }
    CFRelease(windows);
    raised
}

fn frontmost_app_identity() -> Option<(String, u32, Option<String>)> {
    let class = AnyClass::get("NSWorkspace")?;
    autoreleasepool(|_| unsafe {
        let workspace: *mut AnyObject = msg_send![class, sharedWorkspace];
        if workspace.is_null() {
            return None;
        }
        let application: *mut AnyObject = msg_send![workspace, frontmostApplication];
        if application.is_null() {
            return None;
        }
        let process_id: i32 = msg_send![application, processIdentifier];
        let process_id = u32::try_from(process_id).ok().filter(|id| *id != 0)?;
        let bundle: *mut AnyObject = msg_send![application, bundleIdentifier];
        let native_identity = ns_string(bundle);
        let name: *mut AnyObject = msg_send![application, localizedName];
        let app_name = ns_string(name).or_else(|| native_identity.clone())?;
        Some((app_name, process_id, native_identity))
    })
}

unsafe fn ns_string(value: *mut AnyObject) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let bytes: *const std::ffi::c_char = msg_send![value, UTF8String];
    if bytes.is_null() {
        return None;
    }
    let text = CStr::from_ptr(bytes).to_string_lossy().into_owned();
    (!text.is_empty()).then_some(text)
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(process_id: i32) -> *const c_void;
    fn AXUIElementGetTypeID() -> usize;
    fn AXUIElementGetPid(element: *const c_void, process_id: *mut i32) -> i32;
    fn AXUIElementSetMessagingTimeout(element: *const c_void, timeout: f32) -> i32;
    fn AXUIElementPerformAction(element: *const c_void, action: *const c_void) -> i32;
    fn AXUIElementCopyAttributeValue(
        element: *const c_void,
        attribute: *const c_void,
        value: *mut *const c_void,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRetain(value: *const c_void) -> *const c_void;
    fn CFEqual(left: *const c_void, right: *const c_void) -> u8;
    fn CFArrayGetTypeID() -> usize;
    fn CFArrayGetCount(array: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
    fn CFStringCreateWithCString(
        allocator: *const c_void,
        text: *const i8,
        encoding: u32,
    ) -> *const c_void;
    fn CFRelease(value: *const c_void);
    fn CFGetTypeID(value: *const c_void) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFStringGetLength(value: *const c_void) -> isize;
    fn CFStringGetCString(value: *const c_void, buffer: *mut i8, size: isize, encoding: u32) -> u8;
}

unsafe fn copy_ax_attribute(element: *const c_void, name: &[u8]) -> Option<*const c_void> {
    let attribute = CFStringCreateWithCString(std::ptr::null(), name.as_ptr().cast(), 0x08000100);
    if attribute.is_null() {
        return None;
    }
    let mut value = std::ptr::null();
    let status = AXUIElementCopyAttributeValue(element, attribute, &mut value);
    CFRelease(attribute);
    if status != 0 || value.is_null() {
        if !value.is_null() {
            CFRelease(value);
        }
        return None;
    }
    Some(value)
}

fn focused_window_title(process_id: u32) -> Option<String> {
    if !crate::pipeline::is_accessibility_trusted() {
        return None;
    }
    unsafe {
        let application = AXUIElementCreateApplication(process_id as i32);
        if application.is_null() {
            return None;
        }
        let window = copy_ax_attribute(application, b"AXFocusedWindow\0");
        CFRelease(application);
        let window = window?;
        let title = copy_ax_attribute(window, b"AXTitle\0");
        CFRelease(window);
        let title = title?;
        let result = if CFGetTypeID(title) == CFStringGetTypeID() {
            let length = CFStringGetLength(title);
            let mut buffer = vec![0i8; (length as usize).saturating_mul(4).saturating_add(1)];
            let success = CFStringGetCString(
                title,
                buffer.as_mut_ptr(),
                buffer.len() as isize,
                0x08000100,
            );
            if success != 0 {
                CStr::from_ptr(buffer.as_ptr())
                    .to_str()
                    .ok()
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            } else {
                None
            }
        } else {
            None
        };
        CFRelease(title);
        result
    }
}

fn supported_browser(identity: Option<&str>, app_name: &str) -> Option<SupportedBrowser> {
    let identity = identity.unwrap_or_default();
    match identity {
        "com.apple.Safari" => Some(SupportedBrowser {
            script_name: "Safari",
            bundle_id: "com.apple.Safari",
            target: BrowserTarget::Safari,
        }),
        "com.google.Chrome" => Some(SupportedBrowser {
            script_name: "Google Chrome",
            bundle_id: "com.google.Chrome",
            target: BrowserTarget::Chrome,
        }),
        "com.microsoft.edgemac" => Some(SupportedBrowser {
            script_name: "Microsoft Edge",
            bundle_id: "com.microsoft.edgemac",
            target: BrowserTarget::Edge,
        }),
        "com.brave.Browser" => Some(SupportedBrowser {
            script_name: "Brave Browser",
            bundle_id: "com.brave.Browser",
            target: BrowserTarget::Brave,
        }),
        "company.thebrowser.Browser" => Some(SupportedBrowser {
            script_name: "Arc",
            bundle_id: "company.thebrowser.Browser",
            target: BrowserTarget::Arc,
        }),
        _ => match app_name {
            "Safari" => supported_browser(Some("com.apple.Safari"), ""),
            "Google Chrome" => supported_browser(Some("com.google.Chrome"), ""),
            "Microsoft Edge" => supported_browser(Some("com.microsoft.edgemac"), ""),
            "Brave Browser" => supported_browser(Some("com.brave.Browser"), ""),
            "Arc" => supported_browser(Some("company.thebrowser.Browser"), ""),
            _ => None,
        },
    }
}

#[repr(C)]
struct AeDesc {
    descriptor_type: u32,
    data_handle: *mut c_void,
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AECreateDesc(
        descriptor_type: u32,
        data: *const c_void,
        data_size: isize,
        result: *mut AeDesc,
    ) -> i16;
    fn AEDisposeDesc(descriptor: *mut AeDesc) -> i16;
    fn AEDeterminePermissionToAutomateTarget(
        target: *const AeDesc,
        event_class: u32,
        event_id: u32,
        ask_user_if_needed: u8,
    ) -> i32;
}

fn automation_permission_status(bundle_id: &str, ask_user_if_needed: bool) -> i32 {
    const TYPE_APPLICATION_BUNDLE_ID: u32 = u32::from_be_bytes(*b"bund");
    const CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"core");
    const GET_DATA_EVENT_ID: u32 = u32::from_be_bytes(*b"getd");

    let mut target = AeDesc {
        descriptor_type: 0,
        data_handle: std::ptr::null_mut(),
    };
    let create_status = unsafe {
        AECreateDesc(
            TYPE_APPLICATION_BUNDLE_ID,
            bundle_id.as_ptr().cast(),
            bundle_id.len() as isize,
            &mut target,
        )
    };
    if create_status != 0 {
        return i32::from(create_status);
    }

    let permission_status = unsafe {
        AEDeterminePermissionToAutomateTarget(
            &target,
            CORE_EVENT_CLASS,
            GET_DATA_EVENT_ID,
            u8::from(ask_user_if_needed),
        )
    };
    unsafe {
        let _ = AEDisposeDesc(&mut target);
    }
    permission_status
}

pub(crate) fn request_browser_access(target: BrowserTarget) -> BrowserAccessStatus {
    let bundle_id = match target {
        BrowserTarget::Safari => "com.apple.Safari",
        BrowserTarget::Chrome => "com.google.Chrome",
        BrowserTarget::Edge => "com.microsoft.edgemac",
        BrowserTarget::Brave => "com.brave.Browser",
        BrowserTarget::Arc => "company.thebrowser.Browser",
    };
    browser_access_status_for_automation_permission(automation_permission_status(bundle_id, true))
}

fn browser_access_status_for_automation_permission(status: i32) -> BrowserAccessStatus {
    match status {
        0 => BrowserAccessStatus::Available,
        -1743 | -1744 => BrowserAccessStatus::NeedsPermission,
        _ => BrowserAccessStatus::Unknown,
    }
}

fn read_browser_host(browser_name: &str) -> Option<String> {
    let script = if browser_name == "Safari" {
        r#"tell application "Safari" to get URL of current tab of front window"#.to_string()
    } else {
        format!("tell application \"{browser_name}\" to get URL of active tab of front window")
    };
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let transient_url = String::from_utf8(output.stdout).ok()?;
    Url::parse(transient_url.trim())
        .ok()?
        .host_str()
        .map(|host| host.trim_end_matches('.').to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_permission_status_only_requests_action_for_permission_errors() {
        assert_eq!(
            browser_access_status_for_automation_permission(0),
            BrowserAccessStatus::Available
        );
        assert_eq!(
            browser_access_status_for_automation_permission(-1743),
            BrowserAccessStatus::NeedsPermission
        );
        assert_eq!(
            browser_access_status_for_automation_permission(-1744),
            BrowserAccessStatus::NeedsPermission
        );
        assert_eq!(
            browser_access_status_for_automation_permission(-600),
            BrowserAccessStatus::Unknown
        );
        assert_eq!(
            browser_access_status_for_automation_permission(-1),
            BrowserAccessStatus::Unknown
        );
    }
}
