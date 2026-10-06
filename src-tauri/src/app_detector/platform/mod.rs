use std::sync::Arc;

use super::types::ContextSignals;
use super::types::TargetAppGuard;
use super::types::{BrowserAccessStatus, BrowserTarget};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(crate) mod uia_worker;

pub(crate) trait ContextSignalSource: Send + Sync + 'static {
    fn collect(&self) -> Option<ContextSignals>;

    fn requires_focus_guard(&self) -> bool {
        false
    }

    fn current_target_guard(&self) -> Option<TargetAppGuard> {
        None
    }
}

pub(crate) fn default_source() -> Arc<dyn ContextSignalSource> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::MacOsContextSource)
    }
    #[cfg(target_os = "windows")]
    {
        uia_worker::warm_up();
        Arc::new(windows::WindowsContextSource)
    }
    #[cfg(target_os = "linux")]
    {
        Arc::new(linux::LinuxContextSource)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Arc::new(UnsupportedContextSource)
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn capture_selected_text() -> Option<String> {
    windows::capture_selected_text()
}

pub(crate) fn restore_target_application(target: &TargetAppGuard) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::restore_target_application(target)
    }
    #[cfg(target_os = "windows")]
    {
        windows::restore_target_application(target)
    }
    #[cfg(target_os = "linux")]
    {
        linux::restore_target_application(target)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = target;
        false
    }
}

pub(crate) fn request_browser_access(target: BrowserTarget) -> BrowserAccessStatus {
    #[cfg(target_os = "macos")]
    {
        macos::request_browser_access(target)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = target;
        BrowserAccessStatus::NotApplicable
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
struct UnsupportedContextSource;

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
impl ContextSignalSource for UnsupportedContextSource {
    fn collect(&self) -> Option<ContextSignals> {
        None
    }
}
