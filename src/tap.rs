//! Event taps, each as narrow as its job. Only clicks pass through a filtering tap (double-click
//! title bar and click-for-menu can swallow them); other buttons join it only when a throw
//! button is set. Drags go to a listen-only tap, which never holds events up. Modifier and key
//! events are tapped only when a modifier feature is configured. Mouse-moved and scroll taps
//! are switched on only while something needs them (a throw, move/resize, stashed windows).
#![allow(non_upper_case_globals)]

use crate::app;
use crate::config::{Config, MOD_MASK};
use crate::ffi::*;
use crate::geom::Point;
use crate::overlay::Footprint;
use crate::snap::Drag;
use crate::throw::{MoveResize, Quick, Throw};
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr;

#[derive(Default)]
struct State {
    drag: Option<Drag>,
    throw: Option<Throw>,
    throw_by_button: bool,
    quick: Option<Quick>,
    mover: Option<MoveResize>,
    footprint: Option<Footprint>,
    flags: u64,
    swallow_up: bool,
    pending_click: Option<(Point, i64)>,
}

struct Taps {
    mouse: CFMachPortRef,
    drag: CFMachPortRef,
    keys: CFMachPortRef,
    moved: CFMachPortRef,
    moved_on: bool,
    scroll: CFMachPortRef,
    scroll_on: bool,
    sources: Vec<CFRunLoopSourceRef>,
}

thread_local! {
    static ST: RefCell<State> = RefCell::new(State::default());
    static TAPS: RefCell<Option<Taps>> = const { RefCell::new(None) };
}

fn mask(types: &[u32]) -> CGEventMask {
    types.iter().fold(0, |m, t| m | (1u64 << t))
}

pub fn cursor() -> Point {
    unsafe {
        let e = CGEventCreate(ptr::null());
        let p = CGEventGetLocation(e);
        CFRelease(e);
        Point::new(p.x, p.y)
    }
}

fn with_fp<R>(s: &mut State, f: impl FnOnce(&mut Footprint) -> R) -> R {
    f(s.footprint.get_or_insert_with(Footprint::new))
}

fn ours(e: CGEventRef) -> bool {
    unsafe { CGEventGetIntegerValueField(e, kCGEventSourceUserData) == OUR_EVENT_MARK }
}

fn reenable(ty: u32) -> bool {
    if ty == kCGEventTapDisabledByTimeout || ty == kCGEventTapDisabledByUserInput {
        TAPS.with(|t| {
            if let Some(t) = t.borrow().as_ref() {
                for (tap, on) in [(t.mouse, true), (t.drag, true), (t.keys, true), (t.moved, t.moved_on), (t.scroll, t.scroll_on)] {
                    if !tap.is_null() && on {
                        unsafe { CGEventTapEnable(tap, true) };
                    }
                }
            }
        });
        return true;
    }
    false
}

/// Turn the mouse-moved and scroll taps on or off to match what currently needs them.
pub fn update_moved() {
    let (throwing, moving) = ST.with(|s| {
        let s = s.borrow();
        (s.throw.is_some(), s.mover.is_some())
    });
    let (stash, scroll) = app::try_with(|a| (!a.stash.entries.is_empty(), a.cfg.throw.scroll_resizes)).unwrap_or((true, false));
    let need = throwing || moving || stash;
    let need_scroll = throwing && scroll;
    TAPS.with(|t| {
        if let Some(t) = t.borrow_mut().as_mut() {
            if t.moved_on != need && !t.moved.is_null() {
                unsafe { CGEventTapEnable(t.moved, need) };
                t.moved_on = need;
            }
            if t.scroll_on != need_scroll && !t.scroll.is_null() {
                unsafe { CGEventTapEnable(t.scroll, need_scroll) };
                t.scroll_on = need_scroll;
            }
        }
    });
}

pub fn note_hotkey() {
    ST.with(|s| {
        if let Some(q) = s.borrow_mut().quick.as_mut() {
            q.cancelled = true;
        }
    });
}

fn post_click(p: Point, button: i64) {
    unsafe {
        for ty in [kCGEventOtherMouseDown, kCGEventOtherMouseUp] {
            let e = CGEventCreateMouseEvent(ptr::null(), ty, CGPoint { x: p.x, y: p.y }, button as u32);
            CGEventSetIntegerValueField(e, kCGMouseEventButtonNumber, button);
            CGEventSetIntegerValueField(e, kCGEventSourceUserData, OUR_EVENT_MARK);
            CGEventPost(kCGHIDEventTap, e);
            CFRelease(e);
        }
    }
}

