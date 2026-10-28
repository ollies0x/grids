//! Accessibility windows: find them, read them, move them.

use crate::ffi::*;
use crate::geom::{Point, Rect};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;

thread_local! {
    static ATTRS: RefCell<HashMap<&'static str, Cf>> = RefCell::new(HashMap::new());
}

/// Interned CFString for an attribute or action name.
pub fn attr(name: &'static str) -> CFStringRef {
    ATTRS.with(|m| m.borrow_mut().entry(name).or_insert_with(|| cfstr(name)).ptr())
}

pub fn trusted(prompt: bool) -> bool {
    unsafe {
        if !prompt {
            return AXIsProcessTrusted() != 0;
        }
        use objc2_foundation::{NSDictionary, NSNumber, NSString};
        let key = NSString::from_str("AXTrustedCheckOptionPrompt");
        let val = NSNumber::new_bool(true);
        let dict = NSDictionary::from_slices(&[&*key], &[&*val]);
        AXIsProcessTrustedWithOptions(&*dict as *const _ as CFDictionaryRef) != 0
    }
}

fn copy(el: CFTypeRef, name: &'static str) -> Option<Cf> {
    let mut out: CFTypeRef = ptr::null();
    let err = unsafe { AXUIElementCopyAttributeValue(el, attr(name), &mut out) };
    if err == kAXErrorSuccess { Cf::own(out) } else { None }
}

fn set(el: CFTypeRef, name: &'static str, v: CFTypeRef) -> bool {
    unsafe { AXUIElementSetAttributeValue(el, attr(name), v) == kAXErrorSuccess }
}

fn point_value(p: Point) -> Cf {
    let cg = CGPoint { x: p.x, y: p.y };
    Cf(unsafe { AXValueCreate(kAXValueCGPointType, &cg as *const _ as *const c_void) })
}

fn size_value(w: f64, h: f64) -> Cf {
    let cg = CGSize { width: w, height: h };
    Cf(unsafe { AXValueCreate(kAXValueCGSizeType, &cg as *const _ as *const c_void) })
}

pub fn app_element(pid: i32) -> Cf {
    let el = Cf(unsafe { AXUIElementCreateApplication(pid) });
    unsafe { AXUIElementSetMessagingTimeout(el.ptr(), 0.5) };
    el
}

thread_local! {
    static SYSTEM: Cf = {
        let el = Cf(unsafe { AXUIElementCreateSystemWide() });
        unsafe { AXUIElementSetMessagingTimeout(el.ptr(), 0.5) };
        el
    };
}

#[derive(Clone)]
pub struct Window {
    pub el: Cf,
    pub pid: i32,
}

impl PartialEq for Window {
    fn eq(&self, o: &Window) -> bool {
        self.pid == o.pid && self.id() == o.id()
    }
}

impl Window {
    pub fn from_element(el: Cf) -> Option<Window> {
        let mut pid = 0;
        if unsafe { AXUIElementGetPid(el.ptr(), &mut pid) } != kAXErrorSuccess {
            return None;
        }
        Some(Window { el, pid })
    }

    /// The CGWindowID, which is stable for the window's life (0 if unavailable).
    pub fn id(&self) -> u32 {
        let mut id = 0u32;
        unsafe { _AXUIElementGetWindow(self.el.ptr(), &mut id) };
        id
    }

    pub fn string(&self, name: &'static str) -> Option<String> {
        copy(self.el.ptr(), name).and_then(|v| cf_to_string(v.ptr()))
    }
    pub fn title(&self) -> String {
        self.string("AXTitle").unwrap_or_default()
    }
    pub fn role(&self) -> String {
        self.string("AXRole").unwrap_or_default()
    }
    pub fn subrole(&self) -> String {
        self.string("AXSubrole").unwrap_or_default()
    }
    fn flag(&self, name: &'static str) -> bool {
        copy(self.el.ptr(), name).and_then(|v| cf_to_bool(v.ptr())).unwrap_or(false)
    }
    pub fn minimized(&self) -> bool {
        self.flag("AXMinimized")
    }
    pub fn is_fullscreen(&self) -> bool {
        self.flag("AXFullScreen")
    }

