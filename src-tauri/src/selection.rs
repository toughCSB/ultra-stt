#[cfg(not(target_os = "macos"))]
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};

const CLIPBOARD_COPY_SETTLE_MS: u64 = 100;

pub fn selected_text_from_clipboard_result(
    selected: Option<String>,
    sentinel: &str,
) -> Option<String> {
    match selected {
        Some(text) if !text.trim().is_empty() && text != sentinel => Some(text),
        _ => None,
    }
}

fn clipboard_copy_sentinel() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!(
        "__opentypeless_copy_sentinel_{}_{}__",
        std::process::id(),
        nanos
    )
}

#[cfg(target_os = "macos")]
fn copy_selected_text_to_clipboard() -> bool {
    match std::process::Command::new("/usr/bin/osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "c" using command down"#,
        ])
        .status()
    {
        Ok(status) if status.success() => true,
        Ok(status) => {
            tracing::warn!(
                "macOS selected-text copy failed with exit code: {:?}",
                status.code()
            );
            false
        }
        Err(e) => {
            tracing::warn!("Failed to run osascript for selected-text copy: {}", e);
            false
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn copy_selected_text_to_clipboard() -> bool {
    let Ok(mut enigo) = Enigo::new(&EnigoSettings::default()) else {
        return false;
    };

    let pressed = enigo.key(Key::Control, Direction::Press).is_ok();
    if pressed {
        // Windows의 표준 대체 복사 키를 사용한다. 선택이 없는 터미널에도
        // Ctrl+C 중단 입력을 보내지 않으면서 클립보드 기반 조회를 유지한다.
        #[cfg(target_os = "windows")]
        let copy_key = Key::Insert;
        #[cfg(not(target_os = "windows"))]
        let copy_key = Key::Unicode('c');
        let copied = enigo.key(copy_key, Direction::Click).is_ok();
        let released = enigo.key(Key::Control, Direction::Release).is_ok();
        return copied && released;
    }
    pressed
}

#[cfg(target_os = "windows")]
pub fn capture_selected_text() -> Option<String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return None;
    }
    if let Some(text) = crate::app_detector::platform::capture_selected_text() {
        // Some("")는 선택 없음 또는 조회 금지로 확인된 결과이다.
        // None(UIA 미지원/조회 실패)과 구분하여 불필요한 복사 키를 보내지 않는다.
        return (unsafe { GetForegroundWindow() } == window)
            .then_some(text)
            .filter(|text| !text.trim().is_empty());
    }
    // UIA 미지원 앱의 선택 조회도 유지하되, 조회 중 바뀐 창에는 키를 보내지 않는다.
    if unsafe { GetForegroundWindow() } != window {
        return None;
    }
    let text = capture_selected_text_from_clipboard();
    (unsafe { GetForegroundWindow() } == window)
        .then_some(text)
        .flatten()
}

#[cfg(not(target_os = "windows"))]
pub fn capture_selected_text() -> Option<String> {
    capture_selected_text_from_clipboard()
}

fn capture_selected_text_from_clipboard() -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let backup = clipboard.get_text().ok();
    let sentinel = clipboard_copy_sentinel();
    let _ = clipboard.set_text(&sentinel);

    if !copy_selected_text_to_clipboard() {
        tracing::debug!("Selected text copy shortcut could not be sent");
    }

    std::thread::sleep(std::time::Duration::from_millis(CLIPBOARD_COPY_SETTLE_MS));

    let selected = clipboard.get_text().ok();

    if let Some(ref b) = backup {
        let _ = clipboard.set_text(b);
    } else {
        let _ = clipboard.set_text("");
    }

    tracing::info!(
        "Selected text capture: backup_len={}, selected_len={}",
        backup.as_deref().map(|s| s.len()).unwrap_or(0),
        selected.as_deref().map(|s| s.len()).unwrap_or(0)
    );

    let result = selected_text_from_clipboard_result(selected, &sentinel);
    if result.is_none() {
        tracing::debug!("Selected text capture did not produce fresh clipboard text");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_text_rejects_copy_sentinel_when_clipboard_was_unchanged() {
        assert_eq!(
            selected_text_from_clipboard_result(Some("__sentinel__".to_string()), "__sentinel__"),
            None
        );
    }

    #[test]
    fn selected_text_accepts_text_that_matches_previous_clipboard_backup() {
        assert_eq!(
            selected_text_from_clipboard_result(Some("selected text".to_string()), "__sentinel__"),
            Some("selected text".to_string())
        );
    }

    #[test]
    fn selected_text_rejects_whitespace_only_clipboard() {
        assert_eq!(
            selected_text_from_clipboard_result(Some(" \n\t ".to_string()), "__sentinel__"),
            None
        );
    }
}
