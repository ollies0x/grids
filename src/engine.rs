//! Turns a command into window moves: picks the window and screen, works out repeats, applies
//! gaps, remembers history, and hands multi-window, stash, pin and layout commands on.

use crate::actions::{self, Ctx, Gaps};
use crate::app::{App, Last, WinHist};
use crate::ax::{self, Window};
use crate::config::{parse_frac, Placement, Step};
use crate::ffi::*;
use crate::geom::{Point, Rect};
use crate::screens;
use objc2_app_kit::{NSRunningApplication, NSWorkspace};

#[derive(Clone)]
pub enum Target {
    Front,
    Window(Window),
}

impl Target {
    pub fn resolve(self) -> Option<Window> {
        match self {
            Target::Front => ax::focused_window(),
            Target::Window(w) => Some(w),
        }
    }
}

pub fn running_app(pid: i32) -> Option<objc2::rc::Retained<NSRunningApplication>> {
    NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
}

pub fn bundle_id(pid: i32) -> String {
    running_app(pid).and_then(|a| a.bundleIdentifier()).map(|s| s.to_string()).unwrap_or_default()
}

pub fn frontmost_pid() -> Option<i32> {
    NSWorkspace::sharedWorkspace().frontmostApplication().map(|a| a.processIdentifier())
}

fn ignored(a: &App, pid: i32) -> bool {
    if a.cfg.general.ignored_apps.is_empty() {
        return false;
    }
    let id = bundle_id(pid);
    a.cfg.general.ignored_apps.iter().any(|x| *x == id)
}

/// Entry point for every command, from any trigger.
pub fn run(a: &mut App, cmd: &str, target: Target) {
    if let Some(name) = cmd.strip_prefix("custom:") {
        return run_custom(a, name, target);
    }
    if let Some(name) = cmd.strip_prefix("layout:") {
        return crate::layouts::apply_named(a, name);
    }
    let cmd = actions::resolve(cmd);
    let cmd = cmd.as_str();
    if cmd.is_empty() || cmd == "none" {
        return;
    }
    match cmd {
        "open-settings" => return crate::settings::open(None),
        "focus-window-under-cursor" => {
            if let Some(w) = ax::window_at(crate::tap::cursor()) {
                w.raise();
            }
            return;
        }
        "center-on-cursor-display" => {
            let Some(win) = target.resolve() else { return };
            let Some(f) = win.frame() else { return };
            a.refresh_screens();
            if let Some(dest) = screens::index_at(&a.screens, crate::tap::cursor()) {
                if let Some(to) = compute(a, "center", f, win.id(), dest, 0, None) {
                    apply(a, &win, f, to, "center");
                }
            }
            return;
        }

        "pin" => return crate::pin::toggle(a),
        "pin-left" => return crate::pin::toggle_side(a, "left"),
        "pin-right" => return crate::pin::toggle_side(a, "right"),
        "reflow-pin" => return crate::pin::reflow(a),
        "save-layout" => return crate::layouts::save_current(a),
        "show-menu" => {
            crate::timer::after(0.0, || crate::menu::popup_at_cursor(None));
            return;
        }
        c if c.starts_with("win-") => return win_arrow(a, &c[4..], target),
        c if c.starts_with("stash") || c.starts_with("unstash") || c == "cycle-stashed" || c == "toggle-stashed" => {
            return crate::stash::command(a, c, target)
        }
        c if crate::actions::find(c).map(|x| x.cat) == Some("Multiple Windows") => return crate::multi::command(a, c, target),
        _ => {}
    }
    let Some(win) = target.resolve() else { return };
    if ignored(a, win.pid) {
        return;
    }
    match cmd {
        "fullscreen" => win.toggle_fullscreen(),
        "close" => win.close(),
        "minimize" => win.set_minimized(true),
        "quit-app" => {
            if let Some(app) = running_app(win.pid) {
                app.terminate();
            }
        }
        "hide-app" => {
            if let Some(app) = running_app(win.pid) {
                app.hide();
            }
        }
        "restore" => restore(a, &win),
        "restore-to-screen" => {
            let Some(f) = win.frame() else { return };
            if let Some(orig) = a.hist.get(&win.id()).and_then(|h| h.original) {
                let dest = screens::index_for(&a.screens, &orig);
                if dest != screens::index_for(&a.screens, &f) {
                    let area = screens::usable(&a.screens[dest], &a.cfg, None);
                    let r = Rect::new(area.mid_x() - f.w / 2.0, area.mid_y() - f.h / 2.0, f.w.min(area.w), f.h.min(area.h));
                    win.set_frame(r.clamped_into(&area));
                }
            }
        }
        "last" => last(a, &win),
        "next-space" | "prev-space" => switch_space(a, &win, cmd == "next-space"),
        "next-display" | "previous-display" | "next-display-ratio" | "prev-display-ratio" => {
            let dir = if cmd.starts_with("next") { 1 } else { -1 };
            move_display(a, &win, dir, cmd.ends_with("ratio"));
        }
        "next-display-max" | "prev-display-max" => {
            let dir = if cmd.starts_with("next") { 1 } else { -1 };
            move_display(a, &win, dir, false);
            if let Some(f) = win.frame() {
                let idx = screens::index_for(&a.screens, &f);
                if let Some(to) = compute(a, "maximize", f, win.id(), idx, 0, None) {
                    apply(a, &win, f, to, "maximize");
                }
            }
        }
        _ if actions::find(cmd).is_some() => geometry(a, cmd, &win),
        _ => eprintln!("grids: unknown command {cmd}"),
    }
}

