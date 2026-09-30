use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use std::ffi::{CStr, CString};
use tauri::Manager;

unsafe fn ns_string(value: &str) -> *mut AnyObject {
    let value = CString::new(value).expect("diagnostic string contains no NUL");
    let class = AnyClass::get("NSString").expect("NSString is available");
    msg_send![class, stringWithUTF8String: value.as_ptr()]
}

unsafe fn rust_string(value: *mut AnyObject) -> String {
    if value.is_null() {
        return String::new();
    }
    let bytes: *const std::ffi::c_char = msg_send![value, UTF8String];
    if bytes.is_null() {
        return String::new();
    }
    CStr::from_ptr(bytes).to_string_lossy().into_owned()
}

pub fn capture_capsule(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("capsule") else {
        return;
    };
    if let Err(error) = window.with_webview(|platform| unsafe {
        let native_window = platform.ns_window().cast::<AnyObject>();
        let native_webview = platform.inner().cast::<AnyObject>();
        let opaque: Bool = msg_send![native_window, isOpaque];
        let has_shadow: Bool = msg_send![native_window, hasShadow];
        let style_mask: usize = msg_send![native_window, styleMask];
        let focus_ring: usize = msg_send![native_webview, focusRingType];
        let background: *mut AnyObject = msg_send![native_window, backgroundColor];
        let alpha: f64 = msg_send![background, alphaComponent];
        let draws_background: *mut AnyObject =
            msg_send![native_webview, valueForKey: ns_string("drawsBackground")];
        let draws_background: Bool = msg_send![draws_background, boolValue];
        tracing::info!(
            opaque = opaque.as_bool(),
            has_shadow = has_shadow.as_bool(),
            style_mask,
            focus_ring,
            background_alpha = alpha,
            draws_background = draws_background.as_bool(),
            "macOS capsule native transparency"
        );

        let script = ns_string(
            "JSON.stringify([document.documentElement,document.body,document.getElementById('root'),document.querySelector('.jelly-capsule,.jelly-capsule-active,.jelly-capsule-error')].filter(Boolean).map(element=>({tag:element.tagName,classes:element.className,background:getComputedStyle(element).backgroundColor,image:getComputedStyle(element).backgroundImage,colorScheme:getComputedStyle(element).colorScheme,border:getComputedStyle(element).border,shadow:getComputedStyle(element).boxShadow,outline:getComputedStyle(element).outline})))",
        );
        let evaluated = block2::RcBlock::new(move |result: *mut AnyObject, error: *mut AnyObject| {
            if error.is_null() {
                tracing::info!(canvas = %rust_string(result), "macOS capsule canvas");
            } else {
                tracing::warn!("macOS capsule canvas inspection failed");
            }
        });
        let _: () = msg_send![native_webview, evaluateJavaScript: script completionHandler: &*evaluated];

        let snapshot = block2::RcBlock::new(move |image: *mut AnyObject, error: *mut AnyObject| {
            if image.is_null() || !error.is_null() {
                tracing::warn!("macOS capsule snapshot failed");
                return;
            }
            let data: *mut AnyObject = msg_send![image, TIFFRepresentation];
            let saved: Bool = msg_send![data, writeToFile: ns_string("/tmp/ultra-stt-capsule.tiff") atomically: Bool::YES];
            tracing::info!(saved = saved.as_bool(), "macOS capsule snapshot");
        });
        let _: () = msg_send![native_webview, takeSnapshotWithConfiguration: std::ptr::null::<AnyObject>() completionHandler: &*snapshot];
    }) {
        tracing::warn!("macOS capsule inspection could not be scheduled: {error}");
    }
}
