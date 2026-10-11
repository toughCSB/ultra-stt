//! Disposable, same-identity AX helper; unsupported selection keeps Command+C.
use crate::local_process::{Limits, LocalProcess};
use objc2::{
    msg_send,
    rc::autoreleasepool,
    runtime::{AnyClass, AnyObject, Bool},
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::{c_void, CStr, CString},
    io::{self, BufRead, Read, Write},
    time::Duration,
};
const FLAG: &str = "--ultra-stt-mac-selection";
const MAX_TEXT: usize = 65536;
const MAX_LINE: usize = MAX_TEXT * 6 + 128;
const DEADLINE: Duration = Duration::from_millis(900);
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "kind", content = "text")]
enum Reply {
    Selected(String),
    Empty,
    Blocked,
    Fallback,
    Copied,
}
fn parse_reply(bytes: &[u8]) -> Option<Reply> {
    if bytes.len() > MAX_LINE {
        return None;
    }
    let reply: Reply = serde_json::from_slice(bytes).ok()?;
    if matches!(&reply, Reply::Selected(text) if text.len() > MAX_TEXT) {
        return None;
    }
    Some(reply)
}
pub(crate) fn capture() -> Option<String> {
    let runtime = tokio::runtime::Handle::try_current().ok()?;
    runtime.block_on(async {
        let limits = Limits {
            stdin: 5,
            stdout: MAX_LINE,
            stderr: 1024,
            deadline: DEADLINE,
        };
        let pid = front_pid()?;
        let mut child = LocalProcess::spawn(std::env::current_exe().ok()?, limits, |cmd| {
            cmd.args([FLAG, &pid.to_string()]);
        })
        .ok()?;
        let mut backup = None;
        let result = tokio::time::timeout(DEADLINE, async {
            let first = parse_reply(&child.ready_line(MAX_LINE, DEADLINE).await.ok()?)?;
            match first {
                Reply::Selected(text) if front_pid() == Some(pid) => Some(text),
                Reply::Fallback if front_pid() == Some(pid) => {
                    backup = Some(PasteboardBackup::new()?);
                    let sentinel = format!(
                        "__ultra_selection_{}_{}__",
                        std::process::id(),
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()?
                            .as_nanos()
                    );
                    let mut clipboard = arboard::Clipboard::new().ok()?;
                    backup.as_mut()?.armed = true;
                    clipboard.set_text(&sentinel).ok()?;
                    child.send_stdin(b"copy\n", 5, DEADLINE).await.ok()?;
                    let last = parse_reply(&child.ready_line(MAX_LINE, DEADLINE).await.ok()?)?;
                    if last != Reply::Copied || front_pid() != Some(pid) {
                        return None;
                    }
                    crate::selection::selected_text_from_clipboard_result(
                        clipboard.get_text().ok(),
                        &sentinel,
                    )
                    .filter(|text| text.len() <= MAX_TEXT)
                }
                _ => None,
            }
        })
        .await
        .ok()
        .flatten();
        // Kill/reap before restoring so a timed-out helper cannot copy afterwards.
        let _ = child.kill(Duration::from_millis(150)).await;
        drop(backup);
        result.filter(|text| !text.trim().is_empty())
    })
}
pub(crate) fn run_if_requested() -> bool {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some(FLAG) {
        return false;
    }
    let pid = if args.len() == 2 {
        args[1].parse::<i32>().ok().filter(|p| *p > 0)
    } else {
        None
    };
    let result = pid
        .map(|pid| autoreleasepool(|_| unsafe { query(pid) }))
        .unwrap_or(Reply::Blocked);
    send(&result);
    true
}
fn send(reply: &Reply) {
    let mut output = io::stdout().lock();
    let _ = serde_json::to_writer(&mut output, reply);
    let _ = output.write_all(b"\n");
    let _ = output.flush();
}
fn front_pid() -> Option<i32> {
    autoreleasepool(|_| unsafe {
        let class = AnyClass::get("NSWorkspace")?;
        let workspace: *mut AnyObject = msg_send![class, sharedWorkspace];
        let app: *mut AnyObject = msg_send![workspace, frontmostApplication];
        if app.is_null() {
            return None;
        }
        let pid: i32 = msg_send![app, processIdentifier];
        (pid > 0).then_some(pid)
    })
}
struct Ax(*const c_void);
impl Drop for Ax {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
}
unsafe fn attribute(element: *const c_void, name: &str) -> Result<Ax, i32> {
    let name = CString::new(name).map_err(|_| -1)?;
    let attr = Ax(CFStringCreateWithCString(
        std::ptr::null(),
        name.as_ptr(),
        0x08000100,
    ));
    if attr.0.is_null() {
        return Err(-1);
    }
    let mut value = std::ptr::null();
    let status = AXUIElementCopyAttributeValue(element, attr.0, &mut value);
    if status != 0 || value.is_null() {
        if !value.is_null() {
            CFRelease(value);
        }
        return Err(status);
    }
    Ok(Ax(value))
}
unsafe fn string(value: *const c_void) -> Option<String> {
    if CFGetTypeID(value) != CFStringGetTypeID() {
        return None;
    }
    let length = CFStringGetLength(value);
    if !(0..=16384).contains(&length) {
        return None;
    }
    let mut bytes = vec![0i8; length as usize * 4 + 1];
    if CFStringGetCString(value, bytes.as_mut_ptr(), bytes.len() as isize, 0x08000100) == 0 {
        return None;
    }
    CStr::from_ptr(bytes.as_ptr())
        .to_str()
        .ok()
        .map(str::to_owned)
}
unsafe fn focused(app: *const c_void, pid: i32) -> Option<Ax> {
    let element = attribute(app, "AXFocusedUIElement").ok()?;
    if CFGetTypeID(element.0) != AXUIElementGetTypeID() {
        return None;
    }
    AXUIElementSetMessagingTimeout(element.0, 0.08);
    let mut actual = 0;
    (AXUIElementGetPid(element.0, &mut actual) == 0 && actual == pid).then_some(element)
}
unsafe fn unchanged(app: *const c_void, element: *const c_void, pid: i32) -> bool {
    front_pid() == Some(pid) && focused(app, pid).is_some_and(|now| CFEqual(now.0, element) != 0)
}
unsafe fn query(pid: i32) -> Reply {
    if !crate::pipeline::is_accessibility_trusted() || front_pid() != Some(pid) {
        return Reply::Blocked;
    }
    let system = Ax(AXUIElementCreateSystemWide());
    AXUIElementSetMessagingTimeout(system.0, 0.08); // Only this helper's AX messaging.
    let app = Ax(AXUIElementCreateApplication(pid));
    AXUIElementSetMessagingTimeout(app.0, 0.08);
    let Some(element) = focused(app.0, pid) else {
        return Reply::Blocked;
    };
    let Ok(role) = attribute(element.0, "AXRole") else {
        return Reply::Blocked;
    };
    let Some(role) = string(role.0) else {
        return Reply::Blocked;
    };
    if role == "AXSecureTextField" {
        return Reply::Blocked;
    }
    match attribute(element.0, "AXSubrole") {
        Ok(subrole) if string(subrole.0).as_deref() == Some("AXSecureTextField") => {
            return Reply::Blocked
        }
        Err(code) if code != -25205 && code != -25212 => return Reply::Blocked,
        _ => {}
    }
    if let Ok(protected) = attribute(element.0, "AXProtectedContent") {
        if CFGetTypeID(protected.0) == CFBooleanGetTypeID() && CFBooleanGetValue(protected.0) != 0 {
            return Reply::Blocked;
        }
    }
    let result = match attribute(element.0, "AXSelectedText") {
        Ok(selected) => match string(selected.0) {
            Some(text) if text.is_empty() => Reply::Empty,
            Some(text) => Reply::Selected(text),
            None => Reply::Blocked,
        },
        Err(_) => {
            let empty = attribute(element.0, "AXSelectedTextRange")
                .ok()
                .is_some_and(|range| {
                    if CFGetTypeID(range.0) != AXValueGetTypeID() {
                        return false;
                    }
                    let mut value = Range {
                        location: 0,
                        length: 0,
                    };
                    AXValueGetValue(range.0, 4, (&mut value as *mut Range).cast()) != 0
                        && value.length == 0
                });
            if empty {
                Reply::Empty
            } else {
                Reply::Fallback
            }
        }
    };
    if !unchanged(app.0, element.0, pid) {
        return Reply::Blocked;
    }
    if result != Reply::Fallback {
        return result;
    }
    send(&Reply::Fallback);
    let mut command = String::new();
    if io::stdin().lock().take(5).read_line(&mut command).is_err()
        || command != "copy\n"
        || !unchanged(app.0, element.0, pid)
    {
        return Reply::Blocked;
    }
    // kVK_ANSI_C=8; Command=1<<20. Target PID explicitly; never Control=1<<18.
    let down = Ax(CGEventCreateKeyboardEvent(std::ptr::null(), 8, true));
    let up = Ax(CGEventCreateKeyboardEvent(std::ptr::null(), 8, false));
    if down.0.is_null() || up.0.is_null() {
        return Reply::Blocked;
    }
    CGEventSetFlags(down.0, 1 << 20);
    CGEventSetFlags(up.0, 1 << 20);
    CGEventPostToPid(pid, down.0);
    CGEventPostToPid(pid, up.0);
    std::thread::sleep(Duration::from_millis(100));
    if unchanged(app.0, element.0, pid) {
        Reply::Copied
    } else {
        Reply::Blocked
    }
}
#[repr(C)]
struct Range {
    location: isize,
    length: isize,
}
// Preserve all original pasteboard types, including images and RTF.
struct PasteboardBackup {
    items: Vec<Vec<(String, Vec<u8>)>>,
    armed: bool,
}
impl PasteboardBackup {
    fn new() -> Option<Self> {
        autoreleasepool(|_| unsafe {
            let board = pasteboard()?;
            let array: *mut AnyObject = msg_send![board, pasteboardItems];
            let count: usize = if array.is_null() {
                0
            } else {
                msg_send![array, count]
            };
            let mut items = Vec::new();
            let mut total = 0usize;
            for index in 0..count {
                let item: *mut AnyObject = msg_send![array, objectAtIndex: index];
                let types: *mut AnyObject = msg_send![item, types];
                let count: usize = msg_send![types, count];
                let mut values = Vec::new();
                for index in 0..count {
                    let kind: *mut AnyObject = msg_send![types, objectAtIndex: index];
                    let name: *const i8 = msg_send![kind, UTF8String];
                    let data: *mut AnyObject = msg_send![item, dataForType: kind];
                    if name.is_null() || data.is_null() {
                        return None;
                    }
                    let length: usize = msg_send![data, length];
                    total = total.checked_add(length)?;
                    if total > 32 * 1024 * 1024 {
                        return None;
                    }
                    let bytes: *const u8 = msg_send![data, bytes];
                    let bytes = if length == 0 {
                        Vec::new()
                    } else {
                        if bytes.is_null() {
                            return None;
                        }
                        std::slice::from_raw_parts(bytes, length).to_vec()
                    };
                    values.push((CStr::from_ptr(name).to_str().ok()?.to_owned(), bytes));
                }
                items.push(values);
            }
            Some(Self {
                items,
                armed: false,
            })
        })
    }
}
impl Drop for PasteboardBackup {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        autoreleasepool(|_| unsafe {
            let Some(board) = pasteboard() else {
                return;
            };
            let _: isize = msg_send![board, clearContents];
            let array: *mut AnyObject = msg_send![AnyClass::get("NSMutableArray").unwrap(), array];
            for values in &self.items {
                let item: *mut AnyObject =
                    msg_send![AnyClass::get("NSPasteboardItem").unwrap(), new];
                for (kind, bytes) in values {
                    let kind = CString::new(kind.as_str()).unwrap();
                    let ns_kind: *mut AnyObject = msg_send![AnyClass::get("NSString").unwrap(), stringWithUTF8String: kind.as_ptr()];
                    let data: *mut AnyObject = msg_send![AnyClass::get("NSData").unwrap(), dataWithBytes: bytes.as_ptr() length: bytes.len()];
                    let _: Bool = msg_send![item, setData: data forType: ns_kind];
                }
                let _: () = msg_send![array, addObject: item];
                let _: () = msg_send![item, release];
            }
            let _: Bool = msg_send![board, writeObjects: array];
        });
    }
}
unsafe fn pasteboard() -> Option<*mut AnyObject> {
    let board: *mut AnyObject = msg_send![AnyClass::get("NSPasteboard")?, generalPasteboard];
    (!board.is_null()).then_some(board)
}
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateSystemWide() -> *const c_void;
    fn AXUIElementCreateApplication(pid: i32) -> *const c_void;
    fn AXUIElementGetTypeID() -> usize;
    fn AXUIElementGetPid(element: *const c_void, pid: *mut i32) -> i32;
    fn AXUIElementSetMessagingTimeout(element: *const c_void, timeout: f32) -> i32;
    fn AXUIElementCopyAttributeValue(
        element: *const c_void,
        attr: *const c_void,
        value: *mut *const c_void,
    ) -> i32;
    fn AXValueGetTypeID() -> usize;
    fn AXValueGetValue(value: *const c_void, kind: u32, result: *mut c_void) -> u8;
    fn CGEventCreateKeyboardEvent(source: *const c_void, key: u16, down: bool) -> *const c_void;
    fn CGEventSetFlags(event: *const c_void, flags: u64);
    fn CGEventPostToPid(pid: i32, event: *const c_void);
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(value: *const c_void);
    fn CFEqual(left: *const c_void, right: *const c_void) -> u8;
    fn CFGetTypeID(value: *const c_void) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFStringCreateWithCString(
        allocator: *const c_void,
        value: *const i8,
        encoding: u32,
    ) -> *const c_void;
    fn CFStringGetLength(value: *const c_void) -> isize;
    fn CFStringGetCString(value: *const c_void, buffer: *mut i8, size: isize, encoding: u32) -> u8;
    fn CFBooleanGetTypeID() -> usize;
    fn CFBooleanGetValue(value: *const c_void) -> u8;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_rejects_malformed_and_oversized_text() {
        assert!(parse_reply(b"garbage").is_none());
        assert!(parse_reply(
            &serde_json::to_vec(&Reply::Selected("x".repeat(MAX_TEXT + 1))).unwrap()
        )
        .is_none());
        assert!(parse_reply(&vec![0; MAX_LINE + 1]).is_none());
    }
    #[test]
    fn protocol_preserves_unicode_empty_and_blocked() {
        for reply in [
            Reply::Selected("선택 🌍".into()),
            Reply::Empty,
            Reply::Blocked,
            Reply::Fallback,
        ] {
            assert_eq!(
                parse_reply(&serde_json::to_vec(&reply).unwrap()),
                Some(reply)
            );
        }
    }
    #[tokio::test]
    async fn stalled_owned_helper_is_terminated_and_next_call_recovers() {
        let limits = Limits {
            stdin: 0,
            stdout: 32,
            stderr: 32,
            deadline: Duration::from_millis(80),
        };
        let child = LocalProcess::spawn("/bin/sleep", limits, |cmd| {
            cmd.arg("30");
        })
        .unwrap();
        let pid = child.id().unwrap();
        let result = child.run(b"", limits).await;
        assert!(result.is_err());
        tokio::time::sleep(Duration::from_millis(100)).await;
        let alive = std::process::Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success();
        assert!(!alive, "Timed-out owned helper must not survive");
        let result = LocalProcess::spawn("/usr/bin/printf", limits, |cmd| {
            cmd.arg("ok");
        })
        .unwrap()
        .run(b"", limits)
        .await
        .unwrap();
        assert_eq!(result.stdout, b"ok");
    }
}