/// Repeat count for `cmd` on `win`: consecutive runs on the same window, with the window
/// still where the last run put it (sizes get slack for apps that snap to a cell grid).
fn repeat_count(a: &App, cmd: &str, wid: u32, frame: &Rect) -> usize {
    let reset = a.cfg.general.reset_repeat_on_release && a.released_since_last;
    match &a.last {
        Some(l) if !reset && l.cmd == cmd && l.wid == wid && near(&l.rect, frame) => l.count + 1,
        _ => 0,
    }
}

fn near(a: &Rect, b: &Rect) -> bool {
    (a.x - b.x).abs() < 4.0 && (a.y - b.y).abs() < 4.0 && (a.w - b.w).abs() < 30.0 && (a.h - b.h).abs() < 30.0
}

fn self_cycling(id: &str) -> bool {
    let cat = actions::find(id).map(|a| a.cat).unwrap_or("");
    matches!(cat, "Thirds" | "Additional Thirds" | "Corner Two Thirds" | "Fourths" | "Fifths" | "Sixths" | "Eighths" | "Ninths" | "Fill")
        || (id == "center-half")
        || (id.starts_with("move-"))
}

fn cycle_frac(a: &App, count: usize) -> Option<f64> {
    // First press is a half, repeats walk the other selected sizes in their listed order.
    let mut sizes: Vec<f64> = a.cfg.general.cycle_sizes.iter().filter_map(|s| parse_frac(s)).filter(|f| (*f - 0.5).abs() > 1e-6).collect();
    if sizes.is_empty() {
        return None;
    }
    sizes.insert(0, 0.5);
    Some(sizes[count % sizes.len()])
}

/// Other windows on a screen, for Fill/Snap.
pub fn others_on(area: &Rect, exclude: u32) -> Vec<Rect> {
    ax::onscreen_windows().into_iter().filter(|w| w.id != exclude && w.bounds.intersects(area)).map(|w| w.bounds).collect()
}

pub fn pin_rect(a: &App) -> Option<Rect> {
    a.pin.rect
}

/// Compute where `id` puts a window with `frame` on screen `dest`.
pub fn compute(a: &App, id: &str, frame: Rect, wid: u32, dest: usize, repeat: usize, frac: Option<f64>) -> Option<Rect> {
    let s = a.screens.get(dest)?;
    let area = screens::usable(s, &a.cfg, pin_rect(a).as_ref());
    let needs_others = id.starts_with("fill-") || id.starts_with("snap-");
    let others = if needs_others { others_on(&area, wid) } else { vec![] };
    // A window arriving from another screen is measured against the new screen.
    let win = if screens::index_for(&a.screens, &frame) == dest { frame } else { frame.centered_in(&area) };
    let ctx = Ctx { win, area, repeat, frac, portrait: s.portrait(), others: &others, t: &a.cfg.tweaks };
    let out = actions::calc(id, &ctx)?;
    let g = &a.cfg.general;
    let gap_on = match out.gaps {
        Gaps::Tile => true,
        Gaps::Maximize => a.cfg.tweaks.apply_gaps_to_maximize,
        Gaps::MaximizeHeight => a.cfg.tweaks.apply_gaps_to_maximize_height,
        Gaps::None => false,
    };
    Some(if gap_on { actions::apply_gaps(out.rect, &area, g.gap, g.gaps_on_edges) } else { out.rect })
}