extern "C" fn mouse_cb(_p: CGEventTapProxy, ty: u32, e: CGEventRef, _u: *mut c_void) -> CGEventRef {
    if reenable(ty) || ours(e) {
        return e;
    }
    let loc = unsafe { CGEventGetLocation(e) };
    let p = Point::new(loc.x, loc.y);
    let flags = unsafe { CGEventGetFlags(e) };
    let mut swallow = false;
    ST.with(|st| {
        let mut st = st.borrow_mut();
        let s = &mut *st;
        app::try_with(|a| match ty {
            kCGEventLeftMouseDown => {
                if let Some(q) = s.quick.as_mut() {
                    q.cancelled = true;
                }
                let clicks = unsafe { CGEventGetIntegerValueField(e, kCGMouseEventClickState) };
                let cm = a.cfg.menu.click_modifiers;
                if cm != 0 && flags & MOD_MASK == cm {
                    let win = crate::ax::window_at(p);
                    crate::timer::after(0.0, move || crate::menu::popup_at_cursor(win));
                    swallow = true;
                    s.swallow_up = true;
                    return;
                }
                if clicks == 2 && a.cfg.general.double_click_title_bar && title_bar_double_click(a, p) {
                    swallow = true;
                    s.swallow_up = true;
                    return;
                }
                s.drag = Some(Drag::begin(p));
            }
            kCGEventLeftMouseUp => {
                if s.swallow_up {
                    s.swallow_up = false;
                    swallow = true;
                    return;
                }
                if let Some(d) = s.drag.take() {
                    let fp = s.footprint.get_or_insert_with(Footprint::new);
                    crate::snap::released(a, d, fp);
                }
            }
            kCGEventOtherMouseDown => {
                let b = unsafe { CGEventGetIntegerValueField(e, kCGMouseEventButtonNumber) };
                if a.cfg.throw.mouse_button == Some(b) && flags & MOD_MASK == 0 && s.throw.is_none() {
                    if let Some(t) = Throw::begin(a, p) {
                        s.throw = Some(t);
                        s.throw_by_button = true;
                        s.pending_click = Some((p, b));
                        swallow = true;
                    }
                }
            }
            kCGEventOtherMouseDragged => {
                if s.throw_by_button {
                    if let Some(t) = s.throw.as_mut() {
                        let fp = s.footprint.get_or_insert_with(Footprint::new);
                        t.update(a, p, fp);
                    }
                    swallow = true;
                }
            }
            kCGEventOtherMouseUp => {
                let b = unsafe { CGEventGetIntegerValueField(e, kCGMouseEventButtonNumber) };
                if s.throw_by_button && a.cfg.throw.mouse_button == Some(b) {
                    s.throw_by_button = false;
                    swallow = true;
                    if let Some(t) = s.throw.take() {
                        let fp = s.footprint.get_or_insert_with(Footprint::new);
                        if t.target.is_none() && t.max_dist < a.cfg.throw.min_dist_to_appear.max(4.0) {
                            // Never aimed: give the app its click back.
                            t.cancel(fp);
                            if let Some((cp, cb)) = s.pending_click.take() {
                                crate::timer::after(0.0, move || post_click(cp, cb));
                            }
                        } else {
                            t.finish(a, fp);
                        }
                    }
                }
            }
            _ => {}
        });
    });
    if matches!(ty, kCGEventOtherMouseDown | kCGEventOtherMouseUp) {
        crate::timer::after(0.0, update_moved);
    }
    if swallow { ptr::null() } else { e }
}

/// Listen-only: left drags, for drag-to-snap.
extern "C" fn drag_cb(_p: CGEventTapProxy, ty: u32, e: CGEventRef, _u: *mut c_void) -> CGEventRef {
    if reenable(ty) || ours(e) || ty != kCGEventLeftMouseDragged {
        return e;
    }
    let loc = unsafe { CGEventGetLocation(e) };
    let flags = unsafe { CGEventGetFlags(e) };
    ST.with(|st| {
        let mut st = st.borrow_mut();
        let s = &mut *st;
        if let Some(d) = s.drag.as_mut() {
            app::try_with(|a| {
                let fp = s.footprint.get_or_insert_with(Footprint::new);
                crate::snap::dragged(a, d, Point::new(loc.x, loc.y), flags, fp);
            });
        }
    });
    e
}

/// Filtering, only while a throw is aimed: scrolling resizes the footprint instead of the
/// window underneath.
extern "C" fn scroll_cb(_p: CGEventTapProxy, ty: u32, e: CGEventRef, _u: *mut c_void) -> CGEventRef {
    if reenable(ty) || ours(e) || ty != kCGEventScrollWheel {
        return e;
    }
    let loc = unsafe { CGEventGetLocation(e) };
    let d = unsafe { CGEventGetIntegerValueField(e, kCGScrollWheelEventPointDeltaAxis1) } as f64;
    let swallow = ST.with(|st| {
        let mut st = st.borrow_mut();
        let s = &mut *st;
        let Some(t) = s.throw.as_mut() else { return false };
        app::try_with(|a| {
            let fp = s.footprint.get_or_insert_with(Footprint::new);
            t.scroll(a, d, Point::new(loc.x, loc.y), fp);
        });
        true
    });
    if swallow { ptr::null() } else { e }
}

