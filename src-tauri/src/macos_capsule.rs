use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use tauri::Manager;

pub fn configure(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("capsule") else {
        return;
    };
    if let Err(error) = window.with_webview(|platform| unsafe {
        let native_window = platform.ns_window().cast::<AnyObject>();
        let native_webview = platform.inner().cast::<AnyObject>();
        let style_mask: usize = msg_send![native_window, styleMask];
        let has_shadow: Bool = msg_send![native_window, hasShadow];
        tracing::debug!(
            style_mask,
            has_shadow = has_shadow.as_bool(),
            "Capsule window before border removal"
        );

        let _: () = msg_send![native_window, setStyleMask: 0usize];
        let _: () = msg_send![native_window, setHasShadow: Bool::NO];
        let _: () = msg_send![native_window, setOpaque: Bool::NO];
        if let Some(color_class) = AnyClass::get("NSColor") {
            let clear: *mut AnyObject = msg_send![color_class, clearColor];
            let _: () = msg_send![native_window, setBackgroundColor: clear];
        }

        let content: *mut AnyObject = msg_send![native_window, contentView];
        for view in [content, native_webview] {
            if view.is_null() {
                continue;
            }
            let _: () = msg_send![view, setFocusRingType: 1usize];
            let layer: *mut AnyObject = msg_send![view, layer];
            if !layer.is_null() {
                let _: () = msg_send![layer, setBorderWidth: 0.0f64];
                let _: () = msg_send![layer, setShadowOpacity: 0.0f32];
            }
        }
        let _: () = msg_send![native_window, invalidateShadow];
    }) {
        tracing::warn!("Could not configure the borderless capsule: {error}");
    }
}