    /// Normal, movable application windows (not sheets, panels, popovers or the desktop).
    pub fn is_standard(&self) -> bool {
        self.role() == "AXWindow" && matches!(self.subrole().as_str(), "AXStandardWindow" | "")
    }

    pub fn has_toolbar(&self) -> bool {
        let Some(kids) = copy(self.el.ptr(), "AXChildren") else { return false };
        let n = unsafe { CFArrayGetCount(kids.ptr()) };
        (0..n).any(|i| {
            let k = unsafe { CFArrayGetValueAtIndex(kids.ptr(), i) };
            copy(k, "AXRole").and_then(|r| cf_to_string(r.ptr())).as_deref() == Some("AXToolbar")
        })
    }

    pub fn frame(&self) -> Option<Rect> {
        let pos = copy(self.el.ptr(), "AXPosition")?;
        let size = copy(self.el.ptr(), "AXSize")?;
        let mut p = CGPoint::default();
        let mut s = CGSize::default();
        unsafe {
            AXValueGetValue(pos.ptr(), kAXValueCGPointType, &mut p as *mut _ as *mut c_void);
            AXValueGetValue(size.ptr(), kAXValueCGSizeType, &mut s as *mut _ as *mut c_void);
        }
        Some(Rect::new(p.x, p.y, s.width, s.height))
    }

    pub fn set_position(&self, p: Point) -> bool {
        set(self.el.ptr(), "AXPosition", point_value(p).ptr())
    }

    pub fn set_size(&self, w: f64, h: f64) -> bool {
        set(self.el.ptr(), "AXSize", size_value(w, h).ptr())
    }

    /// Size, position, size: the first size keeps the window from being clamped when it
    /// changes display, the last one applies the size the new display allows. Apps with
    /// AXEnhancedUserInterface on (set by VoiceOver-adjacent tools) animate every change,
    /// so it is switched off for the move, which is most of Grids's speed edge.
    pub fn set_frame(&self, r: Rect) {
        let r = r.round();
        let app = app_element(self.pid);
        let eui = copy(app.ptr(), "AXEnhancedUserInterface").and_then(|v| cf_to_bool(v.ptr())).unwrap_or(false);
        if eui {
            set(app.ptr(), "AXEnhancedUserInterface", cf_bool(false));
        }
        self.set_size(r.w, r.h);
        self.set_position(Point::new(r.x, r.y));
        self.set_size(r.w, r.h);
        if let Some(now) = self.frame() {
            if (now.x - r.x).abs() > 1.0 || (now.y - r.y).abs() > 1.0 {
                self.set_position(Point::new(r.x, r.y));
            }
        }
        if eui {
            set(app.ptr(), "AXEnhancedUserInterface", cf_bool(true));
        }
    }

    pub fn perform(&self, action: &'static str) -> bool {
        unsafe { AXUIElementPerformAction(self.el.ptr(), attr(action)) == kAXErrorSuccess }
    }

    pub fn raise(&self) {
        self.perform("AXRaise");
        let app = app_element(self.pid);
        set(app.ptr(), "AXFrontmost", cf_bool(true));
        set(self.el.ptr(), "AXMain", cf_bool(true));
    }

    pub fn set_minimized(&self, v: bool) {
        set(self.el.ptr(), "AXMinimized", cf_bool(v));
    }

    pub fn toggle_fullscreen(&self) {
        let v = self.is_fullscreen();
        set(self.el.ptr(), "AXFullScreen", cf_bool(!v));
    }

