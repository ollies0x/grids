//! Drag-to-snap: edges and corners, snap targets and the snap panel. Also the follow-ups to
//! a drag: restoring an unsnapped window's size and resizing neighbours after an edge drag.

use crate::app::App;
use crate::ax::{self, Window};
use crate::config::{SnapCfg, MOD_MASK};
use crate::engine;
use crate::geom::{Point, Rect};
use crate::overlay::{Footprint, Outlines, Panel};
use crate::screens::{self, Screen};
use crate::timer;
use objc2_app_kit::{NSHapticFeedbackManager, NSHapticFeedbackPattern, NSHapticFeedbackPerformanceTime, NSHapticFeedbackPerformer};

#[derive(PartialEq)]
enum Kind {
    Unknown,
    Moving,
    Resizing,
    Still,
}

pub struct Drag {
    win: Option<Window>,
    wid: u32,
    down: Point,
    start: Rect,
    kind: Kind,
    checks: u32,
    /// When the frame was last read while deciding whether this drag moves a window.
    last_check: f64,
    /// What dropping now would do: a command, or a fixed rect from a layout snap target.
    pending: Option<(String, Rect, usize)>,
    prev_cmd: String,
    panel: Option<Panel>,
    outlines: Option<Outlines>,
    targets: Vec<(String, Rect)>,
    unsnapped: bool,
    bundle: String,
}

impl Drag {
    pub fn begin(p: Point) -> Drag {
        Drag {
            win: None,
            wid: 0,
            down: p,
            start: Rect::default(),
            kind: Kind::Unknown,
            checks: 0,
            last_check: 0.0,
            pending: None,
            prev_cmd: String::new(),
            panel: None,
            outlines: None,
            targets: vec![],
            unsnapped: false,
            bundle: String::new(),
        }
    }
}

/// The command a cursor at `p` on screen `s` selects, if any.
pub fn area_command(p: Point, s: &Screen, cfg: &SnapCfg, t: &crate::config::Tweaks, prev: &str) -> Option<String> {
    let f = s.frame;
    let m = t.snap_edge_margin; // top bottom left right
    let c = t.corner_size;
    let left = p.x <= f.x + m[2];
    let right = p.x >= f.max_x() - m[3] - 1.0;
    let top = p.y <= f.y + m[0];
    let bottom = p.y >= f.max_y() - m[1] - 1.0;
    let near_top = p.y <= f.y + c;
    let near_bottom = p.y >= f.max_y() - c;
    let near_left = p.x <= f.x + c;
    let near_right = p.x >= f.max_x() - c;
    let edges = if s.portrait() { &cfg.portrait_edges } else { &cfg.edges };
    let key = if (left && near_top) || (top && near_left) {
        "topLeft"
    } else if (right && near_top) || (top && near_right) {
        "topRight"
    } else if (left && near_bottom) || (bottom && near_left) {
        "bottomLeft"
    } else if (right && near_bottom) || (bottom && near_right) {
        "bottomRight"
    } else if top {
        "top"
    } else if bottom {
        "bottom"
    } else if left {
        "left"
    } else if right {
        "right"
    } else {
        return None;
    };
    let v = edges.get(key).map(|s| s.as_str()).unwrap_or("");
    if v.is_empty() {
        return None;
    }
    let Some(mode) = v.strip_prefix("compound:") else { return Some(v.to_string()) };
    let horizontal_edge = key == "top" || key == "bottom";
    let ux = (p.x - f.x) / f.w;
    let short = t.short_edge;
    let cmd = match mode {
        "half-corners" => {
            let half = if key == "left" { "left-half" } else { "right-half" };
            if p.y <= f.y + short {
                "top-half"
            } else if p.y >= f.max_y() - short {
                "bottom-half"
            } else {
                half
            }
        }
        "thirds" if horizontal_edge => {
            let third = if ux < 1.0 / 3.0 {
                "first-third"
            } else if ux < 2.0 / 3.0 {
                "center-third"
            } else {
                "last-third"
            };
            // Dragging from a side third toward the centre widens to two thirds.
            match (prev, third) {
                ("first-third" | "first-two-thirds", "center-third") => "first-two-thirds",
                ("last-third" | "last-two-thirds", "center-third") => "last-two-thirds",
                _ => third,
            }
        }
        "halves" if horizontal_edge => {
            if ux < 0.5 { "left-half" } else { "right-half" }
        }
        "sixths" if horizontal_edge => {
            let row = if key == "top" { "top" } else { "bottom" };
            if p.x <= f.x + short {
                return Some(format!("{row}-left-sixth"));
            } else if p.x >= f.max_x() - short {
                return Some(format!("{row}-right-sixth"));
            } else if ux < 1.0 / 3.0 {
                "first-third"
            } else if ux < 2.0 / 3.0 {
                "center-third"
            } else {
                "last-third"
            }
        }
        "fourths" if horizontal_edge => ["first-fourth", "second-fourth", "third-fourth", "last-fourth"][((ux * 4.0) as usize).min(3)],
        _ => return None,
    };
    Some(cmd.to_string())
}

