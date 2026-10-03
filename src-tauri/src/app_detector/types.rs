use std::time::Instant;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextFamily {
    Email,
    WorkChat,
    PersonalChat,
    Document,
    ProjectManagement,
    DeveloperCollaboration,
    PromptOrCode,
    Support,
    Social,
    General,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSource {
    UserMapping,
    BrowserDomain,
    NativeProcess,
    WindowTitle,
    Fallback,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextProfile {
    pub id: String,
    pub family: ContextFamily,
    pub app_label: String,
    pub icon_key: String,
    pub override_id: Option<String>,
    pub source: ContextSource,
    pub confidence: f32,
}

impl ContextProfile {
    pub fn general_browser() -> Self {
        Self::general("general.browser", "Browser")
    }

    pub fn general_native() -> Self {
        Self::general("general.native", "General")
    }

    fn general(id: &str, label: &str) -> Self {
        Self {
            id: id.to_string(),
            family: ContextFamily::General,
            app_label: label.to_string(),
            icon_key: "general".to_string(),
            override_id: None,
            source: ContextSource::Fallback,
            confidence: 0.0,
        }
    }

    pub fn summary(&self) -> ContextProfileSummary {
        ContextProfileSummary {
            profile_id: self.id.clone(),
            family: self.family,
            app_label: self.app_label.clone(),
            icon_key: self.icon_key.clone(),
            override_id: self.override_id.clone(),
            browser_access_status: BrowserAccessStatus::NotApplicable,
            browser_target: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ContextSnapshot {
    pub profile: ContextProfile,
    pub captured_at: Instant,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextProfileSummary {
    pub profile_id: String,
    pub family: ContextFamily,
    pub app_label: String,
    pub icon_key: String,
    pub override_id: Option<String>,
    pub browser_access_status: BrowserAccessStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_target: Option<BrowserTarget>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ContextSignals {
    pub(crate) process_id: Option<u32>,
    pub(crate) native_identity: Option<String>,
    pub(crate) target_window: Option<u64>,
    pub(crate) target_focus: Option<FocusIdentity>,
    pub(crate) focus_required: bool,
    pub(crate) process_alias: Option<String>,
    pub(crate) window_title: Option<String>,
    pub(crate) browser_host: Option<String>,
    pub(crate) is_supported_browser: bool,
    pub(crate) browser_access_status: BrowserAccessStatus,
    pub(crate) browser_target: Option<BrowserTarget>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FocusIdentity {
    WindowsUiAutomation(Vec<i32>),
    WindowsControl(u64),
    MacAccessibility { window: u64, element: u64 },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TargetAppGuard {
    pub process_id: Option<u32>,
    pub native_identity: Option<String>,
    pub target_window: Option<u64>,
    pub target_focus: Option<FocusIdentity>,
    pub focus_required: bool,
}

impl TargetAppGuard {
    pub fn is_empty(&self) -> bool {
        self.process_id.is_none() && self.native_identity.is_none() && !self.focus_required
    }

    pub fn unavailable() -> Self {
        Self {
            focus_required: true,
            ..Self::default()
        }
    }

    pub fn matches(&self, current: &Self) -> bool {
        if self.is_empty() {
            return true;
        }
        if let Some(expected_pid) = self.process_id {
            if current.process_id != Some(expected_pid) {
                return false;
            }
        }
        if let Some(expected_identity) = self.native_identity.as_deref() {
            if !current
                .native_identity
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case(expected_identity))
            {
                return false;
            }
        }
        if self.focus_required {
            if self.target_window.is_none() || self.target_focus.is_none() {
                return false;
            }
            if self.target_window != current.target_window
                || self.target_focus != current.target_focus
            {
                return false;
            }
        }
        true
    }
}

impl From<&ContextSignals> for TargetAppGuard {
    fn from(signals: &ContextSignals) -> Self {
        Self {
            process_id: signals.process_id,
            native_identity: signals.native_identity.clone(),
            target_window: signals.target_window,
            target_focus: signals.target_focus.clone(),
            focus_required: signals.focus_required,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RecordingContext {
    pub profile: ContextProfile,
    pub target_guard: TargetAppGuard,
    pub mapped_scene_id: Option<String>,
    pub browser_access_status: BrowserAccessStatus,
    pub browser_target: Option<BrowserTarget>,
}

impl RecordingContext {
    pub fn summary(&self) -> ContextProfileSummary {
        let mut summary = self.profile.summary();
        summary.browser_access_status = self.browser_access_status;
        summary.browser_target = self.browser_target;
        summary
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserTarget {
    Safari,
    Chrome,
    Edge,
    Brave,
    Arc,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAccessStatus {
    Available,
    NeedsPermission,
    #[default]
    NotApplicable,
    Unknown,
}

impl BrowserAccessStatus {
    #[cfg(any(target_os = "windows", target_os = "linux", test))]
    pub(crate) fn for_unavailable_url_adapter(is_supported_browser: bool) -> Self {
        if is_supported_browser {
            Self::Unknown
        } else {
            Self::NotApplicable
        }
    }

    pub fn as_history_value(self) -> Option<&'static str> {
        match self {
            Self::Available => Some("available"),
            Self::NeedsPermission => Some("needs_permission"),
            Self::Unknown => Some("unknown"),
            Self::NotApplicable => None,
        }
    }

    pub fn from_history_value(value: Option<&str>) -> Self {
        match value {
            Some("available") => Self::Available,
            Some("needs_permission") => Self::NeedsPermission,
            Some("unknown") => Self::Unknown,
            _ => Self::NotApplicable,
        }
    }
}

#[cfg(test)]
mod browser_access_status_tests {
    use super::*;

    #[test]
    fn unavailable_url_adapters_report_supported_browsers_as_unknown() {
        assert_eq!(
            BrowserAccessStatus::for_unavailable_url_adapter(true),
            BrowserAccessStatus::Unknown
        );
        assert_eq!(
            BrowserAccessStatus::for_unavailable_url_adapter(false),
            BrowserAccessStatus::NotApplicable
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Message,
    Email,
    Prose,
    TaskUpdate,
    DeveloperNote,
    Prompt,
    SupportReply,
    SocialPost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Formality {
    Casual,
    Neutral,
    Professional,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    Compact,
    Balanced,
    Expanded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkupPolicy {
    PlainText,
    Light,
    Structured,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListBehavior {
    Preserve,
    LineBreaks,
    NumberWhenExplicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStyleOverride {
    pub artifact_kind: ArtifactKind,
    pub formality: Formality,
    pub density: Density,
    pub markup: MarkupPolicy,
    pub list_behavior: ListBehavior,
}

pub fn is_valid_profile_id(id: &str) -> bool {
    let mut parts = id.split('.');
    let Some(prefix) = parts.next() else {
        return false;
    };
    let Some(slug) = parts.next() else {
        return false;
    };
    if parts.next().is_some() || prefix.is_empty() || slug.is_empty() {
        return false;
    }

    [prefix, slug].into_iter().all(|part| {
        part.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guarded(window: u64, focus: FocusIdentity) -> TargetAppGuard {
        TargetAppGuard {
            process_id: Some(42),
            native_identity: Some("chrome.exe".into()),
            target_window: Some(window),
            target_focus: Some(focus),
            focus_required: true,
        }
    }

    #[test]
    fn insertion_target_requires_the_same_window_and_control() {
        let expected = guarded(10, FocusIdentity::WindowsUiAutomation(vec![1, 2, 3]));
        assert!(expected.matches(&expected));
        assert!(!expected.matches(&guarded(
            11,
            FocusIdentity::WindowsUiAutomation(vec![1, 2, 3])
        )));
        assert!(!expected.matches(&guarded(
            10,
            FocusIdentity::WindowsUiAutomation(vec![1, 2, 4])
        )));
        assert!(!expected.matches(&guarded(10, FocusIdentity::WindowsControl(99))));
        assert!(!expected.matches(&TargetAppGuard {
            target_focus: None,
            ..expected.clone()
        }));
        assert!(!expected.matches(&TargetAppGuard::unavailable()));
    }

    #[test]
    fn restoring_the_original_window_requires_the_original_focus_too() {
        let expected = guarded(10, FocusIdentity::WindowsUiAutomation(vec![7, 8]));
        let other_window = guarded(11, FocusIdentity::WindowsUiAutomation(vec![7, 8]));
        let wrong_field = guarded(10, FocusIdentity::WindowsUiAutomation(vec![7, 9]));
        assert!(!expected.matches(&other_window));
        assert!(!expected.matches(&wrong_field));
        assert!(expected.matches(&expected));
    }

    #[test]
    fn target_unavailable_at_capture_never_authorizes_insertion() {
        let unavailable = TargetAppGuard::unavailable();
        assert!(!unavailable.is_empty());
        assert!(!unavailable.matches(&unavailable));
        assert!(!unavailable.matches(&guarded(10, FocusIdentity::WindowsUiAutomation(vec![1]))));
    }

    #[test]
    fn context_types_profile_ids_are_lowercase_and_bounded() {
        assert!(is_valid_profile_id("dev.github"));
        assert!(is_valid_profile_id("general.native"));
        assert!(!is_valid_profile_id("Dev.GitHub"));
        assert!(!is_valid_profile_id("github"));
        assert!(!is_valid_profile_id("dev.github.com"));
    }

    #[test]
    fn context_types_summary_contains_only_reviewed_metadata() {
        let profile = ContextProfile {
            id: "email.gmail".to_string(),
            family: ContextFamily::Email,
            app_label: "Gmail".to_string(),
            icon_key: "gmail".to_string(),
            override_id: Some("gmail".to_string()),
            source: ContextSource::BrowserDomain,
            confidence: 1.0,
        };

        let json = serde_json::to_string(&profile.summary()).unwrap();
        assert!(json.contains("email.gmail"));
        for forbidden in ["process", "window", "host", "url", "confidence"] {
            assert!(!json.contains(forbidden));
        }
    }
}