    pub fn close(&self) {
        if let Some(btn) = copy(self.el.ptr(), "AXCloseButton") {
            unsafe { AXUIElementPerformAction(btn.ptr(), attr("AXPress")) };
        }
    }
}

/// The focused window of the frontmost app.
pub fn focused_window() -> Option<Window> {
    let mut pid = 0;
    if let Some(app) = SYSTEM.with(|s| copy(s.ptr(), "AXFocusedApplication")) {
        unsafe { AXUIElementGetPid(app.ptr(), &mut pid) };
        let w = copy(app.ptr(), "AXFocusedWindow").or_else(|| copy(app.ptr(), "AXMainWindow"));
        if let Some(w) = w.and_then(Window::from_element) {
            if w.is_standard() {
                return Some(w);
            }
        }
    }
    // Chromium (Brave, Chrome) fails both AXFocusedApplication and AXMainWindow. Fall back to
    // the topmost on-screen window of the frontmost app, which is the one being looked at.
    if pid == 0 {
        pid = crate::engine::frontmost_pid()?;
    }
    let wins: Vec<Window> = onscreen_windows().into_iter().filter(|i| i.pid == pid).filter_map(|i| window_by_id(pid, i.id)).collect();
    wins.iter().find(|w| w.is_standard()).or(wins.first()).cloned()
}

/// The window under a screen point: the window server's topmost window there (so floating
/// windows win, as they look), mapped to its Accessibility element. Falls back to an AX hit
/// test for windows the id lookup can't resolve.
pub fn window_at(p: Point) -> Option<Window> {
    if let Some((pid, id)) = top_window_at(p) {
        if let Some(w) = window_by_id(pid, id) {
            return Some(w);
        }
    }
    ax_window_at(p)
}

/// Topmost visible window (below the menu bar and Dock layers) containing `p`, not ours.
fn top_window_at(p: Point) -> Option<(i32, u32)> {
    let own = std::process::id() as i32;
    let list = Cf::own(unsafe { CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, 0) })?;
    let n = unsafe { CFArrayGetCount(list.ptr()) };
    for i in 0..n {
        let d = unsafe { CFArrayGetValueAtIndex(list.ptr(), i) };
        let layer = cf_to_f64(dict_get(d, "kCGWindowLayer")).unwrap_or(99.0);
        let alpha = cf_to_f64(dict_get(d, "kCGWindowAlpha")).unwrap_or(1.0);
        let pid = cf_to_f64(dict_get(d, "kCGWindowOwnerPID")).unwrap_or(0.0) as i32;
        if layer >= 20.0 || alpha <= 0.0 || pid == own {
            continue;
        }
        let mut cg = CGRect::default();
        let b = dict_get(d, "kCGWindowBounds");
        if b.is_null() || !unsafe { CGRectMakeWithDictionaryRepresentation(b, &mut cg) } {
            continue;
        }
        if Rect::new(cg.origin.x, cg.origin.y, cg.size.width, cg.size.height).contains(p) {
            return Some((pid, cf_to_f64(dict_get(d, "kCGWindowNumber")).unwrap_or(0.0) as u32));
        }
    }
    None
}

fn ax_window_at(p: Point) -> Option<Window> {
    let mut el: AXUIElementRef = ptr::null();
    let err = SYSTEM.with(|s| unsafe { AXUIElementCopyElementAtPosition(s.ptr(), p.x as f32, p.y as f32, &mut el) });
    if err != kAXErrorSuccess {
        return None;
    }
    let el = Cf::own(el)?;
    let role = copy(el.ptr(), "AXRole").and_then(|r| cf_to_string(r.ptr())).unwrap_or_default();
    if role == "AXWindow" {
        return Window::from_element(el);
    }
    if let Some(w) = copy(el.ptr(), "AXWindow") {
        return Window::from_element(w);
    }
    let mut cur = el;
    for _ in 0..32 {
        let parent = copy(cur.ptr(), "AXParent")?;
        let r = copy(parent.ptr(), "AXRole").and_then(|r| cf_to_string(r.ptr())).unwrap_or_default();
        if r == "AXWindow" {
            return Window::from_element(parent);
        }
        cur = parent;
    }
    None
}