fn haptic() {
    let p = NSHapticFeedbackManager::defaultPerformer();
    p.performFeedbackPattern_performanceTime(NSHapticFeedbackPattern::Alignment, NSHapticFeedbackPerformanceTime::Now);
}

/// Snap targets for this drag: custom entries everywhere, layout entries for this app.
fn targets(a: &App, frame: Rect, wid: u32, bundle: &str, flags: u64) -> Vec<(String, Rect)> {
    let need = a.cfg.snap.targets_modifiers & MOD_MASK;
    let pressed = flags & MOD_MASK & need == need && need != 0;
    let show = match a.cfg.snap.targets_mode.as_str() {
        "whilePressing" => pressed,
        "whileNotPressing" => !pressed,
        _ => true,
    };
    if !show {
        return vec![];
    }
    let usable = |step: &crate::config::Step| {
        let p = step.place.as_ref()?;
        if p.w.is_none() || p.h.is_none() || (p.position == "custom" && (p.x.is_none() || p.y.is_none())) {
            return None;
        }
        if matches!(step.display.as_str(), "next" | "previous") {
            return None;
        }
        engine::step_target(a, step, frame, wid, 0)
    };
    let mut out = Vec::new();
    for c in a.cfg.custom.iter().filter(|c| c.snap_target) {
        if let Some(r) = c.steps.first().and_then(usable) {
            out.push((format!("custom:{}", c.name), r));
        }
    }
    for l in &a.cfg.layouts {
        for e in l.entries.iter().filter(|e| e.snap_target && (e.app == bundle || e.app == "*")) {
            if let Some(r) = e.steps.first().and_then(usable) {
                out.push((String::new(), r));
            }
        }
    }
    out
}