fn geometry(a: &mut App, id: &str, win: &Window) {
    let Some(frame) = win.frame() else { return };
    let wid = win.id();
    a.refresh_screens();
    let cur = screens::index_for(&a.screens, &frame);
    let count = repeat_count(a, id, wid, &frame);
    let mode = a.cfg.general.repeat_mode.clone();
    let mut dest = cur;
    let mut act = id.to_string();
    let mut frac = None;
    let mut repeat = count;

    if actions::half_based(id) || actions::side(id) != 0 || !self_cycling(id) {
        match mode.as_str() {
            "cycleSizes" if actions::half_based(id) => {
                frac = cycle_frac(a, count);
                repeat = 0;
            }
            "cycleDisplays" if count > 0 && !self_cycling(id) => {
                dest = (cur + 1) % a.screens.len().max(1);
                repeat = 0;
            }
            "adjacentDisplay" | "adjacentOrCycle" if actions::side(id) != 0 => {
                // Already there? Carry on across to the neighbouring display.
                let here = compute(a, id, frame, wid, cur, 0, None);
                if here.map(|r| near(&r, &frame)).unwrap_or(false) {
                    if let Some(adj) = screens::adjacent(&a.screens, cur, actions::side(id)) {
                        dest = adj;
                        act = actions::mirror(id).to_string();
                    } else if mode == "adjacentOrCycle" && actions::half_based(id) {
                        frac = cycle_frac(a, count);
                    }
                }
                repeat = 0;
            }
            _ => {}
        }
    }

    let Some(target) = compute(a, &act, frame, wid, dest, repeat, frac) else { return };
    crate::stash::forget_if_acted(a, wid);
    apply(a, win, frame, target, id);
    a.last = Some(Last { cmd: id.to_string(), wid, rect: win.frame().unwrap_or(target), count });
    a.released_since_last = false;
    if dest != cur && a.cfg.general.move_cursor_across_displays || a.cfg.general.move_cursor_always {
        warp(target.center());
    }
}

/// Move a window and keep its history.
pub fn apply(a: &mut App, win: &Window, from: Rect, to: Rect, cmd: &str) {
    let wid = win.id();
    let h = a.hist.entry(wid).or_insert_with(WinHist::default);
    if h.original.is_none() {
        h.original = Some(from);
    }
    h.stack.push(from);
    if h.stack.len() > 16 {
        h.stack.remove(0);
    }
    h.cmd = cmd.to_string();
    h.rect = to;
    win.set_frame(to);
    if let Some(now) = win.frame() {
        a.hist.get_mut(&wid).unwrap().rect = now;
    }
}

pub fn warp(p: Point) {
    unsafe {
        CGWarpMouseCursorPosition(CGPoint { x: p.x, y: p.y });
        CGAssociateMouseAndMouseCursorPosition(true);
    }
}

fn restore(a: &mut App, win: &Window) {
    let wid = win.id();
    let Some(orig) = a.hist.get(&wid).and_then(|h| h.original) else { return };
    let cur = win.frame().unwrap_or(orig);
    // Keep it on the screen it is on now, at its original size.
    a.refresh_screens();
    let s = &a.screens[screens::index_for(&a.screens, &cur)];
    let to = if s.frame.intersects(&orig) { orig } else { Rect::new(cur.x, cur.y, orig.w, orig.h).clamped_into(&s.visible) };
    win.set_frame(to);
    a.hist.remove(&wid);
    a.last = None;
}

fn last(a: &mut App, win: &Window) {
    let wid = win.id();
    let Some(prev) = a.hist.get_mut(&wid).and_then(|h| h.stack.pop()) else { return };
    win.set_frame(prev);
    if let Some(h) = a.hist.get_mut(&wid) {
        h.rect = prev;
        h.cmd.clear();
    }
    a.last = None;
}

