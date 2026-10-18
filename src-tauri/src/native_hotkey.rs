use std::collections::HashSet;
#[cfg(any(target_os = "macos", target_os = "windows", test))]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri_plugin_global_shortcut::ShortcutState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeHotkeyTrigger {
    Fn,
    FnSpace,
    FnLeftShift,
    Shift,
    LeftShift,
    RightShift,
    Ctrl,
    LeftCtrl,
    RightCtrl,
    Alt,
    LeftAlt,
    RightAlt,
    RightAltSpace,
    RightAltLeftShift,
    Meta,
    LeftMeta,
    RightMeta,
}

impl NativeHotkeyTrigger {
    pub fn canonical(self) -> &'static str {
        match self {
            Self::Fn => "Fn",
            Self::FnSpace => "Fn+Space",
            Self::FnLeftShift => "Fn+LeftShift",
            Self::Shift => "Shift",
            Self::LeftShift => "LeftShift",
            Self::RightShift => "RightShift",
            Self::Ctrl => "Ctrl",
            Self::LeftCtrl => "LeftCtrl",
            Self::RightCtrl => "RightCtrl",
            Self::Alt => "Alt",
            Self::LeftAlt => "LeftAlt",
            Self::RightAlt => "RightAlt",
            Self::RightAltSpace => "RightAlt+Space",
            Self::RightAltLeftShift => "RightAlt+LeftShift",
            Self::Meta => "Meta",
            Self::LeftMeta => "LeftMeta",
            Self::RightMeta => "RightMeta",
        }
    }

    #[cfg(test)]
    fn base(self) -> NativeHotkeyTrigger {
        match self {
            Self::Fn | Self::FnSpace | Self::FnLeftShift => Self::Fn,
            Self::RightAlt | Self::RightAltSpace | Self::RightAltLeftShift => Self::RightAlt,
            other => other,
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows", test))]
    fn from_base_combo(base: NativeHotkeyTrigger, combo: NativeComboKey) -> Option<Self> {
        match (base, combo) {
            (Self::Fn, NativeComboKey::Space) => Some(Self::FnSpace),
            (Self::Fn, NativeComboKey::LeftShift) => Some(Self::FnLeftShift),
            (Self::RightAlt, NativeComboKey::Space) => Some(Self::RightAltSpace),
            (Self::RightAlt, NativeComboKey::LeftShift) => Some(Self::RightAltLeftShift),
            _ => None,
        }
    }

    pub fn is_modifier_tap(self) -> bool {
        matches!(
            self,
            Self::Shift
                | Self::LeftShift
                | Self::RightShift
                | Self::Ctrl
                | Self::LeftCtrl
                | Self::RightCtrl
                | Self::Alt
                | Self::LeftAlt
                | Self::RightAlt
                | Self::Meta
                | Self::LeftMeta
                | Self::RightMeta
        )
    }

    pub fn overlaps(self, other: Self) -> bool {
        if self == other {
            return true;
        }
        PhysicalModifier::ALL
            .iter()
            .any(|key| self.matches_modifier(*key) && other.matches_modifier(*key))
    }

    fn matches_modifier(self, key: PhysicalModifier) -> bool {
        matches!(
            (self, key),
            (
                Self::Shift,
                PhysicalModifier::LeftShift | PhysicalModifier::RightShift
            ) | (Self::LeftShift, PhysicalModifier::LeftShift)
                | (Self::RightShift, PhysicalModifier::RightShift)
                | (
                    Self::Ctrl,
                    PhysicalModifier::LeftCtrl | PhysicalModifier::RightCtrl
                )
                | (Self::LeftCtrl, PhysicalModifier::LeftCtrl)
                | (Self::RightCtrl, PhysicalModifier::RightCtrl)
                | (
                    Self::Alt,
                    PhysicalModifier::LeftAlt | PhysicalModifier::RightAlt
                )
                | (Self::LeftAlt, PhysicalModifier::LeftAlt)
                | (Self::RightAlt, PhysicalModifier::RightAlt)
                | (
                    Self::Meta,
                    PhysicalModifier::LeftMeta | PhysicalModifier::RightMeta
                )
                | (Self::LeftMeta, PhysicalModifier::LeftMeta)
                | (Self::RightMeta, PhysicalModifier::RightMeta)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhysicalModifier {
    LeftShift,
    RightShift,
    LeftCtrl,
    RightCtrl,
    LeftAlt,
    RightAlt,
    LeftMeta,
    RightMeta,
}

impl PhysicalModifier {
    const ALL: [Self; 8] = [
        Self::LeftShift,
        Self::RightShift,
        Self::LeftCtrl,
        Self::RightCtrl,
        Self::LeftAlt,
        Self::RightAlt,
        Self::LeftMeta,
        Self::RightMeta,
    ];

    fn bit(self) -> u8 {
        1 << Self::ALL.iter().position(|key| *key == self).unwrap()
    }
}

#[derive(Default)]
struct ModifierTapState {
    held: u8,
    candidate: Option<PhysicalModifier>,
    other_held: HashSet<u32>,
}

impl ModifierTapState {
    fn modifier_edge(&mut self, key: PhysicalModifier, pressed: bool) -> Option<PhysicalModifier> {
        let bit = key.bit();
        if pressed {
            if self.held & bit != 0 {
                return None;
            }
            if self.held == 0 && self.other_held.is_empty() {
                self.candidate = Some(key);
            } else {
                self.candidate = None;
            }
            self.held |= bit;
            None
        } else {
            if self.held & bit == 0 {
                return None;
            }
            self.held &= !bit;
            let tapped = if self.candidate == Some(key) && self.held == 0 {
                Some(key)
            } else {
                None
            };
            self.candidate = None;
            tapped
        }
    }

    fn other_key_edge(&mut self, code: u32, pressed: bool) {
        if pressed {
            self.other_held.insert(code);
            self.candidate = None;
        } else {
            self.other_held.remove(&code);
        }
    }

    fn cancel(&mut self) {
        self.candidate = None;
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum NativeComboKey {
    Space,
    LeftShift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeHotkeyBinding {
    pub role: crate::hotkey::HotkeyRole,
    pub index: usize,
    pub trigger: NativeHotkeyTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeHotkeyEvent {
    pub role: crate::hotkey::HotkeyRole,
    pub index: usize,
    pub state: ShortcutState,
    pub tap_only: bool,
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
#[derive(Default)]
struct NativeHeldState {
    held: AtomicBool,
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
impl NativeHeldState {
    fn edge(&self, pressed: bool) -> Option<ShortcutState> {
        match (pressed, self.held.swap(pressed, Ordering::SeqCst)) {
            (true, false) => Some(ShortcutState::Pressed),
            (false, true) => Some(ShortcutState::Released),
            _ => None,
        }
    }
}

#[derive(Clone, Default)]
pub struct NativeHotkeyRuntime {
    inner: Arc<Mutex<NativeHotkeyRuntimeInner>>,
}

#[derive(Default)]
struct NativeHotkeyRuntimeInner {
    monitor: Option<platform::PlatformNativeMonitor>,
}

impl NativeHotkeyRuntime {
    pub fn install(
        &self,
        bindings: Vec<NativeHotkeyBinding>,
        hold_dictation: bool,
        handler: Arc<dyn Fn(NativeHotkeyEvent) + Send + Sync>,
    ) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let _ = inner.monitor.take();

        if bindings.is_empty() {
            return Ok(());
        }

        inner.monitor = Some(platform::PlatformNativeMonitor::start(
            bindings,
            hold_dictation,
            handler,
        )?);
        Ok(())
    }

    pub fn pause(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let _ = inner.monitor.take();
    }
}

type NativeHotkeyHandler = Arc<dyn Fn(NativeHotkeyEvent) + Send + Sync + 'static>;

#[cfg(any(target_os = "macos", target_os = "windows", test))]
struct NativeMonitoredBinding {
    binding: NativeHotkeyBinding,
    held: NativeHeldState,
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
impl NativeMonitoredBinding {
    fn new(binding: NativeHotkeyBinding) -> Self {
        Self {
            binding,
            held: NativeHeldState::default(),
        }
    }
}

#[cfg(test)]
fn monitored_bindings_for_base(
    bindings: Vec<NativeHotkeyBinding>,
    base: NativeHotkeyTrigger,
) -> Vec<NativeMonitoredBinding> {
    bindings
        .into_iter()
        .filter(|binding| binding.trigger.base() == base)
        .map(NativeMonitoredBinding::new)
        .collect()
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn has_binding_for_trigger(
    bindings: &[NativeMonitoredBinding],
    trigger: NativeHotkeyTrigger,
) -> bool {
    bindings
        .iter()
        .any(|binding| binding.binding.trigger == trigger)
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn dispatch_matching_bindings(
    bindings: &[NativeMonitoredBinding],
    trigger: NativeHotkeyTrigger,
    pressed: bool,
    handler: &NativeHotkeyHandler,
) -> bool {
    let mut matched = false;
    for binding in bindings {
        if binding.binding.trigger != trigger {
            continue;
        }
        matched = true;
        if let Some(state) = binding.held.edge(pressed) {
            (handler.as_ref())(NativeHotkeyEvent {
                role: binding.binding.role,
                index: binding.binding.index,
                state,
                tap_only: trigger.is_modifier_tap(),
            });
        }
    }
    matched
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn dispatch_modifier_tap(
    bindings: &[NativeMonitoredBinding],
    key: PhysicalModifier,
    handler: &NativeHotkeyHandler,
) {
    for binding in bindings {
        if !binding.binding.trigger.matches_modifier(key) {
            continue;
        }
        (handler.as_ref())(NativeHotkeyEvent {
            role: binding.binding.role,
            index: binding.binding.index,
            state: ShortcutState::Pressed,
            tap_only: true,
        });
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
#[derive(Default)]
struct NativeComboState {
    base_pressed: bool,
    pending_base_press: bool,
    combo_used: bool,
    hold_base: bool,
    edge_generation: u64,
}

#[cfg(any(target_os = "macos", test))]
fn activate_pending_hold_base(
    state: &mut NativeComboState,
    bindings: &[NativeMonitoredBinding],
    base: NativeHotkeyTrigger,
    edge_generation: u64,
    handler: &NativeHotkeyHandler,
) -> bool {
    if !state.hold_base
        || !state.base_pressed
        || !state.pending_base_press
        || state.combo_used
        || state.edge_generation != edge_generation
    {
        return false;
    }
    state.pending_base_press = false;
    dispatch_matching_bindings(bindings, base, true, handler)
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn dispatch_native_base_edge(
    state: &mut NativeComboState,
    bindings: &[NativeMonitoredBinding],
    base: NativeHotkeyTrigger,
    pressed: bool,
    handler: &NativeHotkeyHandler,
) -> bool {
    if state.base_pressed == pressed {
        return false;
    }
    state.edge_generation = state.edge_generation.wrapping_add(1);
    if pressed {
        state.base_pressed = true;
        state.combo_used = false;
        state.pending_base_press = true;
        if state.pending_base_press {
            return true;
        }
        return dispatch_matching_bindings(bindings, base, true, handler);
    }

    if state.pending_base_press && !state.combo_used {
        if state.hold_base {
            state.base_pressed = false;
            state.pending_base_press = false;
            return true;
        }
        let matched = dispatch_matching_bindings(bindings, base, true, handler);
        let _ = dispatch_matching_bindings(bindings, base, false, handler);
        state.base_pressed = false;
        state.pending_base_press = false;
        return matched;
    }

    if state.combo_used {
        for combo in [NativeComboKey::Space, NativeComboKey::LeftShift] {
            if let Some(trigger) = NativeHotkeyTrigger::from_base_combo(base, combo) {
                let _ = dispatch_matching_bindings(bindings, trigger, false, handler);
            }
        }
        state.base_pressed = false;
        state.pending_base_press = false;
        state.combo_used = false;
        return true;
    }

    state.base_pressed = false;
    dispatch_matching_bindings(bindings, base, false, handler)
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn dispatch_native_combo_edge(
    state: &mut NativeComboState,
    bindings: &[NativeMonitoredBinding],
    base: NativeHotkeyTrigger,
    combo: NativeComboKey,
    pressed: bool,
    handler: &NativeHotkeyHandler,
) -> bool {
    if !state.base_pressed || (pressed && state.combo_used) {
        return false;
    }
    let Some(trigger) = NativeHotkeyTrigger::from_base_combo(base, combo) else {
        return false;
    };
    if !has_binding_for_trigger(bindings, trigger) {
        return false;
    }

    if pressed {
        if state.hold_base {
            // A slowly entered combo may follow an already active hold.
            // Stop the bare shortcut before dispatching the combo.
            let _ = dispatch_matching_bindings(bindings, base, false, handler);
        }
        state.combo_used = true;
        state.pending_base_press = false;
    }
    dispatch_matching_bindings(bindings, trigger, pressed, handler)
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{
        activate_pending_hold_base, dispatch_matching_bindings, dispatch_modifier_tap,
        dispatch_native_base_edge, dispatch_native_combo_edge, ModifierTapState, NativeComboKey,
        NativeComboState, NativeHotkeyHandler, NativeHotkeyTrigger, NativeMonitoredBinding,
        PhysicalModifier,
    };
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
    const HOLD_COMBO_GRACE_PERIOD: Duration = Duration::from_millis(200);

    type CgEventMask = u64;
    type CgEventType = u32;
    type CgEventTapLocation = u32;
    type CgEventTapPlacement = u32;
    type CgEventTapOptions = u32;
    type CgEventField = u32;
    type CgEventFlags = u64;
    type CfStringRef = *const c_void;
    type CfAllocatorRef = *const c_void;

    #[repr(C)]
    struct OpaqueCgEvent(c_void);
    type CgEventRef = *mut OpaqueCgEvent;

    #[repr(C)]
    struct OpaqueCfMachPort(c_void);
    type CfMachPortRef = *mut OpaqueCfMachPort;

    #[repr(C)]
    struct OpaqueCfRunLoop(c_void);
    type CfRunLoopRef = *mut OpaqueCfRunLoop;

    #[repr(C)]
    struct OpaqueCfRunLoopSource(c_void);
    type CfRunLoopSourceRef = *mut OpaqueCfRunLoopSource;

    const SESSION_EVENT_TAP: CgEventTapLocation = 1;
    const HEAD_INSERT: CgEventTapPlacement = 0;
    const TAP_OPTION_DEFAULT: CgEventTapOptions = 0;

    const KEY_DOWN: CgEventType = 10;
    const KEY_UP: CgEventType = 11;
    const FLAGS_CHANGED: CgEventType = 12;
    const TAP_DISABLED_BY_TIMEOUT: CgEventType = 0xFFFF_FFFE;
    const TAP_DISABLED_BY_USER_INPUT: CgEventType = 0xFFFF_FFFF;

    const KEYBOARD_EVENT_KEYCODE: CgEventField = 9;
    const FLAG_MASK_SECONDARY_FN: CgEventFlags = 0x0080_0000;
    const FN_KEYCODE: i64 = 63;
    const SPACE_KEYCODE: i64 = 49;
    const LEFT_SHIFT_KEYCODE: i64 = 56;
    const MODIFIER_KEYS: [(i64, PhysicalModifier, CgEventFlags); 8] = [
        (56, PhysicalModifier::LeftShift, 0x0000_0002),
        (60, PhysicalModifier::RightShift, 0x0000_0004),
        (59, PhysicalModifier::LeftCtrl, 0x0000_0001),
        (62, PhysicalModifier::RightCtrl, 0x0000_2000),
        (58, PhysicalModifier::LeftAlt, 0x0000_0020),
        (61, PhysicalModifier::RightAlt, 0x0000_0040),
        (55, PhysicalModifier::LeftMeta, 0x0000_0008),
        (54, PhysicalModifier::RightMeta, 0x0000_0010),
    ];

    type CgEventTapCallBack = extern "C" fn(
        proxy: *mut c_void,
        event_type: CgEventType,
        event: CgEventRef,
        user_info: *mut c_void,
    ) -> CgEventRef;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventTapCreate(
            tap: CgEventTapLocation,
            place: CgEventTapPlacement,
            options: CgEventTapOptions,
            events_of_interest: CgEventMask,
            callback: CgEventTapCallBack,
            user_info: *mut c_void,
        ) -> CfMachPortRef;
        fn CGEventTapEnable(tap: CfMachPortRef, enable: bool);
        fn CGEventGetIntegerValueField(event: CgEventRef, field: CgEventField) -> i64;
        fn CGEventGetFlags(event: CgEventRef) -> CgEventFlags;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFMachPortCreateRunLoopSource(
            allocator: CfAllocatorRef,
            port: CfMachPortRef,
            order: isize,
        ) -> CfRunLoopSourceRef;
        fn CFRunLoopGetCurrent() -> CfRunLoopRef;
        fn CFRunLoopAddSource(rl: CfRunLoopRef, source: CfRunLoopSourceRef, mode: CfStringRef);
        fn CFRunLoopRun();
        fn CFRunLoopStop(rl: CfRunLoopRef);
        fn CFRelease(cf: *const c_void);
        static kCFRunLoopCommonModes: CfStringRef;
    }

    struct MacShutdownHandles {
        tap: Mutex<Option<CfMachPortRef>>,
        runloop: Mutex<Option<CfRunLoopRef>>,
        cancelled: AtomicBool,
    }

    unsafe impl Send for MacShutdownHandles {}
    unsafe impl Sync for MacShutdownHandles {}

    impl MacShutdownHandles {
        fn new() -> Self {
            Self {
                tap: Mutex::new(None),
                runloop: Mutex::new(None),
                cancelled: AtomicBool::new(false),
            }
        }

        fn shutdown(&self) {
            self.cancelled.store(true, Ordering::SeqCst);
            if let Some(tap) = self.tap.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                unsafe { CGEventTapEnable(*tap, false) };
            }
            if let Some(runloop) = self
                .runloop
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
            {
                unsafe { CFRunLoopStop(*runloop) };
            }
        }
    }

    pub struct PlatformNativeMonitor {
        handles: Arc<MacShutdownHandles>,
    }

    impl PlatformNativeMonitor {
        pub fn start(
            bindings: Vec<super::NativeHotkeyBinding>,
            hold_dictation: bool,
            handler: NativeHotkeyHandler,
        ) -> Result<Self, String> {
            let bindings: Vec<_> = bindings
                .into_iter()
                .map(NativeMonitoredBinding::new)
                .collect();
            if bindings.is_empty() {
                return Err("No supported macOS native hotkeys were provided".to_string());
            }

            let handles = Arc::new(MacShutdownHandles::new());
            let thread_handles = Arc::clone(&handles);
            let (status_tx, status_rx) = mpsc::channel();
            thread::Builder::new()
                .name("opentypeless-native-hotkey-mac".to_string())
                .spawn(move || {
                    run_event_tap_loop(bindings, hold_dictation, handler, thread_handles, status_tx)
                })
                .map_err(|error| {
                    format!("Failed to spawn macOS native hotkey monitor thread: {error}")
                })?;

            match status_rx.recv_timeout(STARTUP_TIMEOUT) {
                Ok(Ok(())) => Ok(Self { handles }),
                Ok(Err(error)) => {
                    handles.shutdown();
                    Err(error)
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    handles.shutdown();
                    Err(
                        "Timed out starting macOS native hotkey EventTap after 3 seconds"
                            .to_string(),
                    )
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    handles.shutdown();
                    Err(
                        "macOS native hotkey EventTap thread exited before startup completed"
                            .to_string(),
                    )
                }
            }
        }
    }

    impl Drop for PlatformNativeMonitor {
        fn drop(&mut self) {
            self.handles.shutdown();
        }
    }

    struct CallbackContext {
        bindings: Arc<Vec<NativeMonitoredBinding>>,
        handler: NativeHotkeyHandler,
        handles: Arc<MacShutdownHandles>,
        state: Arc<Mutex<NativeComboState>>,
        modifier_state: Mutex<ModifierTapState>,
    }

    fn run_event_tap_loop(
        bindings: Vec<NativeMonitoredBinding>,
        hold_dictation: bool,
        handler: NativeHotkeyHandler,
        handles: Arc<MacShutdownHandles>,
        status_tx: mpsc::Sender<Result<(), String>>,
    ) {
        let hold_base = hold_dictation
            && bindings.iter().any(|binding| {
                binding.binding.trigger == NativeHotkeyTrigger::Fn
                    && binding.binding.role == crate::hotkey::HotkeyRole::Dictation
            });
        let context = Box::into_raw(Box::new(CallbackContext {
            bindings: Arc::new(bindings),
            handler,
            handles: Arc::clone(&handles),
            state: Arc::new(Mutex::new(NativeComboState {
                hold_base,
                ..NativeComboState::default()
            })),
            modifier_state: Mutex::new(ModifierTapState::default()),
        }));
        let mask: CgEventMask = (1u64 << FLAGS_CHANGED) | (1u64 << KEY_DOWN) | (1u64 << KEY_UP);

        unsafe {
            let tap = CGEventTapCreate(
                SESSION_EVENT_TAP,
                HEAD_INSERT,
                TAP_OPTION_DEFAULT,
                mask,
                event_tap_callback,
                context as *mut c_void,
            );
            if tap.is_null() {
                drop(Box::from_raw(context));
                let _ = status_tx.send(Err(
                    "Failed to create macOS native hotkey EventTap; Accessibility permission may be denied"
                        .to_string(),
                ));
                return;
            }
            *handles.tap.lock().unwrap_or_else(|e| e.into_inner()) = Some(tap);

            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if source.is_null() {
                CGEventTapEnable(tap, false);
                *handles.tap.lock().unwrap_or_else(|e| e.into_inner()) = None;
                CFRelease(tap as *const c_void);
                drop(Box::from_raw(context));
                let _ = status_tx.send(Err(
                    "Failed to create macOS native hotkey EventTap run loop source".to_string(),
                ));
                return;
            }

            let runloop = CFRunLoopGetCurrent();
            *handles.runloop.lock().unwrap_or_else(|e| e.into_inner()) = Some(runloop);

            if handles.cancelled.load(Ordering::SeqCst) {
                CGEventTapEnable(tap, false);
                CFRelease(source as *const c_void);
                *handles.tap.lock().unwrap_or_else(|e| e.into_inner()) = None;
                *handles.runloop.lock().unwrap_or_else(|e| e.into_inner()) = None;
                CFRelease(tap as *const c_void);
                drop(Box::from_raw(context));
                let _ = status_tx.send(Err(
                    "macOS native hotkey EventTap startup was cancelled".to_string()
                ));
                return;
            }

            CFRunLoopAddSource(runloop, source, kCFRunLoopCommonModes);
            CFRelease(source as *const c_void);
            CGEventTapEnable(tap, true);

            if status_tx.send(Ok(())).is_err() || handles.cancelled.load(Ordering::SeqCst) {
                handles.shutdown();
            }

            if !handles.cancelled.load(Ordering::SeqCst) {
                CFRunLoopRun();
            }

            if let Some(tap) = handles.tap.lock().unwrap_or_else(|e| e.into_inner()).take() {
                CGEventTapEnable(tap, false);
                CFRelease(tap as *const c_void);
            }
            *handles.runloop.lock().unwrap_or_else(|e| e.into_inner()) = None;
            drop(Box::from_raw(context));
        }
    }

    extern "C" fn event_tap_callback(
        _proxy: *mut c_void,
        event_type: CgEventType,
        event: CgEventRef,
        user_info: *mut c_void,
    ) -> CgEventRef {
        if user_info.is_null() {
            return event;
        }
        let context = unsafe { &*(user_info as *const CallbackContext) };

        match event_type {
            TAP_DISABLED_BY_TIMEOUT | TAP_DISABLED_BY_USER_INPUT => {
                // macOS can disable the tap while a key is held. Clear the
                // captured state so a missing key-up cannot leave Fn stuck.
                let mut state = context.state.lock().unwrap_or_else(|e| e.into_inner());
                if state.hold_base && state.base_pressed && !state.pending_base_press {
                    let _ = dispatch_matching_bindings(
                        &context.bindings,
                        NativeHotkeyTrigger::Fn,
                        false,
                        &context.handler,
                    );
                }
                state.combo_used = true;
                let _ = dispatch_native_base_edge(
                    &mut state,
                    &context.bindings,
                    NativeHotkeyTrigger::Fn,
                    false,
                    &context.handler,
                );
                drop(state);
                *context
                    .modifier_state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = ModifierTapState::default();
                if let Some(tap) = context
                    .handles
                    .tap
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .as_ref()
                {
                    unsafe { CGEventTapEnable(*tap, true) };
                }
            }
            FLAGS_CHANGED => handle_flags_changed(context, event),
            KEY_DOWN => handle_key_event(context, event, true),
            KEY_UP => handle_key_event(context, event, false),
            _ => {}
        }

        event
    }

    fn handle_flags_changed(context: &CallbackContext, event: CgEventRef) {
        let keycode = unsafe { CGEventGetIntegerValueField(event, KEYBOARD_EVENT_KEYCODE) };
        let flags = unsafe { CGEventGetFlags(event) };
        if keycode == FN_KEYCODE {
            let pressed = (flags & FLAG_MASK_SECONDARY_FN) != 0;
            context
                .modifier_state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .other_key_edge(FN_KEYCODE as u32, pressed);
            let mut state = context.state.lock().unwrap_or_else(|e| e.into_inner());
            let was_pressed = state.base_pressed;
            let _ = dispatch_native_base_edge(
                &mut state,
                &context.bindings,
                NativeHotkeyTrigger::Fn,
                pressed,
                &context.handler,
            );
            if pressed && !was_pressed && state.hold_base && state.pending_base_press {
                let generation = state.edge_generation;
                let state = Arc::clone(&context.state);
                let bindings = Arc::clone(&context.bindings);
                let handler = Arc::clone(&context.handler);
                let handles = Arc::clone(&context.handles);
                if let Err(error) = thread::Builder::new()
                    .name("opentypeless-fn-hold-delay".to_string())
                    .spawn(move || {
                        thread::sleep(HOLD_COMBO_GRACE_PERIOD);
                        if handles.cancelled.load(Ordering::SeqCst) {
                            return;
                        }
                        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                        let _ = activate_pending_hold_base(
                            &mut state,
                            &bindings,
                            NativeHotkeyTrigger::Fn,
                            generation,
                            &handler,
                        );
                    })
                {
                    tracing::warn!("Failed to schedule Fn hold hotkey: {error}");
                }
            }
            return;
        }

        if let Some((_, key, side_flag)) =
            MODIFIER_KEYS.iter().find(|(code, _, _)| *code == keycode)
        {
            let pressed = flags & *side_flag != 0;
            let tapped = {
                let mut modifier_state = context
                    .modifier_state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                let tapped = modifier_state.modifier_edge(*key, pressed);
                if flags & (0x0000_207f & !*side_flag) != 0 {
                    modifier_state.cancel();
                    None
                } else {
                    tapped
                }
            };
            if let Some(key) = tapped {
                dispatch_modifier_tap(&context.bindings, key, &context.handler);
            }
            let mut state = context.state.lock().unwrap_or_else(|e| e.into_inner());
            let matched = if keycode == LEFT_SHIFT_KEYCODE {
                dispatch_native_combo_edge(
                    &mut state,
                    &context.bindings,
                    NativeHotkeyTrigger::Fn,
                    NativeComboKey::LeftShift,
                    pressed,
                    &context.handler,
                )
            } else {
                false
            };
            if pressed && !matched && state.base_pressed {
                state.combo_used = true;
            }
            return;
        }

        context
            .modifier_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .cancel();
        let mut state = context.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.base_pressed {
            state.combo_used = true;
        }
    }

    fn handle_key_event(context: &CallbackContext, event: CgEventRef, pressed: bool) {
        let keycode = unsafe { CGEventGetIntegerValueField(event, KEYBOARD_EVENT_KEYCODE) };
        context
            .modifier_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .other_key_edge(keycode as u32, pressed);
        let mut state = context.state.lock().unwrap_or_else(|e| e.into_inner());
        let matched = keycode == SPACE_KEYCODE
            && dispatch_native_combo_edge(
                &mut state,
                &context.bindings,
                NativeHotkeyTrigger::Fn,
                NativeComboKey::Space,
                pressed,
                &context.handler,
            );
        if pressed && !matched && state.base_pressed {
            state.combo_used = true;
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::{
        dispatch_modifier_tap, dispatch_native_base_edge, dispatch_native_combo_edge,
        ModifierTapState, NativeComboKey, NativeComboState, NativeHotkeyHandler,
        NativeHotkeyTrigger, NativeMonitoredBinding, PhysicalModifier,
    };
    use std::ptr;
    use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering as AtomicOrdering};
    use std::sync::{mpsc, Arc};
    use std::thread;
    use std::time::Duration;
    use windows_sys::Win32::Foundation::{GetLastError, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, PeekMessageW, PostThreadMessageW,
        SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, HHOOK,
        KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, MSG, PM_NOREMOVE, WH_KEYBOARD_LL, WM_QUIT,
    };

    const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
    const ACCEPT_SYNTHETIC_EVENTS_ENV: &str = "OPENTYPELESS_ACCEPT_SYNTHETIC_HOTKEY_EVENTS";

    const WM_KEYDOWN: usize = 0x0100;
    const WM_KEYUP: usize = 0x0101;
    const WM_SYSKEYDOWN: usize = 0x0104;
    const WM_SYSKEYUP: usize = 0x0105;
    const VK_SPACE: u32 = 0x20;
    const VK_RMENU: u32 = 0xA5;
    const VK_LSHIFT: u32 = 0xA0;
    const VK_RSHIFT: u32 = 0xA1;
    const VK_LCONTROL: u32 = 0xA2;
    const VK_RCONTROL: u32 = 0xA3;
    const VK_LMENU: u32 = 0xA4;
    const VK_LWIN: u32 = 0x5B;
    const VK_RWIN: u32 = 0x5C;

    static HOOK_CONTEXT: AtomicPtr<CallbackContext> = AtomicPtr::new(ptr::null_mut());

    pub struct PlatformNativeMonitor {
        thread_id: u32,
        thread: Option<thread::JoinHandle<()>>,
    }

    struct WindowsStartupState {
        thread_id: AtomicU32,
        cancelled: AtomicBool,
    }

    impl WindowsStartupState {
        fn new() -> Self {
            Self {
                thread_id: AtomicU32::new(0),
                cancelled: AtomicBool::new(false),
            }
        }

        fn cancel(&self) {
            self.cancelled.store(true, AtomicOrdering::SeqCst);
            let thread_id = self.thread_id.load(AtomicOrdering::SeqCst);
            if thread_id != 0 {
                let ok = unsafe { PostThreadMessageW(thread_id, WM_QUIT, 0, 0) };
                if ok == 0 {
                    tracing::warn!(
                        "Failed to post WM_QUIT to cancelled Windows native hotkey hook thread: {}",
                        unsafe { GetLastError() }
                    );
                }
            }
        }

        fn is_cancelled(&self) -> bool {
            self.cancelled.load(AtomicOrdering::SeqCst)
        }
    }

    impl PlatformNativeMonitor {
        pub fn start(
            bindings: Vec<super::NativeHotkeyBinding>,
            _hold_dictation: bool,
            handler: NativeHotkeyHandler,
        ) -> Result<Self, String> {
            let bindings: Vec<_> = bindings
                .into_iter()
                .map(NativeMonitoredBinding::new)
                .collect();
            if bindings.is_empty() {
                return Err("No supported Windows native hotkeys were provided".to_string());
            }

            let (status_tx, status_rx) = mpsc::channel();
            let startup = Arc::new(WindowsStartupState::new());
            let thread_startup = Arc::clone(&startup);
            let thread = thread::Builder::new()
                .name("opentypeless-native-hotkey-win".to_string())
                .spawn(move || run_keyboard_hook_loop(bindings, handler, thread_startup, status_tx))
                .map_err(|error| {
                    format!("Failed to spawn Windows native hotkey monitor thread: {error}")
                })?;

            match status_rx.recv_timeout(STARTUP_TIMEOUT) {
                Ok(Ok(thread_id)) => Ok(Self {
                    thread_id,
                    thread: Some(thread),
                }),
                Ok(Err(error)) => {
                    startup.cancel();
                    let _ = thread.join();
                    Err(error)
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    startup.cancel();
                    let _ = thread.join();
                    Err("Timed out starting Windows native hotkey hook after 3 seconds".to_string())
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    startup.cancel();
                    let _ = thread.join();
                    Err(
                        "Windows native hotkey hook thread exited before startup completed"
                            .to_string(),
                    )
                }
            }
        }
    }

    impl Drop for PlatformNativeMonitor {
        fn drop(&mut self) {
            let ok = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) };
            if ok == 0 {
                tracing::warn!(
                    "Failed to post WM_QUIT to Windows native hotkey hook thread: {}",
                    unsafe { GetLastError() }
                );
            }
            if let Some(thread) = self.thread.take() {
                if thread.thread().id() != thread::current().id() && thread.join().is_err() {
                    tracing::warn!("Windows native hotkey hook thread panicked during shutdown");
                }
            }
        }
    }

    struct CallbackContext {
        bindings: Vec<NativeMonitoredBinding>,
        handler: NativeHotkeyHandler,
        hook: std::sync::Mutex<Option<HHOOK>>,
        state: std::sync::Mutex<NativeComboState>,
        modifier_state: std::sync::Mutex<ModifierTapState>,
    }

    unsafe impl Send for CallbackContext {}
    unsafe impl Sync for CallbackContext {}

    fn run_keyboard_hook_loop(
        bindings: Vec<NativeMonitoredBinding>,
        handler: NativeHotkeyHandler,
        startup: Arc<WindowsStartupState>,
        status_tx: mpsc::Sender<Result<u32, String>>,
    ) {
        let thread_id = unsafe { GetCurrentThreadId() };

        unsafe {
            let mut message: MSG = std::mem::zeroed();
            let _ = PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_NOREMOVE);
            startup.thread_id.store(thread_id, AtomicOrdering::SeqCst);

            if startup.is_cancelled() {
                let _ = status_tx.send(Err(
                    "Windows native hotkey hook startup was cancelled before install".to_string(),
                ));
                return;
            }

            let context = Box::into_raw(Box::new(CallbackContext {
                bindings,
                handler,
                hook: std::sync::Mutex::new(None),
                state: std::sync::Mutex::new(NativeComboState::default()),
                modifier_state: std::sync::Mutex::new(ModifierTapState::default()),
            }));
            HOOK_CONTEXT.store(context, AtomicOrdering::SeqCst);

            // Global low-level hooks should be installed with the current module handle.
            let module = GetModuleHandleW(ptr::null());
            if module.is_null() {
                let error = GetLastError();
                let _ = HOOK_CONTEXT.compare_exchange(
                    context,
                    ptr::null_mut(),
                    AtomicOrdering::SeqCst,
                    AtomicOrdering::SeqCst,
                );
                drop(Box::from_raw(context));
                let _ = status_tx.send(Err(format!(
                    "Failed to resolve Windows module handle for native hotkey hook: Win32 error {error}"
                )));
                return;
            }

            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), module, 0);
            if hook.is_null() {
                let _ = HOOK_CONTEXT.compare_exchange(
                    context,
                    ptr::null_mut(),
                    AtomicOrdering::SeqCst,
                    AtomicOrdering::SeqCst,
                );
                drop(Box::from_raw(context));
                let _ = status_tx.send(Err(format!(
                    "Failed to install Windows native hotkey keyboard hook: Win32 error {}",
                    GetLastError()
                )));
                return;
            }
            *(*context).hook.lock().unwrap_or_else(|e| e.into_inner()) = Some(hook);

            if startup.is_cancelled() {
                cleanup_context(context);
                let _ = status_tx.send(Err(
                    "Windows native hotkey hook startup was cancelled after install".to_string(),
                ));
                return;
            }

            if status_tx.send(Ok(thread_id)).is_err() || startup.is_cancelled() {
                cleanup_context(context);
                return;
            }

            loop {
                let result = GetMessageW(&mut message, ptr::null_mut(), 0, 0);
                if result <= 0 {
                    break;
                }
                let _ = TranslateMessage(&message);
                let _ = DispatchMessageW(&message);
            }

            cleanup_context(context);
        }
    }

    unsafe fn cleanup_context(context: *mut CallbackContext) {
        if let Some(hook) = (*context)
            .hook
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            if UnhookWindowsHookEx(hook) == 0 {
                tracing::warn!(
                    "Failed to unhook Windows native hotkey keyboard hook: {}",
                    GetLastError()
                );
            }
        }
        let _ = HOOK_CONTEXT.compare_exchange(
            context,
            ptr::null_mut(),
            AtomicOrdering::SeqCst,
            AtomicOrdering::SeqCst,
        );
        drop(Box::from_raw(context));
    }

    unsafe extern "system" fn low_level_keyboard_proc(
        code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if code == HC_ACTION as i32 && lparam != 0 {
            let keyboard = *(lparam as *const KBDLLHOOKSTRUCT);
            let injected = (keyboard.flags & LLKHF_INJECTED) != 0;
            if injected && !accept_synthetic_events() {
                if let Some(context) = unsafe { callback_context() } {
                    context
                        .modifier_state
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .cancel();
                    context
                        .state
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .combo_used = true;
                }
            } else if dispatch_keyboard_event(
                keyboard.vkCode,
                keyboard.scanCode,
                keyboard.flags,
                wparam,
            ) {
                return 1;
            }
        }

        CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
    }

    fn dispatch_keyboard_event(vk_code: u32, scan_code: u32, flags: u32, message: WPARAM) -> bool {
        let pressed = match message {
            WM_KEYDOWN | WM_SYSKEYDOWN => true,
            WM_KEYUP | WM_SYSKEYUP => false,
            _ => return false,
        };

        let context = unsafe { callback_context() };
        let Some(context) = context else {
            return false;
        };

        let physical = match vk_code {
            VK_LSHIFT => Some(PhysicalModifier::LeftShift),
            VK_RSHIFT => Some(PhysicalModifier::RightShift),
            0x10 if scan_code == 0x36 => Some(PhysicalModifier::RightShift),
            0x10 => Some(PhysicalModifier::LeftShift),
            VK_LCONTROL => Some(PhysicalModifier::LeftCtrl),
            VK_RCONTROL => Some(PhysicalModifier::RightCtrl),
            0x11 if flags & LLKHF_EXTENDED != 0 => Some(PhysicalModifier::RightCtrl),
            0x11 => Some(PhysicalModifier::LeftCtrl),
            VK_LMENU => Some(PhysicalModifier::LeftAlt),
            VK_RMENU => Some(PhysicalModifier::RightAlt),
            0x12 if flags & LLKHF_EXTENDED != 0 => Some(PhysicalModifier::RightAlt),
            0x12 => Some(PhysicalModifier::LeftAlt),
            VK_LWIN => Some(PhysicalModifier::LeftMeta),
            VK_RWIN => Some(PhysicalModifier::RightMeta),
            _ => None,
        };

        if let Some(key) = physical {
            let (other_held, tapped) = {
                let mut modifier_state = context
                    .modifier_state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                let other_held =
                    modifier_state.held & !key.bit() != 0 || !modifier_state.other_held.is_empty();
                let tapped = modifier_state.modifier_edge(key, pressed).filter(|tapped| {
                    *tapped != PhysicalModifier::RightAlt
                        || !context
                            .bindings
                            .iter()
                            .any(|binding| binding.binding.trigger == NativeHotkeyTrigger::RightAlt)
                });
                (other_held, tapped)
            };
            if let Some(tapped) = tapped {
                dispatch_modifier_tap(&context.bindings, tapped, &context.handler);
            }
            let mut state = context.state.lock().unwrap_or_else(|e| e.into_inner());
            if key == PhysicalModifier::RightAlt {
                let _ = dispatch_native_base_edge(
                    &mut state,
                    &context.bindings,
                    NativeHotkeyTrigger::RightAlt,
                    pressed,
                    &context.handler,
                );
                if pressed && other_held {
                    state.combo_used = true;
                }
            } else {
                let matched = key == PhysicalModifier::LeftShift
                    && dispatch_native_combo_edge(
                        &mut state,
                        &context.bindings,
                        NativeHotkeyTrigger::RightAlt,
                        NativeComboKey::LeftShift,
                        pressed,
                        &context.handler,
                    );
                if pressed && !matched && state.base_pressed {
                    state.combo_used = true;
                }
                return matched;
            }
            return false;
        }

        context
            .modifier_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .other_key_edge(vk_code, pressed);
        let mut state = context.state.lock().unwrap_or_else(|e| e.into_inner());
        let matched = vk_code == VK_SPACE
            && dispatch_native_combo_edge(
                &mut state,
                &context.bindings,
                NativeHotkeyTrigger::RightAlt,
                NativeComboKey::Space,
                pressed,
                &context.handler,
            );
        if pressed && !matched && state.base_pressed {
            state.combo_used = true;
        }
        matched
    }

    unsafe fn callback_context<'a>() -> Option<&'a CallbackContext> {
        let ptr = HOOK_CONTEXT.load(AtomicOrdering::SeqCst);
        if ptr.is_null() {
            None
        } else {
            Some(&*ptr)
        }
    }

    fn accept_synthetic_events() -> bool {
        std::env::var(ACCEPT_SYNTHETIC_EVENTS_ENV).ok().as_deref() == Some("1")
    }
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
mod platform {
    use super::{NativeHotkeyBinding, NativeHotkeyHandler};

    pub struct PlatformNativeMonitor;

    impl PlatformNativeMonitor {
        pub fn start(
            _bindings: Vec<NativeHotkeyBinding>,
            _hold_dictation: bool,
            _handler: NativeHotkeyHandler,
        ) -> Result<Self, String> {
            Err("Native hotkey runtime is unsupported on this platform".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_state_dedupes_repeat_edges() {
        let state = NativeHeldState::default();

        assert_eq!(state.edge(true), Some(ShortcutState::Pressed));
        assert_eq!(state.edge(true), None);
        assert_eq!(state.edge(false), Some(ShortcutState::Released));
        assert_eq!(state.edge(false), None);
        assert_eq!(state.edge(true), Some(ShortcutState::Pressed));
    }

    #[test]
    fn modifier_tap_requires_a_clean_keyup() {
        let mut state = ModifierTapState::default();
        assert_eq!(state.modifier_edge(PhysicalModifier::LeftCtrl, true), None);
        assert_eq!(state.modifier_edge(PhysicalModifier::LeftCtrl, true), None);
        assert_eq!(
            state.modifier_edge(PhysicalModifier::LeftCtrl, false),
            Some(PhysicalModifier::LeftCtrl)
        );
        assert_eq!(state.modifier_edge(PhysicalModifier::LeftCtrl, false), None);

        state.modifier_edge(PhysicalModifier::LeftCtrl, true);
        state.other_key_edge(0x43, true);
        state.other_key_edge(0x43, false);
        assert_eq!(state.modifier_edge(PhysicalModifier::LeftCtrl, false), None);

        state.other_key_edge(0x43, true);
        state.modifier_edge(PhysicalModifier::LeftCtrl, true);
        assert_eq!(state.modifier_edge(PhysicalModifier::LeftCtrl, false), None);
        state.other_key_edge(0x43, false);
    }

    #[test]
    fn modifier_tap_rejects_chords_and_interrupted_input() {
        let mut state = ModifierTapState::default();
        state.modifier_edge(PhysicalModifier::LeftShift, true);
        state.modifier_edge(PhysicalModifier::RightShift, true);
        assert_eq!(
            state.modifier_edge(PhysicalModifier::RightShift, false),
            None
        );
        assert_eq!(
            state.modifier_edge(PhysicalModifier::LeftShift, false),
            None
        );

        state.modifier_edge(PhysicalModifier::RightCtrl, true);
        state.cancel();
        assert_eq!(
            state.modifier_edge(PhysicalModifier::RightCtrl, false),
            None
        );
    }

    #[test]
    fn modifier_overlap_distinguishes_sides() {
        assert!(NativeHotkeyTrigger::Ctrl.overlaps(NativeHotkeyTrigger::RightCtrl));
        assert!(!NativeHotkeyTrigger::LeftCtrl.overlaps(NativeHotkeyTrigger::RightCtrl));
        assert!(NativeHotkeyTrigger::Alt.overlaps(NativeHotkeyTrigger::RightAlt));
        assert!(!NativeHotkeyTrigger::Fn.overlaps(NativeHotkeyTrigger::Ctrl));
    }

    #[test]
    fn generic_alt_tap_accepts_either_side_without_repeating() {
        let bindings = vec![NativeMonitoredBinding::new(NativeHotkeyBinding {
            role: crate::hotkey::HotkeyRole::Ask,
            index: 0,
            trigger: NativeHotkeyTrigger::Alt,
        })];
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let handler: NativeHotkeyHandler = Arc::new(move |event| {
            captured.lock().unwrap().push((event.state, event.tap_only));
        });
        let mut state = ModifierTapState::default();
        for key in [PhysicalModifier::LeftAlt, PhysicalModifier::RightAlt] {
            assert_eq!(state.modifier_edge(key, true), None);
            assert_eq!(state.modifier_edge(key, true), None);
            if let Some(tapped) = state.modifier_edge(key, false) {
                dispatch_modifier_tap(&bindings, tapped, &handler);
            }
        }
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                (ShortcutState::Pressed, true),
                (ShortcutState::Pressed, true),
            ]
        );
    }

    #[test]
    fn fn_tap_waits_for_keyup_and_ignores_a_used_chord() {
        let bindings = vec![NativeMonitoredBinding::new(NativeHotkeyBinding {
            role: crate::hotkey::HotkeyRole::Dictation,
            index: 0,
            trigger: NativeHotkeyTrigger::Fn,
        })];
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let handler: NativeHotkeyHandler = Arc::new(move |event| {
            captured.lock().unwrap().push(event.state);
        });
        let mut state = NativeComboState::default();
        dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            true,
            &handler,
        );
        assert!(events.lock().unwrap().is_empty());
        state.combo_used = true;
        dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            false,
            &handler,
        );
        assert!(events.lock().unwrap().is_empty());
        dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            true,
            &handler,
        );
        dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            false,
            &handler,
        );
        assert_eq!(
            *events.lock().unwrap(),
            vec![ShortcutState::Pressed, ShortcutState::Released]
        );
    }

    #[test]
    fn runtime_install_accepts_shared_handler_arc() {
        let runtime = NativeHotkeyRuntime::default();
        let handler: Arc<dyn Fn(NativeHotkeyEvent) + Send + Sync> = Arc::new(|_| {});

        assert!(runtime.install(Vec::new(), false, handler).is_ok());
    }

    #[test]
    fn combo_state_suppresses_bare_base_when_combo_is_used() {
        let bindings = vec![
            NativeMonitoredBinding::new(NativeHotkeyBinding {
                role: crate::hotkey::HotkeyRole::Dictation,
                index: 0,
                trigger: NativeHotkeyTrigger::Fn,
            }),
            NativeMonitoredBinding::new(NativeHotkeyBinding {
                role: crate::hotkey::HotkeyRole::Ask,
                index: 0,
                trigger: NativeHotkeyTrigger::FnSpace,
            }),
        ];
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let handler: NativeHotkeyHandler = Arc::new(move |event| {
            captured.lock().unwrap().push((event.role, event.state));
        });
        let mut state = NativeComboState::default();

        assert!(dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            true,
            &handler,
        ));
        assert!(events.lock().unwrap().is_empty());

        assert!(dispatch_native_combo_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            NativeComboKey::Space,
            true,
            &handler,
        ));
        assert!(dispatch_native_combo_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            NativeComboKey::Space,
            false,
            &handler,
        ));
        assert!(dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            false,
            &handler,
        ));

        assert_eq!(
            *events.lock().unwrap(),
            vec![
                (crate::hotkey::HotkeyRole::Ask, ShortcutState::Pressed),
                (crate::hotkey::HotkeyRole::Ask, ShortcutState::Released),
            ]
        );
    }

    #[test]
    fn combo_state_dispatches_bare_base_when_no_combo_is_used() {
        let bindings = vec![
            NativeMonitoredBinding::new(NativeHotkeyBinding {
                role: crate::hotkey::HotkeyRole::Dictation,
                index: 0,
                trigger: NativeHotkeyTrigger::Fn,
            }),
            NativeMonitoredBinding::new(NativeHotkeyBinding {
                role: crate::hotkey::HotkeyRole::Ask,
                index: 0,
                trigger: NativeHotkeyTrigger::FnSpace,
            }),
        ];
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let handler: NativeHotkeyHandler = Arc::new(move |event| {
            captured.lock().unwrap().push((event.role, event.state));
        });
        let mut state = NativeComboState::default();

        assert!(dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            true,
            &handler,
        ));
        assert!(events.lock().unwrap().is_empty());

        assert!(dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            false,
            &handler,
        ));

        assert_eq!(
            *events.lock().unwrap(),
            vec![
                (crate::hotkey::HotkeyRole::Dictation, ShortcutState::Pressed),
                (
                    crate::hotkey::HotkeyRole::Dictation,
                    ShortcutState::Released
                ),
            ]
        );
    }

    #[test]
    fn hold_fn_starts_while_held_and_stops_on_release() {
        let bindings = vec![
            NativeMonitoredBinding::new(NativeHotkeyBinding {
                role: crate::hotkey::HotkeyRole::Dictation,
                index: 0,
                trigger: NativeHotkeyTrigger::Fn,
            }),
            NativeMonitoredBinding::new(NativeHotkeyBinding {
                role: crate::hotkey::HotkeyRole::Ask,
                index: 0,
                trigger: NativeHotkeyTrigger::FnSpace,
            }),
        ];
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let handler: NativeHotkeyHandler = Arc::new(move |event| {
            captured.lock().unwrap().push((event.role, event.state));
        });
        let mut state = NativeComboState {
            hold_base: true,
            ..NativeComboState::default()
        };

        assert!(dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            true,
            &handler,
        ));
        assert!(events.lock().unwrap().is_empty());

        let generation = state.edge_generation;
        assert!(activate_pending_hold_base(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            generation,
            &handler,
        ));
        assert_eq!(
            *events.lock().unwrap(),
            vec![(crate::hotkey::HotkeyRole::Dictation, ShortcutState::Pressed)]
        );

        assert!(dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            false,
            &handler,
        ));
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                (crate::hotkey::HotkeyRole::Dictation, ShortcutState::Pressed),
                (
                    crate::hotkey::HotkeyRole::Dictation,
                    ShortcutState::Released
                ),
            ]
        );
    }

    #[test]
    fn fn_combo_cancels_pending_hold_and_short_tap_does_not_start() {
        let bindings = vec![
            NativeMonitoredBinding::new(NativeHotkeyBinding {
                role: crate::hotkey::HotkeyRole::Dictation,
                index: 0,
                trigger: NativeHotkeyTrigger::Fn,
            }),
            NativeMonitoredBinding::new(NativeHotkeyBinding {
                role: crate::hotkey::HotkeyRole::Ask,
                index: 0,
                trigger: NativeHotkeyTrigger::FnSpace,
            }),
        ];
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let handler: NativeHotkeyHandler = Arc::new(move |event| {
            captured.lock().unwrap().push((event.role, event.state));
        });
        let mut state = NativeComboState {
            hold_base: true,
            ..NativeComboState::default()
        };

        let _ = dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            true,
            &handler,
        );
        let stale_generation = state.edge_generation;
        let _ = dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            false,
            &handler,
        );
        assert!(!activate_pending_hold_base(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            stale_generation,
            &handler,
        ));
        assert!(events.lock().unwrap().is_empty());

        let _ = dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            true,
            &handler,
        );
        let generation = state.edge_generation;
        let _ = dispatch_native_combo_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            NativeComboKey::Space,
            true,
            &handler,
        );
        assert!(!activate_pending_hold_base(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            generation,
            &handler,
        ));
        let _ = dispatch_native_base_edge(
            &mut state,
            &bindings,
            NativeHotkeyTrigger::Fn,
            false,
            &handler,
        );
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                (crate::hotkey::HotkeyRole::Ask, ShortcutState::Pressed),
                (crate::hotkey::HotkeyRole::Ask, ShortcutState::Released),
            ]
        );
    }
}