/// Called for every left-mouse drag event.
pub fn dragged(a: &mut App, d: &mut Drag, p: Point, flags: u64, fp: &mut Footprint) {
    if d.kind == Kind::Unknown {
        if d.win.is_none() {
            d.win = ax::window_at(d.down).filter(|w| w.is_standard());
            match &d.win {
                Some(w) => {
                    d.wid = w.id();
                    d.start = w.frame().unwrap_or_default();
                    d.bundle = engine::bundle_id(w.pid);
                }
                None => {
                    d.kind = Kind::Still;
                    return;
                }
            }
        }
        // Reading the frame is a round trip to the app; drags arrive at up to 120 Hz and most
        // are text selections, so look at most every 30 ms.
        let t = crate::ffi::now();
        if t - d.last_check < 0.03 {
            return;
        }
        d.last_check = t;
        d.checks += 1;
        let Some(now) = d.win.as_ref().and_then(|w| w.frame()) else { return };
        let moved = (now.x - d.start.x).abs() > 0.5 || (now.y - d.start.y).abs() > 0.5;
        let sized = (now.w - d.start.w).abs() > 0.5 || (now.h - d.start.h).abs() > 0.5;
        // A resize keeps one edge fixed on each axis. Anything else is a move, including
        // macOS shrinking a full-screen-sized window back as the drag starts.
        let pinned = |a0: f64, a1: f64, b0: f64, b1: f64| (a0 - b0).abs() < 1.0 || (a1 - b1).abs() < 1.0;
        let edge_resize = sized && pinned(now.x, now.max_x(), d.start.x, d.start.max_x()) && pinned(now.y, now.max_y(), d.start.y, d.start.max_y());
        if edge_resize {
            d.kind = Kind::Resizing;
        } else if moved || sized {
            if sized {
                // The system already restored a size; don't restore on top of it.
                d.unsnapped = true;
                d.start = now;
            }
            d.kind = Kind::Moving;
            on_move_start(a, d, p, flags);
        } else if d.checks > 30 {
            d.kind = Kind::Still;
        }
        return;
    }
    if d.kind != Kind::Moving || !a.cfg.snap.enabled {
        return;
    }
    let mods_ok = a.cfg.snap.modifiers == 0 || flags & MOD_MASK & a.cfg.snap.modifiers == a.cfg.snap.modifiers;
    let Some(idx) = screens::index_at(&a.screens, p) else { return };
    let mut next: Option<(String, Rect, usize)> = None;
    if let Some(panel) = d.panel.as_mut() {
        if let Some(cmd) = panel.hover(p) {
            next = engine::compute(a, &cmd, d.start, d.wid, idx, 0, None).map(|r| (cmd, r, idx));
        }
    }
    if next.is_none() {
        if let Some((cmd, r)) = d.targets.iter().find(|(_, r)| r.contains(p)) {
            next = Some((cmd.clone(), *r, idx));
        }
    }
    if next.is_none() && mods_ok {
        if let Some(cmd) = area_command(p, &a.screens[idx], &a.cfg.snap, &a.cfg.tweaks, &d.prev_cmd) {
            if let Some(r) = target_rect(a, &cmd, d.start, d.wid, idx) {
                next = Some((cmd, r, idx));
            }
        }
    }
    let changed = next.as_ref().map(|n| &n.0) != d.pending.as_ref().map(|n| &n.0);
    match &next {
        Some((cmd, r, _)) => {
            if changed && a.cfg.snap.haptic {
                haptic();
            }
            if !cmd.is_empty() {
                d.prev_cmd = cmd.clone();
            }
            fp.show(*r, &a.cfg.tweaks, a.cfg.snap.animate);
        }
        None => fp.hide(),
    }
    d.pending = next;
}

fn target_rect(a: &App, cmd: &str, frame: Rect, wid: u32, idx: usize) -> Option<Rect> {
    if let Some(name) = cmd.strip_prefix("custom:") {
        let spec = a.cfg.custom(name)?;
        return engine::step_target(a, spec.steps.first()?, frame.centered_in(&a.screens[idx].visible), wid, 0);
    }
    if crate::actions::find(cmd).map(|x| matches!(x.cat, "Fill" | "Halves" | "Corners" | "Thirds" | "Additional Thirds" | "Corner Two Thirds" | "Fourths" | "Fifths" | "Sixths" | "Eighths" | "Ninths" | "Maximize" | "Other" | "Move" | "Snap" | "Size")).unwrap_or(false) {
        return engine::compute(a, cmd, frame, wid, idx, 0, None);
    }
    // Stash, control and other non-geometry commands: preview the whole screen dimmed.
    Some(a.screens[idx].visible.inset(40.0, 40.0, 40.0, 40.0))
}

fn on_move_start(a: &mut App, d: &mut Drag, p: Point, flags: u64) {
    a.refresh_screens();
    // Unstash when dragged clear of the edge.
    if a.cfg.stash.unstash_on_move {
        crate::stash::forget(a, d.wid);
    }
    // Dragging a snapped window away restores its pre-snap size.
    if a.cfg.snap.restore_on_unsnap && !d.unsnapped {
        if let Some(h) = a.hist.get(&d.wid).cloned() {
            if let (Some(orig), true) = (h.original, h.rect.approx_eq(&d.start, 3.0)) {
                if let Some(w) = &d.win {
                    let rel = if d.start.w > 0.0 { (p.x - d.start.x) / d.start.w } else { 0.5 };
                    let now = w.frame().unwrap_or(d.start);
                    let nx = p.x - orig.w * rel;
                    w.set_size(orig.w, orig.h);
                    w.set_position(Point::new(nx, now.y));
                    d.start = Rect::new(nx, now.y, orig.w, orig.h);
                }
                a.hist.remove(&d.wid);
                d.unsnapped = true;
            }
        }
    }
    if !a.cfg.snap.enabled {
        return;
    }
    d.targets = targets(a, d.start, d.wid, &d.bundle, flags);
    if !d.targets.is_empty() {
        let rects: Vec<Rect> = d.targets.iter().map(|t| t.1).collect();
        d.outlines = Some(Outlines::show(&rects, a.cfg.tweaks.footprint_color));
    }
    if a.cfg.snap.panel_enabled && !a.cfg.snap.panel_items.is_empty() {
        let idx = screens::index_at(&a.screens, p).unwrap_or(0);
        let items: Vec<(String, Option<Rect>)> = a.cfg.snap.panel_items.iter().map(|c| (c.clone(), crate::layouts::preview_unit(a, c))).collect();
        d.panel = Some(Panel::new(&items, p, &a.cfg.snap.panel_position, &a.screens[idx].visible, a.cfg.tweaks.footprint_color));
    }
}