fn title_bar_double_click(a: &mut app::App, p: Point) -> bool {
    let Some(win) = crate::ax::window_at(p) else { return false };
    let Some(f) = win.frame() else { return false };
    if p.y - f.y > 32.0 || !win.is_standard() {
        return false;
    }
    let role = crate::ax::role_at(p);
    if !matches!(role.as_str(), "AXWindow" | "AXToolbar" | "AXGroup" | "AXStaticText" | "AXTabGroup") {
        return false;
    }
    let bundle = crate::engine::bundle_id(win.pid);
    if a.cfg.tweaks.double_click_ignored_apps.iter().any(|x| *x == bundle) {
        return false;
    }
    let idx = crate::screens::index_for(&a.screens, &f);
    let max = crate::engine::compute(a, "maximize", f, win.id(), idx, 0, None);
    let is_max = max.map(|m| m.approx_eq(&f, 4.0)).unwrap_or(false);
    let cmd = if is_max && a.cfg.tweaks.double_click_restore { "restore" } else { "maximize" };
    crate::timer::after(0.0, move || app::with(|a| crate::engine::run(a, cmd, crate::engine::Target::Window(win))));
    true
}

extern "C" fn keys_cb(_p: CGEventTapProxy, ty: u32, e: CGEventRef, _u: *mut c_void) -> CGEventRef {
    if reenable(ty) || ours(e) {
        return e;
    }
    if ty == kCGEventKeyDown {
        // Some input tools send key-downs for modifier keys themselves (codes 54-63); those
        // are part of pressing the modifier, not a shortcut.
        let code = unsafe { CGEventGetIntegerValueField(e, kCGKeyboardEventKeycode) };
        if (54..=63).contains(&code) {
            return e;
        }
        ST.with(|st| {
            let mut s = st.borrow_mut();
            if let Some(q) = s.quick.as_mut() {
                q.cancelled = true;
            }
            // A key pressed with the throw modifiers down is a shortcut, not a throw.
            if !s.throw_by_button {
                if let Some(t) = s.throw.take() {
                    with_fp(&mut s, |fp| t.cancel(fp));
                }
            }
        });
        return e;
    }
    if ty != kCGEventFlagsChanged {
        return e;
    }
    let flags = unsafe { CGEventGetFlags(e) } & MOD_MASK;
    // Keyboard events don't reliably carry the cursor position; ask for it.
    let p = cursor();
    ST.with(|st| {
        let mut st = st.borrow_mut();
        let s = &mut *st;
        let prev = s.flags;
        s.flags = flags;
        app::try_with(|a| {
            if flags == 0 {
                a.released_since_last = true;
            }
            // Window Throw by modifiers.
            let tm = a.cfg.throw.modifiers;
            if !s.throw_by_button {
                if tm != 0 && flags == tm && s.throw.is_none() {
                    s.throw = Throw::begin(a, p);
                    if let Some(t) = s.throw.as_mut() {
                        let fp = s.footprint.get_or_insert_with(Footprint::new);
                        t.update(a, p, fp);
                    }
                } else if s.throw.is_some() && flags != tm {
                    let t = s.throw.take().unwrap();
                    let fp = s.footprint.get_or_insert_with(Footprint::new);
                    // Releasing finishes; adding another modifier cancels.
                    if flags & !tm == 0 { t.finish(a, fp) } else { t.cancel(fp) }
                }
            }
            // Move & Resize.
            let (mm, rm) = (a.cfg.move_resize.move_modifiers, a.cfg.move_resize.resize_modifiers);
            if s.mover.is_none() && flags != 0 && (flags == mm || flags == rm) {
                s.mover = MoveResize::begin(p, flags == rm);
            } else if s.mover.is_some() && flags != mm && flags != rm {
                s.mover = None;
            }
            // Quick Throw: armed on press, fired on release.
            if prev == 0 && flags != 0 {
                s.quick = Quick::arm(a, flags, p);
            } else if flags == 0 {
                if let Some(q) = s.quick.take() {
                    q.release(a, p);
                }
            } else if s.quick.is_some() && prev != 0 && flags != prev {
                s.quick = None;
            }
        });
    });
    crate::timer::after(0.0, update_moved);
    e
}

