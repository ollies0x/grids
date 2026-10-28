//! Raw C bindings for CoreFoundation, Accessibility, CoreGraphics events and Carbon hotkeys,
//! plus a small owned CF reference type. Kept as plain externs: these APIs are C, stable, and
//! this is all the surface Grids needs.
#![allow(non_upper_case_globals, non_snake_case, dead_code)]
// Apple's k-prefixed names are kept so they can be looked up in the SDK headers.

use std::ffi::{c_char, c_void, CString};
use std::ptr;

pub type CFTypeRef = *const c_void;
pub type CFStringRef = *const c_void;
pub type CFArrayRef = *const c_void;
pub type CFDictionaryRef = *const c_void;
pub type CFAllocatorRef = *const c_void;
pub type CFRunLoopRef = *const c_void;
pub type CFRunLoopSourceRef = *const c_void;
pub type CFMachPortRef = *const c_void;
pub type CFIndex = isize;
pub type AXUIElementRef = *const c_void;
pub type AXObserverRef = *const c_void;
pub type AXValueRef = *const c_void;
pub type AXError = i32;
pub type CGEventRef = *const c_void;
pub type CGEventTapProxy = *const c_void;
pub type CGEventMask = u64;
pub type CGDirectDisplayID = u32;
pub type CGWindowID = u32;
pub type OSStatus = i32;
pub type EventTargetRef = *const c_void;
pub type EventHandlerRef = *const c_void;
pub type EventHandlerCallRef = *const c_void;
pub type EventRef = *const c_void;
pub type EventHotKeyRef = *const c_void;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CGSize {
    pub width: f64,
    pub height: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

pub const kCFStringEncodingUTF8: u32 = 0x0800_0100;
pub const kCFNumberSInt64Type: i32 = 4;
pub const kCFNumberFloat64Type: i32 = 6;

pub const kAXErrorSuccess: AXError = 0;
pub const kAXValueCGPointType: u32 = 1;
pub const kAXValueCGSizeType: u32 = 2;

// CGEvent types
pub const kCGEventLeftMouseDown: u32 = 1;
pub const kCGEventLeftMouseUp: u32 = 2;
pub const kCGEventRightMouseDown: u32 = 3;
pub const kCGEventRightMouseUp: u32 = 4;
pub const kCGEventMouseMoved: u32 = 5;
pub const kCGEventLeftMouseDragged: u32 = 6;
pub const kCGEventRightMouseDragged: u32 = 7;
pub const kCGEventKeyDown: u32 = 10;
pub const kCGEventKeyUp: u32 = 11;
pub const kCGEventFlagsChanged: u32 = 12;
pub const kCGEventScrollWheel: u32 = 22;
pub const kCGEventOtherMouseDown: u32 = 25;
pub const kCGEventOtherMouseUp: u32 = 26;
pub const kCGEventOtherMouseDragged: u32 = 27;
pub const kCGEventTapDisabledByTimeout: u32 = 0xFFFF_FFFE;
pub const kCGEventTapDisabledByUserInput: u32 = 0xFFFF_FFFF;

pub const kCGMouseEventClickState: u32 = 1;
pub const kCGMouseEventButtonNumber: u32 = 3;
pub const kCGKeyboardEventAutorepeat: u32 = 8;
pub const kCGKeyboardEventKeycode: u32 = 9;
pub const kCGScrollWheelEventDeltaAxis1: u32 = 11;
pub const kCGScrollWheelEventPointDeltaAxis1: u32 = 96;
pub const kCGEventSourceUserData: u32 = 42;

pub const kCGSessionEventTap: u32 = 1;
pub const kCGHIDEventTap: u32 = 0;
pub const kCGHeadInsertEventTap: u32 = 0;
pub const kCGEventTapOptionDefault: u32 = 0;
pub const kCGEventTapOptionListenOnly: u32 = 1;

pub const kCGWindowListOptionOnScreenOnly: u32 = 1 << 0;
pub const kCGWindowListExcludeDesktopElements: u32 = 1 << 4;

/// Marker in kCGEventSourceUserData on events Grids posts, so its own tap ignores them.
pub const OUR_EVENT_MARK: i64 = 0x4752_4944; // "GRID"

pub type CGEventTapCallBack =
    extern "C" fn(proxy: CGEventTapProxy, ty: u32, event: CGEventRef, user: *mut c_void) -> CGEventRef;
pub type AXObserverCallback =
    extern "C" fn(observer: AXObserverRef, element: AXUIElementRef, notification: CFStringRef, refcon: *mut c_void);
pub type CFRunLoopTimerCallBack = extern "C" fn(timer: *const c_void, info: *mut c_void);
pub type EventHandlerUPP = extern "C" fn(EventHandlerCallRef, EventRef, *mut c_void) -> OSStatus;
pub type CFNotificationCallback =
    extern "C" fn(center: *const c_void, observer: *mut c_void, name: CFStringRef, object: *const c_void, info: CFDictionaryRef);

#[repr(C)]
pub struct CFRunLoopTimerContext {
    pub version: CFIndex,
    pub info: *mut c_void,
    pub retain: *const c_void,
    pub release: *const c_void,
    pub copy_description: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EventTypeSpec {
    pub event_class: u32,
    pub event_kind: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct EventHotKeyID {
    pub signature: u32,
    pub id: u32,
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub static kCFAllocatorDefault: CFAllocatorRef;
    pub static kCFRunLoopCommonModes: CFStringRef;
    pub static kCFBooleanTrue: CFTypeRef;
    pub static kCFBooleanFalse: CFTypeRef;
    pub fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFGetTypeID(cf: CFTypeRef) -> usize;
    pub fn CFStringGetTypeID() -> usize;
    pub fn CFBooleanGetTypeID() -> usize;
    pub fn CFNumberGetTypeID() -> usize;
    pub fn CFArrayGetTypeID() -> usize;
    pub fn CFDictionaryGetTypeID() -> usize;
    pub fn CFStringCreateWithCString(alloc: CFAllocatorRef, s: *const c_char, enc: u32) -> CFStringRef;
    pub fn CFStringGetLength(s: CFStringRef) -> CFIndex;
    pub fn CFStringGetMaximumSizeForEncoding(len: CFIndex, enc: u32) -> CFIndex;
    pub fn CFStringGetCString(s: CFStringRef, buf: *mut c_char, size: CFIndex, enc: u32) -> u8;
    pub fn CFArrayGetCount(a: CFArrayRef) -> CFIndex;
    pub fn CFArrayGetValueAtIndex(a: CFArrayRef, i: CFIndex) -> CFTypeRef;
    pub fn CFDictionaryGetValue(d: CFDictionaryRef, key: CFTypeRef) -> CFTypeRef;
    pub fn CFBooleanGetValue(b: CFTypeRef) -> u8;
    pub fn CFNumberGetValue(n: CFTypeRef, ty: i32, out: *mut c_void) -> u8;
    pub fn CFNumberCreate(alloc: CFAllocatorRef, ty: i32, val: *const c_void) -> CFTypeRef;
    pub fn CFRunLoopGetMain() -> CFRunLoopRef;
    pub fn CFRunLoopAddSource(rl: CFRunLoopRef, src: CFRunLoopSourceRef, mode: CFStringRef);
    pub fn CFRunLoopRemoveSource(rl: CFRunLoopRef, src: CFRunLoopSourceRef, mode: CFStringRef);
    pub fn CFMachPortCreateRunLoopSource(alloc: CFAllocatorRef, port: CFMachPortRef, order: CFIndex) -> CFRunLoopSourceRef;
    pub fn CFMachPortInvalidate(port: CFMachPortRef);
    pub fn CFRunLoopTimerCreate(
        alloc: CFAllocatorRef,
        fire: f64,
        interval: f64,
        flags: u64,
        order: CFIndex,
        cb: CFRunLoopTimerCallBack,
        ctx: *mut CFRunLoopTimerContext,
    ) -> *const c_void;
    pub fn CFRunLoopAddTimer(rl: CFRunLoopRef, timer: *const c_void, mode: CFStringRef);
    pub fn CFRunLoopTimerInvalidate(timer: *const c_void);
    pub fn CFAbsoluteTimeGetCurrent() -> f64;
    pub fn CFPreferencesCopyAppValue(key: CFStringRef, app: CFStringRef) -> CFTypeRef;
    pub fn CFNotificationCenterGetDistributedCenter() -> *const c_void;
    pub fn CFNotificationCenterAddObserver(
        center: *const c_void,
        observer: *const c_void,
        cb: CFNotificationCallback,
        name: CFStringRef,
        object: *const c_void,
        behavior: isize,
    );
    pub fn CFNotificationCenterPostNotification(
        center: *const c_void,
        name: CFStringRef,
        object: *const c_void,
        info: CFDictionaryRef,
        deliver_immediately: u8,
    );
    pub fn CFUUIDCreateString(alloc: CFAllocatorRef, uuid: CFTypeRef) -> CFStringRef;
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> u8;
    pub fn AXIsProcessTrusted() -> u8;
    pub fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    pub fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    pub fn AXUIElementCopyAttributeValue(el: AXUIElementRef, attr: CFStringRef, out: *mut CFTypeRef) -> AXError;
    pub fn AXUIElementSetAttributeValue(el: AXUIElementRef, attr: CFStringRef, value: CFTypeRef) -> AXError;
    pub fn AXUIElementIsAttributeSettable(el: AXUIElementRef, attr: CFStringRef, out: *mut u8) -> AXError;
    pub fn AXUIElementPerformAction(el: AXUIElementRef, action: CFStringRef) -> AXError;
    pub fn AXUIElementCopyElementAtPosition(app: AXUIElementRef, x: f32, y: f32, out: *mut AXUIElementRef) -> AXError;
    pub fn AXUIElementGetPid(el: AXUIElementRef, pid: *mut i32) -> AXError;
    pub fn AXUIElementSetMessagingTimeout(el: AXUIElementRef, secs: f32) -> AXError;
    pub fn AXValueCreate(ty: u32, ptr: *const c_void) -> AXValueRef;
    pub fn AXValueGetValue(v: AXValueRef, ty: u32, out: *mut c_void) -> u8;
    pub fn AXObserverCreate(pid: i32, cb: AXObserverCallback, out: *mut AXObserverRef) -> AXError;
    pub fn AXObserverAddNotification(obs: AXObserverRef, el: AXUIElementRef, n: CFStringRef, refcon: *mut c_void) -> AXError;
    pub fn AXObserverGetRunLoopSource(obs: AXObserverRef) -> CFRunLoopSourceRef;
    /// Private but long-stable: the CGWindowID behind an AX window element.
    pub fn _AXUIElementGetWindow(el: AXUIElementRef, id: *mut CGWindowID) -> AXError;

    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        mask: CGEventMask,
        cb: CGEventTapCallBack,
        user: *mut c_void,
    ) -> CFMachPortRef;
    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    pub fn CGEventGetLocation(e: CGEventRef) -> CGPoint;
    pub fn CGEventGetFlags(e: CGEventRef) -> u64;
    pub fn CGEventSetFlags(e: CGEventRef, flags: u64);
    pub fn CGEventGetIntegerValueField(e: CGEventRef, field: u32) -> i64;
    pub fn CGEventSetIntegerValueField(e: CGEventRef, field: u32, v: i64);
    pub fn CGEventCreate(src: *const c_void) -> CGEventRef;
    pub fn CGEventCreateMouseEvent(src: *const c_void, ty: u32, p: CGPoint, button: u32) -> CGEventRef;
    pub fn CGEventCreateKeyboardEvent(src: *const c_void, key: u16, down: bool) -> CGEventRef;
    pub fn CGEventPost(tap: u32, e: CGEventRef);
    pub fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;
    pub fn CGAssociateMouseAndMouseCursorPosition(connected: bool) -> i32;
    pub fn CGWindowListCopyWindowInfo(option: u32, relative: CGWindowID) -> CFArrayRef;
    pub fn CGRectMakeWithDictionaryRepresentation(d: CFDictionaryRef, out: *mut CGRect) -> bool;
    pub fn CGDisplayCreateUUIDFromDisplayID(id: CGDirectDisplayID) -> CFTypeRef;
    pub fn CGMainDisplayID() -> CGDirectDisplayID;
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    pub fn GetApplicationEventTarget() -> EventTargetRef;
    pub fn InstallEventHandler(
        target: EventTargetRef,
        handler: EventHandlerUPP,
        n: u32,
        types: *const EventTypeSpec,
        user: *mut c_void,
        out: *mut EventHandlerRef,
    ) -> OSStatus;
    pub fn RegisterEventHotKey(
        code: u32,
        mods: u32,
        id: EventHotKeyID,
        target: EventTargetRef,
        options: u32,
        out: *mut EventHotKeyRef,
    ) -> OSStatus;
    pub fn UnregisterEventHotKey(r: EventHotKeyRef) -> OSStatus;
    pub fn GetEventParameter(
        e: EventRef,
        name: u32,
        ty: u32,
        out_ty: *mut u32,
        size: usize,
        out_size: *mut usize,
        data: *mut c_void,
    ) -> OSStatus;
    pub fn GetEventKind(e: EventRef) -> u32;
}

pub const kEventClassKeyboard: u32 = u32::from_be_bytes(*b"keyb");
pub const kEventHotKeyPressed: u32 = 5;
pub const kEventHotKeyReleased: u32 = 6;
pub const kEventParamDirectObject: u32 = u32::from_be_bytes(*b"----");
pub const typeEventHotKeyID: u32 = u32::from_be_bytes(*b"hkid");

/// An owned CoreFoundation reference (released on drop).
pub struct Cf(pub CFTypeRef);

impl Cf {
    /// Take ownership of a +1 reference. Null stays null.
    pub fn own(p: CFTypeRef) -> Option<Cf> {
        if p.is_null() { None } else { Some(Cf(p)) }
    }
    /// Retain a borrowed (+0) reference.
    pub fn retain(p: CFTypeRef) -> Option<Cf> {
        if p.is_null() {
            None
        } else {
            unsafe { CFRetain(p) };
            Some(Cf(p))
        }
    }
    pub fn ptr(&self) -> CFTypeRef {
        self.0
    }
}

impl Clone for Cf {
    fn clone(&self) -> Self {
        unsafe { CFRetain(self.0) };
        Cf(self.0)
    }
}

impl Drop for Cf {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}

pub fn cfstr(s: &str) -> Cf {
    let c = CString::new(s).unwrap_or_default();
    Cf(unsafe { CFStringCreateWithCString(kCFAllocatorDefault, c.as_ptr(), kCFStringEncodingUTF8) })
}

pub fn cf_to_string(p: CFTypeRef) -> Option<String> {
    if p.is_null() || unsafe { CFGetTypeID(p) != CFStringGetTypeID() } {
        return None;
    }
    unsafe {
        let len = CFStringGetLength(p);
        let max = CFStringGetMaximumSizeForEncoding(len, kCFStringEncodingUTF8) + 1;
        let mut buf = vec![0u8; max as usize];
        if CFStringGetCString(p, buf.as_mut_ptr() as *mut c_char, max, kCFStringEncodingUTF8) == 0 {
            return None;
        }
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        Some(String::from_utf8_lossy(&buf[..end]).into_owned())
    }
}

pub fn cf_to_bool(p: CFTypeRef) -> Option<bool> {
    if p.is_null() {
        return None;
    }
    unsafe {
        let t = CFGetTypeID(p);
        if t == CFBooleanGetTypeID() {
            Some(CFBooleanGetValue(p) != 0)
        } else if t == CFNumberGetTypeID() {
            cf_to_f64(p).map(|v| v != 0.0)
        } else {
            None
        }
    }
}

pub fn cf_to_f64(p: CFTypeRef) -> Option<f64> {
    if p.is_null() || unsafe { CFGetTypeID(p) != CFNumberGetTypeID() } {
        return None;
    }
    let mut v = 0f64;
    let ok = unsafe { CFNumberGetValue(p, kCFNumberFloat64Type, &mut v as *mut f64 as *mut c_void) };
    if ok != 0 { Some(v) } else { None }
}

pub fn cf_number(v: f64) -> Cf {
    Cf(unsafe { CFNumberCreate(kCFAllocatorDefault, kCFNumberFloat64Type, &v as *const f64 as *const c_void) })
}

pub fn cf_bool(v: bool) -> CFTypeRef {
    unsafe { if v { kCFBooleanTrue } else { kCFBooleanFalse } }
}

pub fn dict_get(d: CFDictionaryRef, key: &str) -> CFTypeRef {
    if d.is_null() {
        return ptr::null();
    }
    let k = cfstr(key);
    unsafe { CFDictionaryGetValue(d, k.ptr()) }
}

/// Read a value from another app's preferences domain.
pub fn pref_value(domain: &str, key: &str) -> Option<Cf> {
    let (d, k) = (cfstr(domain), cfstr(key));
    Cf::own(unsafe { CFPreferencesCopyAppValue(k.ptr(), d.ptr()) })
}

pub fn now() -> f64 {
    unsafe { CFAbsoluteTimeGetCurrent() }
}

// kqueue file watching on the main run loop, and display reconfiguration notices.
pub type CFFileDescriptorRef = *const c_void;
pub type CFFileDescriptorCallBack = extern "C" fn(f: CFFileDescriptorRef, types: usize, info: *mut c_void);
pub type CGDisplayReconfigurationCallBack = extern "C" fn(display: u32, flags: u32, user: *mut c_void);
pub const kCGDisplayBeginConfigurationFlag: u32 = 1;

#[repr(C)]
pub struct KEvent {
    pub ident: usize,
    pub filter: i16,
    pub flags: u16,
    pub fflags: u32,
    pub data: isize,
    pub udata: *mut c_void,
}
pub const EVFILT_VNODE: i16 = -4;
pub const EV_ADD: u16 = 0x1;
pub const EV_CLEAR: u16 = 0x20;
pub const NOTE_DELETE: u32 = 0x1;
pub const NOTE_WRITE: u32 = 0x2;
pub const NOTE_EXTEND: u32 = 0x4;
pub const NOTE_ATTRIB: u32 = 0x8;
pub const NOTE_RENAME: u32 = 0x20;
pub const O_EVTONLY: i32 = 0x8000;

unsafe extern "C" {
    pub fn kqueue() -> i32;
    pub fn kevent(kq: i32, changes: *const KEvent, nchanges: i32, events: *mut KEvent, nevents: i32, timeout: *const [i64; 2]) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub fn CFFileDescriptorCreate(alloc: CFAllocatorRef, fd: i32, close_on_invalidate: bool, cb: CFFileDescriptorCallBack, ctx: *const c_void) -> CFFileDescriptorRef;
    pub fn CFFileDescriptorEnableCallBacks(f: CFFileDescriptorRef, types: usize);
    pub fn CFFileDescriptorCreateRunLoopSource(alloc: CFAllocatorRef, f: CFFileDescriptorRef, order: CFIndex) -> CFRunLoopSourceRef;
    pub fn CFFileDescriptorInvalidate(f: CFFileDescriptorRef);
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn CGDisplayRegisterReconfigurationCallback(cb: CGDisplayReconfigurationCallBack, user: *mut c_void) -> i32;
}