/// Mouse up: snap if a target was showing, or resize neighbours after an edge drag.
pub fn released(a: &mut App, d: Drag, fp: &mut Footprint) {
    fp.hide();
    let Some(win) = d.win else { return };
    match d.kind {
        Kind::Moving => {
            let Some((cmd, rect, _)) = d.pending else { return };
            let wid = d.wid;
            // Let the app finish its own drag before moving the window, and check once more
            // afterwards: some apps settle their drag after the mouse-up and undo the snap.
            let (w2, r2) = (win.clone(), rect);
            timer::after(0.3, move || {
                if let Some(now) = w2.frame() {
                    if !now.approx_eq(&r2, 3.0) && crate::app::try_with(|a| a.hist.get(&wid).map(|h| h.rect.approx_eq(&r2, 3.0)).unwrap_or(false)).unwrap_or(false) {
                        w2.set_frame(r2);
                    }
                }
            });
            timer::after(0.02, move || {
                crate::app::with(|a| {
                    let from = a.hist.get(&wid).and_then(|h| h.original).unwrap_or(d.start);
                    let geometric = crate::actions::find(&cmd).map(|x| x.cat != "Control" && x.cat != "Stash").unwrap_or(true);
                    if geometric {
                        let now = win.frame().unwrap_or(from);
                        engine::apply(a, &win, now, rect, &cmd);
                        if let Some(h) = a.hist.get_mut(&wid) {
                            h.snapped = true;
                            h.original = Some(Rect { x: now.x, y: now.y, ..from });
                        }
                    } else {
                        engine::run(a, &cmd, engine::Target::Window(win.clone()));
                    }
                })
            });
        }
        Kind::Resizing if a.cfg.general.resize_adjacent => {
            let start = d.start;
            let wid = d.wid;
            timer::after(0.05, move || {
                crate::app::with(|a| {
                    if let Some(end) = win.frame() {
                        resize_neighbours(a, wid, start, end);
                    }
                })
            });
        }
        _ => {}
    }
}

/// After a window edge is dragged, move the touching edges of its neighbours with it.
fn resize_neighbours(a: &mut App, wid: u32, s: Rect, e: Rect) {
    let tol = a.cfg.general.gap + 6.0;
    let gap = a.cfg.general.gap;
    for o in ax::onscreen_windows() {
        if o.id == wid {
            continue;
        }
        let b = o.bounds;
        let v_overlap = b.y < s.max_y() && b.max_y() > s.y;
        let h_overlap = b.x < s.max_x() && b.max_x() > s.x;
        let mut n = b;
        if v_overlap && (e.max_x() - s.max_x()).abs() > 0.5 && (b.x - s.max_x()).abs() <= tol {
            n.x = e.max_x() + gap;
            n.w = b.max_x() - n.x;
        }
        if v_overlap && (e.x - s.x).abs() > 0.5 && (b.max_x() - s.x).abs() <= tol {
            n.w = e.x - gap - b.x;
        }
        if h_overlap && (e.max_y() - s.max_y()).abs() > 0.5 && (b.y - s.max_y()).abs() <= tol {
            n.y = e.max_y() + gap;
            n.h = b.max_y() - n.y;
        }
        if h_overlap && (e.y - s.y).abs() > 0.5 && (b.max_y() - s.y).abs() <= tol {
            n.h = e.y - gap - b.y;
        }
        if n != b && n.w > 80.0 && n.h > 60.0 {
            if let Some(w) = ax::window_by_id(o.pid, o.id) {
                w.set_frame(n);
            }
        }
    }
}