extern "C" fn moved_cb(_p: CGEventTapProxy, ty: u32, e: CGEventRef, _u: *mut c_void) -> CGEventRef {
    if reenable(ty) || ours(e) {
        return e;
    }
    let l = unsafe { CGEventGetLocation(e) };
    let p = Point::new(l.x, l.y);
    let flags = unsafe { CGEventGetFlags(e) };
    ST.with(|st| {
        let mut st = st.borrow_mut();
        let s = &mut *st;
        app::try_with(|a| {
            if let Some(t) = s.throw.as_mut() {
                let fp = s.footprint.get_or_insert_with(Footprint::new);
                t.update(a, p, fp);
            }
            if let Some(m) = s.mover.as_mut() {
                m.update(p);
            }
            if !a.stash.entries.is_empty() {
                crate::stash::on_mouse(a, p, flags);
            }
        });
    });
    e
}

fn make_tap(m: CGEventMask, opt: u32, cb: CGEventTapCallBack) -> (CFMachPortRef, Option<CFRunLoopSourceRef>) {
    unsafe {
        let tap = CGEventTapCreate(kCGSessionEventTap, kCGHeadInsertEventTap, opt, m, cb, ptr::null_mut());
        if tap.is_null() {
            return (ptr::null(), None);
        }
        let src = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0);
        CFRunLoopAddSource(CFRunLoopGetMain(), src, kCFRunLoopCommonModes);
        (tap, Some(src))
    }
}

fn needs_keys(cfg: &Config) -> bool {
    cfg.throw.modifiers != 0
        || !cfg.quick_throw.is_empty()
        || cfg.move_resize.move_modifiers != 0
        || cfg.move_resize.resize_modifiers != 0
        || cfg.general.reset_repeat_on_release
}

/// (Re)create taps for the current configuration. Returns false without Accessibility trust.
pub fn install(cfg: &Config) -> bool {
    uninstall();
    let mut mouse_mask = mask(&[kCGEventLeftMouseDown, kCGEventLeftMouseUp]);
    if cfg.throw.mouse_button.is_some() {
        mouse_mask |= mask(&[kCGEventOtherMouseDown, kCGEventOtherMouseDragged, kCGEventOtherMouseUp]);
    }
    // A filtering tap holds every click until it returns; only pay for that when a feature
    // may swallow one.
    let filters = cfg.menu.click_modifiers != 0 || cfg.general.double_click_title_bar || cfg.throw.mouse_button.is_some();
    let opt = if filters { kCGEventTapOptionDefault } else { kCGEventTapOptionListenOnly };
    let (mouse, s1) = make_tap(mouse_mask, opt, mouse_cb);
    if mouse.is_null() {
        return false;
    }
    let mut sources = vec![s1.unwrap()];
    let (drag, s2) = make_tap(mask(&[kCGEventLeftMouseDragged]), kCGEventTapOptionListenOnly, drag_cb);
    sources.extend(s2);
    let keys = if needs_keys(cfg) {
        // Never swallows, so listen-only: typing is never held up.
        let (k, s) = make_tap(mask(&[kCGEventFlagsChanged, kCGEventKeyDown]), kCGEventTapOptionListenOnly, keys_cb);
        sources.extend(s);
        k
    } else {
        ptr::null()
    };
    let (moved, s3) = make_tap(mask(&[kCGEventMouseMoved, kCGEventLeftMouseDragged]), kCGEventTapOptionListenOnly, moved_cb);
    sources.extend(s3);
    let (scroll, s4) = make_tap(mask(&[kCGEventScrollWheel]), kCGEventTapOptionDefault, scroll_cb);
    sources.extend(s4);
    for tap in [moved, scroll] {
        if !tap.is_null() {
            unsafe { CGEventTapEnable(tap, false) };
        }
    }
    TAPS.with(|t| *t.borrow_mut() = Some(Taps { mouse, drag, keys, moved, moved_on: false, scroll, scroll_on: false, sources }));
    update_moved();
    true
}

pub fn uninstall() {
    TAPS.with(|t| {
        if let Some(t) = t.borrow_mut().take() {
            unsafe {
                for src in t.sources {
                    CFRunLoopRemoveSource(CFRunLoopGetMain(), src, kCFRunLoopCommonModes);
                    CFRelease(src);
                }
                for tap in [t.mouse, t.drag, t.keys, t.moved, t.scroll] {
                    if !tap.is_null() {
                        CFMachPortInvalidate(tap);
                        CFRelease(tap);
                    }
                }
            }
        }
    });
}

/// Trackpad gesture hooks (from the multitouch thread, already hopped to main).
pub fn gesture_throw(begin: bool) {
    let p = cursor();
    ST.with(|st| {
        let mut st = st.borrow_mut();
        let s = &mut *st;
        app::try_with(|a| {
            if begin {
                if s.throw.is_none() {
                    s.throw = Throw::begin(a, p);
                    s.throw_by_button = true; // keep modifier changes from ending it
                }
            } else if let Some(t) = s.throw.take() {
                s.throw_by_button = false;
                let fp = s.footprint.get_or_insert_with(Footprint::new);
                t.finish(a, fp);
            }
        });
    });
    update_moved();
}