pub fn move_display(a: &mut App, win: &Window, dir: i32, ratio: bool) {
    let Some(frame) = win.frame() else { return };
    a.refresh_screens();
    let n = a.screens.len();
    if n < 2 {
        return;
    }
    let cur = screens::index_for(&a.screens, &frame);
    let dest = ((cur as i32 + dir).rem_euclid(n as i32)) as usize;
    let wid = win.id();
    let pin = pin_rect(a);
    let from_area = screens::usable(&a.screens[cur], &a.cfg, pin.as_ref());
    let to_area = screens::usable(&a.screens[dest], &a.cfg, pin.as_ref());
    // A window still sitting where an action put it gets that action again on the new screen.
    let replay = a.hist.get(&wid).filter(|h| !h.cmd.is_empty() && near(&h.rect, &frame)).map(|h| h.cmd.clone());
    let target = if let (false, Some(cmd)) = (ratio, replay.as_deref()) {
        compute(a, cmd, frame, wid, dest, 0, None)
    } else {
        None
    };
    let target = target.unwrap_or_else(|| {
        let u = from_area.to_unit(&frame);
        if ratio {
            to_area.sub(u.x, u.y, u.w, u.h)
        } else {
            let w = frame.w.min(to_area.w);
            let h = frame.h.min(to_area.h);
            let x = to_area.x + (to_area.w - w) * if from_area.w > frame.w { (frame.x - from_area.x) / (from_area.w - frame.w) } else { 0.5 };
            let y = to_area.y + (to_area.h - h) * if from_area.h > frame.h { (frame.y - from_area.y) / (from_area.h - frame.h) } else { 0.5 };
            Rect::new(x, y, w, h).clamped_into(&to_area)
        }
    });
    let cmd = replay.unwrap_or_default();
    apply(a, win, frame, target, &cmd);
    if a.cfg.general.move_cursor_across_displays {
        warp(target.center());
    }
}

/// Grab the title bar and press the Space-switch shortcut, the only public way to carry a
/// window to another Space.
fn switch_space(a: &mut App, win: &Window, next: bool) {
    let Some(f) = win.frame() else { return };
    let sc = if next { a.cfg.tweaks.space_switch_right } else { a.cfg.tweaks.space_switch_left };
    let grab = CGPoint { x: f.x + (f.w / 2.0).min(120.0).max(80.0), y: f.y + 10.0 };
    let back = crate::tap::cursor();
    unsafe {
        let down = CGEventCreateMouseEvent(std::ptr::null(), kCGEventLeftMouseDown, grab, 0);
        CGEventSetIntegerValueField(down, kCGEventSourceUserData, OUR_EVENT_MARK);
        CGEventPost(kCGHIDEventTap, down);
        CFRelease(down);
    }
    crate::timer::after(0.08, move || unsafe {
        for down in [true, false] {
            let k = CGEventCreateKeyboardEvent(std::ptr::null(), sc.key_code as u16, down);
            CGEventSetFlags(k, sc.modifier_flags);
            CGEventSetIntegerValueField(k, kCGEventSourceUserData, OUR_EVENT_MARK);
            CGEventPost(kCGHIDEventTap, k);
            CFRelease(k);
        }
    });
    crate::timer::after(0.45, move || unsafe {
        let up = CGEventCreateMouseEvent(std::ptr::null(), kCGEventLeftMouseUp, grab, 0);
        CGEventSetIntegerValueField(up, kCGEventSourceUserData, OUR_EVENT_MARK);
        CGEventPost(kCGHIDEventTap, up);
        CFRelease(up);
        CGWarpMouseCursorPosition(CGPoint { x: back.x, y: back.y });
    });
}

/// Resolve a size/position entry against an area. Values <= 1 are fractions of the area,
/// larger values are points, missing ones keep the window's current value.
pub fn place(p: &Placement, win: Rect, area: Rect) -> Rect {
    let dim = |v: Option<f64>, total: f64, cur: f64| match v {
        None => cur,
        Some(v) if v.abs() <= 1.0 => v * total,
        Some(v) => v,
    };
    let w = dim(p.w, area.w, win.w).min(area.w);
    let h = dim(p.h, area.h, win.h).min(area.h);
    let cx = area.mid_x() - w / 2.0;
    let cy = area.mid_y() - h / 2.0;
    let (l, r, t, b) = (area.x, area.max_x() - w, area.y, area.max_y() - h);
    let (x, y) = match p.position.as_str() {
        "top" => (cx, t),
        "bottom" => (cx, b),
        "left" => (l, cy),
        "right" => (r, cy),
        "topLeft" => (l, t),
        "topRight" => (r, t),
        "bottomLeft" => (l, b),
        "bottomRight" => (r, b),
        "custom" => (area.x + dim(p.x, area.w, win.x - area.x), area.y + dim(p.y, area.h, win.y - area.y)),
        "centerOffset" => (cx + dim(p.x, area.w, 0.0), cy + dim(p.y, area.h, 0.0)),
        _ => (cx, cy),
    };
    Rect::new(x, y, w, h)
}