/// Role of the element under a point (used to tell a title bar from its buttons).
pub fn role_at(p: Point) -> String {
    let mut el: AXUIElementRef = ptr::null();
    let err = SYSTEM.with(|s| unsafe { AXUIElementCopyElementAtPosition(s.ptr(), p.x as f32, p.y as f32, &mut el) });
    if err != kAXErrorSuccess {
        return String::new();
    }
    Cf::own(el).and_then(|e| copy(e.ptr(), "AXRole")).and_then(|r| cf_to_string(r.ptr())).unwrap_or_default()
}

pub fn app_windows(pid: i32) -> Vec<Window> {
    let app = app_element(pid);
    let Some(list) = copy(app.ptr(), "AXWindows") else { return vec![] };
    let n = unsafe { CFArrayGetCount(list.ptr()) };
    (0..n)
        .filter_map(|i| Cf::retain(unsafe { CFArrayGetValueAtIndex(list.ptr(), i) }))
        .map(|el| Window { el, pid })
        .collect()
}

pub fn window_by_id(pid: i32, id: u32) -> Option<Window> {
    app_windows(pid).into_iter().find(|w| w.id() == id)
}

/// On-screen normal windows from the window server, front to back. Cheap: one call, no AX.
#[derive(Clone, Debug)]
pub struct WinInfo {
    pub id: u32,
    pub pid: i32,
    pub bounds: Rect,
    pub owner: String,
}

/// The owning pid of a window id, at any layer (floating panels included).
pub fn owner_of(id: u32) -> Option<i32> {
    let list = unsafe { CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, 0) };
    let list = Cf::own(list)?;
    let n = unsafe { CFArrayGetCount(list.ptr()) };
    (0..n).find_map(|i| {
        let d = unsafe { CFArrayGetValueAtIndex(list.ptr(), i) };
        (cf_to_f64(dict_get(d, "kCGWindowNumber")) == Some(id as f64)).then(|| cf_to_f64(dict_get(d, "kCGWindowOwnerPID")).unwrap_or(0.0) as i32)
    })
}

pub fn onscreen_windows() -> Vec<WinInfo> {
    let own = std::process::id() as i32;
    let list = unsafe { CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, 0) };
    let Some(list) = Cf::own(list) else { return vec![] };
    let n = unsafe { CFArrayGetCount(list.ptr()) };
    let mut out = Vec::new();
    for i in 0..n {
        let d = unsafe { CFArrayGetValueAtIndex(list.ptr(), i) };
        let layer = cf_to_f64(dict_get(d, "kCGWindowLayer")).unwrap_or(1.0);
        let alpha = cf_to_f64(dict_get(d, "kCGWindowAlpha")).unwrap_or(1.0);
        let pid = cf_to_f64(dict_get(d, "kCGWindowOwnerPID")).unwrap_or(0.0) as i32;
        if layer != 0.0 || alpha <= 0.0 || pid == own {
            continue;
        }
        let mut cg = CGRect::default();
        let b = dict_get(d, "kCGWindowBounds");
        if b.is_null() || !unsafe { CGRectMakeWithDictionaryRepresentation(b, &mut cg) } {
            continue;
        }
        let bounds = Rect::new(cg.origin.x, cg.origin.y, cg.size.width, cg.size.height);
        if bounds.w < 60.0 || bounds.h < 40.0 {
            continue;
        }
        out.push(WinInfo {
            id: cf_to_f64(dict_get(d, "kCGWindowNumber")).unwrap_or(0.0) as u32,
            pid,
            bounds,
            owner: cf_to_string(dict_get(d, "kCGWindowOwnerName")).unwrap_or_default(),
        });
    }
    out
}
