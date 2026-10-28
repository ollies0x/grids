//! Displays in global top-left coordinates, in Rectangle's "left to right, line by line" order.

use crate::config::Config;
use crate::ffi::*;
use crate::geom::{Point, Rect};
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;
use objc2_foundation::{NSNumber, NSString};

#[derive(Clone, Debug)]
pub struct Screen {
    pub uuid: String,
    pub name: String,
    pub frame: Rect,
    pub visible: Rect,
    pub notch: bool,
    pub main: bool,
}

impl Screen {
    pub fn portrait(&self) -> bool {
        self.frame.h > self.frame.w
    }
}

fn ns_screens() -> Vec<Retained<NSScreen>> {
    let mtm = MainThreadMarker::new().expect("main thread");
    NSScreen::screens(mtm).to_vec()
}

/// Height of the primary display, the pivot between AppKit and top-left coordinates.
pub fn primary_height() -> f64 {
    ns_screens().first().map(|s| s.frame().size.height).unwrap_or(0.0)
}

/// AppKit (bottom-left) rect -> global top-left rect.
pub fn flip(x: f64, y: f64, w: f64, h: f64, ph: f64) -> Rect {
    Rect::new(x, ph - (y + h), w, h)
}

pub fn display_uuid(id: u32) -> String {
    let u = unsafe { CGDisplayCreateUUIDFromDisplayID(id) };
    let Some(u) = Cf::own(u) else { return String::new() };
    let s = unsafe { CFUUIDCreateString(kCFAllocatorDefault, u.ptr()) };
    Cf::own(s).and_then(|s| cf_to_string(s.ptr())).unwrap_or_default()
}

pub fn all(ordered_by_x: bool) -> Vec<Screen> {
    let list = ns_screens();
    let ph = list.first().map(|s| s.frame().size.height).unwrap_or(0.0);
    let main_id = unsafe { CGMainDisplayID() };
    let mut out: Vec<Screen> = list
        .iter()
        .map(|s| {
            let f = s.frame();
            let v = s.visibleFrame();
            let desc = s.deviceDescription();
            let id = desc
                .objectForKey(&NSString::from_str("NSScreenNumber"))
                .and_then(|o| o.downcast::<NSNumber>().ok())
                .map(|n| n.unsignedIntValue())
                .unwrap_or(0);
            let notch = s.safeAreaInsets().top > 0.0;
            Screen {
                uuid: display_uuid(id),
                name: s.localizedName().to_string(),
                frame: flip(f.origin.x, f.origin.y, f.size.width, f.size.height, ph),
                visible: flip(v.origin.x, v.origin.y, v.size.width, v.size.height, ph),
                notch,
                main: id == main_id,
            }
        })
        .collect();
    order(&mut out, ordered_by_x);
    out
}

/// Rows top to bottom (a screen starts a new row once it is wholly below the row's first
/// screen), each row left to right; or plain x order.
pub fn order(v: &mut [Screen], by_x: bool) {
    if by_x {
        v.sort_by(|a, b| a.frame.x.partial_cmp(&b.frame.x).unwrap());
        return;
    }
    v.sort_by(|a, b| a.frame.y.partial_cmp(&b.frame.y).unwrap());
    let mut rows: Vec<Vec<Screen>> = Vec::new();
    for s in v.iter() {
        match rows.last_mut() {
            Some(row) if s.frame.y < row[0].frame.max_y() - 1.0 => row.push(s.clone()),
            _ => rows.push(vec![s.clone()]),
        }
    }
    let mut flat = Vec::new();
    for mut row in rows {
        row.sort_by(|a, b| a.frame.x.partial_cmp(&b.frame.x).unwrap());
        flat.extend(row);
    }
    v.clone_from_slice(&flat);
}

/// Index of the screen holding most of `r` (or its centre).
pub fn index_for(list: &[Screen], r: &Rect) -> usize {
    let mut best = (0, -1.0);
    for (i, s) in list.iter().enumerate() {
        let a = s.frame.intersect(r).area();
        if a > best.1 {
            best = (i, a);
        }
    }
    if best.1 <= 0.0 {
        if let Some(i) = list.iter().position(|s| s.frame.contains(r.center())) {
            return i;
        }
    }
    best.0
}

pub fn index_at(list: &[Screen], p: Point) -> Option<usize> {
    list.iter().position(|s| s.frame.contains(p))
}

/// The screen directly left (-1) or right (+1) of `i`, overlapping it vertically.
pub fn adjacent(list: &[Screen], i: usize, dir: i32) -> Option<usize> {
    let cur = &list[i].frame;
    list.iter()
        .enumerate()
        .filter(|(j, s)| {
            *j != i
                && s.frame.y < cur.max_y()
                && s.frame.max_y() > cur.y
                && if dir < 0 { s.frame.max_x() <= cur.x + 1.0 } else { s.frame.x >= cur.max_x() - 1.0 }
        })
        .min_by(|a, b| {
            let da = if dir < 0 { cur.x - a.1.frame.max_x() } else { a.1.frame.x - cur.max_x() };
            let db = if dir < 0 { cur.x - b.1.frame.max_x() } else { b.1.frame.x - cur.max_x() };
            da.partial_cmp(&db).unwrap()
        })
        .map(|(j, _)| j)
}

fn stage_manager_on() -> bool {
    pref_value("com.apple.WindowManager", "GloballyEnabled").and_then(|v| cf_to_bool(v.ptr())).unwrap_or(false)
}

/// The rect actions tile into: visible frame minus configured edge gaps, Stage Manager's
/// strip and the pinned window's column.
pub fn usable(s: &Screen, cfg: &Config, pin: Option<&Rect>) -> Rect {
    let t = &cfg.tweaks;
    let mut r = s.visible;
    if !t.screen_edge_gaps_main_only || s.main {
        let top = if s.notch { t.screen_edge_gap_top_notch.unwrap_or(t.screen_edge_gaps[0]) } else { t.screen_edge_gaps[0] };
        r = r.inset(t.screen_edge_gaps[2], top, t.screen_edge_gaps[3], t.screen_edge_gaps[1]);
    }
    let stage = cfg.general.stage_manager_area;
    if stage > 0.0 && stage_manager_on() {
        let w = if stage <= 1.0 { r.w * stage } else { stage };
        r = r.inset(w, 0.0, 0.0, 0.0);
    }
    if let Some(p) = pin {
        if s.frame.intersects(p) {
            if p.mid_x() > r.mid_x() {
                r.w = (p.x - r.x - cfg.general.gap).max(100.0);
            } else {
                let right = r.max_x();
                r.x = p.max_x() + cfg.general.gap;
                r.w = (right - r.x).max(100.0);
            }
        }
    }
    r
}

/// Resolve a Step display selector relative to screen `cur`.
pub fn resolve(list: &[Screen], cur: usize, sel: &str) -> usize {
    let n = list.len().max(1);
    match sel {
        "next" => (cur + 1) % n,
        "previous" | "prev" => (cur + n - 1) % n,
        s if s.starts_with("number:") => s[7..].trim().parse::<usize>().ok().map(|k| k.saturating_sub(1).min(n - 1)).unwrap_or(cur),
        s if s.starts_with("name:") => list.iter().position(|x| x.name == s[5..]).unwrap_or(cur),
        _ => cur,
    }
}