/// Where one step of a custom entry or layout entry puts a window.
pub fn step_target(a: &App, step: &Step, frame: Rect, wid: u32, repeat: usize) -> Option<Rect> {
    let cur = screens::index_for(&a.screens, &frame);
    let dest = screens::resolve(&a.screens, cur, if step.display.is_empty() { "unchanged" } else { &step.display });
    if let Some(cmd) = step.command.as_deref().filter(|c| !c.is_empty()) {
        return compute(a, cmd, frame, wid, dest, repeat, None);
    }
    let p = step.place.as_ref()?;
    let s = a.screens.get(dest)?;
    let area = screens::usable(s, &a.cfg, pin_rect(a).as_ref());
    let win = if dest == cur { frame } else { frame.centered_in(&area) };
    let r = place(p, win, area);
    let fills_edge = (r.x - area.x).abs() < 1.0 || (r.max_x() - area.max_x()).abs() < 1.0;
    Some(if fills_edge { actions::apply_gaps(r, &area, a.cfg.general.gap, a.cfg.general.gaps_on_edges) } else { r })
}

fn run_custom(a: &mut App, name: &str, target: Target) {
    let Some(spec) = a.cfg.custom(name).cloned() else { return };
    if spec.steps.is_empty() {
        return;
    }
    let Some(win) = target.resolve() else { return };
    let Some(frame) = win.frame() else { return };
    a.refresh_screens();
    let wid = win.id();
    let cmd = format!("custom:{name}");
    let count = repeat_count(a, &cmd, wid, &frame);
    let step = &spec.steps[count % spec.steps.len()];
    let Some(to) = step_target(a, step, frame, wid, 0) else { return };
    crate::stash::forget_if_acted(a, wid);
    apply(a, &win, frame, to, &cmd);
    a.last = Some(Last { cmd, wid, rect: win.frame().unwrap_or(to), count });
    a.released_since_last = false;
}

/// The Windows-style arrow state machine.
fn win_arrow(a: &mut App, dir: &str, target: Target) {
    let Some(win) = target.resolve() else { return };
    let Some(frame) = win.frame() else { return };
    a.refresh_screens();
    let wid = win.id();
    let cur = screens::index_for(&a.screens, &frame);
    // Where is it now? Match against the presets the machine moves between.
    let states = ["maximize", "left-half", "right-half", "top-left", "top-right", "bottom-left", "bottom-right"];
    let state = states
        .iter()
        .find(|s| compute(a, s, frame, wid, cur, 0, None).map(|r| near(&r, &frame)).unwrap_or(false))
        .copied()
        .unwrap_or("free");
    let next: &str = match (state, dir) {
        ("free", "left") => "left-half",
        ("free", "right") => "right-half",
        ("free", "up") => "maximize",
        ("free", "down") => "minimize",
        ("maximize", "left") => "left-half",
        ("maximize", "right") => "right-half",
        ("maximize", "down") => "restore",
        ("left-half", "up") => "top-left",
        ("left-half", "down") => "bottom-left",
        ("left-half", "right") => "restore",
        ("left-half", "left") => "<right-half",
        ("right-half", "up") => "top-right",
        ("right-half", "down") => "bottom-right",
        ("right-half", "left") => "restore",
        ("right-half", "right") => ">left-half",
        ("top-left", "down") => "left-half",
        ("top-left", "right") => "top-right",
        ("top-left", "up") => "maximize",
        ("top-left", "left") => "<top-right",
        ("top-right", "down") => "right-half",
        ("top-right", "left") => "top-left",
        ("top-right", "up") => "maximize",
        ("top-right", "right") => ">top-left",
        ("bottom-left", "up") => "left-half",
        ("bottom-left", "right") => "bottom-right",
        ("bottom-left", "down") => "minimize",
        ("bottom-left", "left") => "<bottom-right",
        ("bottom-right", "up") => "right-half",
        ("bottom-right", "left") => "bottom-left",
        ("bottom-right", "down") => "minimize",
        ("bottom-right", "right") => ">bottom-left",
        _ => return,
    };
    if let Some(rest) = next.strip_prefix('<').or_else(|| next.strip_prefix('>')) {
        let side = if next.starts_with('<') { -1 } else { 1 };
        if let Some(adj) = screens::adjacent(&a.screens, cur, side) {
            if let Some(to) = compute(a, rest, frame, wid, adj, 0, None) {
                apply(a, &win, frame, to, rest);
            }
        }
        return;
    }
    run(a, next, Target::Window(win));
}
