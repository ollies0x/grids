//! Stash: tuck windows off a screen edge and slide them back when the cursor reaches it.

use crate::app::App;
use crate::ax::{self, Window};
use crate::config::{StashCfg, CMD};
use crate::engine::Target;
use crate::geom::{Point, Rect};
use crate::overlay::Tab;
use crate::screens;
use crate::timer;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Edge {
    Left,
    Right,
    Up,
    Down,
}

pub struct Stashed {
    pub win: Window,
    pub wid: u32,
    pub edge: Edge,
    /// Where it was before stashing (Unstash returns it here).
    pub home: Rect,
    pub hidden: Rect,
    pub shown_rect: Rect,
    pub shown: bool,
    pub screen: Rect,
    /// Held for its lifetime: dropping it removes the tab.
    pub _tab: Option<Tab>,
    pub tab_rect: Option<Rect>,
    pub pending: bool,
}

#[derive(Default)]
pub struct StashState {
    pub entries: Vec<Stashed>,
    pub cycle: usize,
}

const PALETTE: [[f64; 3]; 8] = [
    [0.98, 0.42, 0.42],
    [0.98, 0.70, 0.30],
    [0.95, 0.88, 0.35],
    [0.45, 0.85, 0.50],
    [0.35, 0.80, 0.90],
    [0.45, 0.60, 1.00],
    [0.70, 0.50, 1.00],
    [0.95, 0.50, 0.85],
];

fn rects(edge: Edge, f: Rect, s: Rect, vis: Rect, sliver: f64) -> (Rect, Rect) {
    match edge {
        Edge::Left => (Rect { x: s.x - f.w + sliver, ..f }, Rect { x: vis.x, ..f }),
        Edge::Right => (Rect { x: s.max_x() - sliver, ..f }, Rect { x: vis.max_x() - f.w, ..f }),
        Edge::Down => (Rect { y: s.max_y() - sliver, ..f }, Rect { y: vis.max_y() - f.h, ..f }),
        Edge::Up => (Rect { y: s.y - f.h + sliver, ..f }, Rect { y: vis.y, ..f }),
    }
}

fn tab_rect(edge: Edge, hidden: Rect, s: Rect, taken: &[Rect]) -> Rect {
    let (len, thick) = (64.0, 7.0);
    let mut r = match edge {
        Edge::Left => Rect::new(s.x, hidden.mid_y() - len / 2.0, thick, len),
        Edge::Right => Rect::new(s.max_x() - thick, hidden.mid_y() - len / 2.0, thick, len),
        Edge::Down => Rect::new(hidden.mid_x() - len / 2.0, s.max_y() - thick, len, thick),
        Edge::Up => Rect::new(hidden.mid_x() - len / 2.0, s.y, len, thick),
    };
    // Step along the edge until it does not overlap another tab.
    for _ in 0..20 {
        if !taken.iter().any(|t| t.intersects(&r.inset(-4.0, -4.0, -4.0, -4.0))) {
            break;
        }
        match edge {
            Edge::Left | Edge::Right => r.y += len + 8.0,
            _ => r.x += len + 8.0,
        }
    }
    r.clamped_into(&s)
}

/// Slide a window to `to`, in steps when animating.
fn slide(win: &Window, from: Rect, to: Rect, animate: bool) {
    if !animate {
        win.set_position(Point::new(to.x, to.y));
        return;
    }
    let steps = 7;
    for i in 1..=steps {
        let w = win.clone();
        let t = i as f64 / steps as f64;
        let e = 1.0 - (1.0 - t).powi(3); // ease out
        let r = from.lerp(&to, e);
        timer::after(0.016 * i as f64, move || {
            w.set_position(Point::new(r.x, r.y));
        });
    }
}

fn stash_window(a: &mut App, win: Window, edge: Edge) {
    let Some(f) = win.frame() else { return };
    let wid = win.id();
    forget(a, wid);
    a.refresh_screens();
    let sc = &a.screens[screens::index_for(&a.screens, &f)];
    let (s, vis) = (sc.frame, sc.visible);
    let cfg: StashCfg = a.cfg.stash.clone();
    let (mut hidden, shown_rect) = rects(edge, f, s, vis, cfg.sliver);
    slide(&win, f, hidden, cfg.animate);
    if !cfg.animate {
        // The window server keeps some windows partly on screen; tuck those into a corner.
        if let Some(now) = win.frame() {
            if cfg.hide_in_corner && now.intersect(&s).area() > (cfg.sliver * 2.0) * f.h.max(f.w) {
                hidden = match edge {
                    Edge::Left | Edge::Right => Rect { y: s.max_y() - cfg.sliver * 4.0, ..now },
                    _ => Rect { x: s.max_x() - cfg.sliver * 4.0, ..now },
                };
                win.set_position(Point::new(hidden.x, hidden.y));
            }
        }
    }
    let color = PALETTE[(a.stash.entries.len() + wid as usize) % PALETTE.len()];
    let taken: Vec<Rect> = a.stash.entries.iter().filter_map(|e| e.tab_rect).collect();
    let tr = cfg.color_tabs.then(|| tab_rect(edge, hidden, s, &taken));
    let tab = tr.map(|r| Tab::new(r, color));
    a.stash.entries.push(Stashed { win, wid, edge, home: f, hidden, shown_rect, shown: false, screen: s, _tab: tab, tab_rect: tr, pending: false });
    crate::tap::update_moved();
}

/// Drop a window from the stash without moving it.
pub fn forget(a: &mut App, wid: u32) {
    let before = a.stash.entries.len();
    a.stash.entries.retain(|e| e.wid != wid);
    if a.stash.entries.len() != before {
        timer::after(0.0, crate::tap::update_moved);
    }
}

pub fn forget_if_acted(a: &mut App, wid: u32) {
    if a.cfg.stash.unstash_on_action {
        forget(a, wid);
    }
}

fn unstash(a: &mut App, i: usize) {
    let e = a.stash.entries.remove(i);
    let from = e.win.frame().unwrap_or(e.hidden);
    slide(&e.win, from, e.home, a.cfg.stash.animate);
    e.win.raise();
    timer::after(0.0, crate::tap::update_moved);
}

fn show(a: &mut App, i: usize) {
    let animate = a.cfg.stash.animate;
    let e = &mut a.stash.entries[i];
    if e.shown {
        return;
    }
    slide(&e.win, e.hidden, e.shown_rect, animate);
    e.win.raise();
    e.shown = true;
}

fn hide(a: &mut App, i: usize) {
    let animate = a.cfg.stash.animate;
    let e = &mut a.stash.entries[i];
    if !e.shown {
        return;
    }
    let from = e.win.frame().unwrap_or(e.shown_rect);
    // Remember any move or resize made while it was out.
    if (from.w - e.shown_rect.w).abs() > 1.0 || (from.h - e.shown_rect.h).abs() > 1.0 {
        let (h, s) = rects(e.edge, from, e.screen, e.screen, a.cfg.stash.sliver);
        e.hidden = h;
        e.shown_rect = s;
    }
    slide(&e.win, from, e.hidden, animate);
    e.shown = false;
}

fn at_edge(e: &Stashed, p: Point) -> bool {
    let s = e.screen;
    let m = 2.0;
    if let Some(t) = e.tab_rect {
        if t.inset(-6.0, -6.0, -6.0, -6.0).contains(p) {
            return true;
        }
    }
    let h = e.hidden;
    match e.edge {
        Edge::Left => p.x <= s.x + m && p.y >= h.y && p.y <= h.max_y(),
        Edge::Right => p.x >= s.max_x() - m - 1.0 && p.y >= h.y && p.y <= h.max_y(),
        Edge::Up => p.y <= s.y + m && p.x >= h.x && p.x <= h.max_x(),
        Edge::Down => p.y >= s.max_y() - m - 1.0 && p.x >= h.x && p.x <= h.max_x(),
    }
}

/// Cursor moved: reveal at the edge, hide again on leaving.
pub fn on_mouse(a: &mut App, p: Point, flags: u64) {
    let cfg = a.cfg.stash.clone();
    let cmd = flags & CMD != 0;
    // With tabs, one window per edge spot: prefer the tab under the cursor.
    let tab_hit = a.stash.entries.iter().position(|e| !e.shown && e.tab_rect.map(|t| t.inset(-6.0, -6.0, -6.0, -6.0).contains(p)).unwrap_or(false));
    for i in 0..a.stash.entries.len() {
        let e = &a.stash.entries[i];
        if e.pending {
            continue;
        }
        let wants_show = !e.shown && (tab_hit == Some(i) || tab_hit.is_none() && at_edge(e, p)) && (!cfg.show_requires_cmd || cmd);
        let inside = e.win.frame().map(|f| f.inset(-12.0, -12.0, -12.0, -12.0).contains(p)).unwrap_or(false);
        let wants_hide = e.shown && cfg.hide_on_leave && !inside && !at_edge(e, p) && (!cfg.hide_requires_cmd || cmd);
        if !(wants_show || wants_hide) {
            continue;
        }
        let delay = if wants_show { cfg.show_delay } else { cfg.hide_delay };
        let wid = e.wid;
        if delay <= 0.0 {
            if wants_show { show(a, i) } else { hide(a, i) }
            continue;
        }
        a.stash.entries[i].pending = true;
        timer::after(delay, move || {
            crate::app::with(|a| {
                let Some(i) = a.stash.entries.iter().position(|e| e.wid == wid) else { return };
                a.stash.entries[i].pending = false;
                let p = crate::tap::cursor();
                let e = &a.stash.entries[i];
                let inside = e.win.frame().map(|f| f.inset(-12.0, -12.0, -12.0, -12.0).contains(p)).unwrap_or(false);
                if wants_show && at_edge(e, p) {
                    show(a, i);
                } else if wants_hide && !inside && !at_edge(e, p) {
                    hide(a, i);
                }
            })
        });
    }
}

/// An app came to the front: slide its stashed windows out.
pub fn app_activated(a: &mut App, pid: i32) {
    if !a.cfg.stash.unhide_on_activate {
        return;
    }
    let idx: Vec<usize> = a.stash.entries.iter().enumerate().filter(|(_, e)| e.win.pid == pid && !e.shown).map(|(i, _)| i).collect();
    for i in idx {
        show(a, i);
    }
}

fn nearest_edge(f: &Rect, s: &Rect) -> Edge {
    if f.mid_x() < s.mid_x() { Edge::Left } else { Edge::Right }
}

pub fn command(a: &mut App, cmd: &str, target: Target) {
    match cmd {
        "stash-left" | "stash-right" | "stash-up" | "stash-down" => {
            let edge = match cmd {
                "stash-left" => Edge::Left,
                "stash-right" => Edge::Right,
                "stash-up" => Edge::Up,
                _ => Edge::Down,
            };
            if let Some(w) = target.resolve() {
                stash_window(a, w, edge);
            }
        }
        "unstash" => {
            let wid = target.resolve().map(|w| w.id());
            let i = wid.and_then(|id| a.stash.entries.iter().position(|e| e.wid == id)).or_else(|| a.stash.entries.len().checked_sub(1));
            if let Some(i) = i {
                unstash(a, i);
            }
        }
        "unstash-all" => {
            while !a.stash.entries.is_empty() {
                unstash(a, a.stash.entries.len() - 1);
            }
        }
        "stash-all" | "stash-all-but-front" => {
            let front = ax::focused_window().map(|w| w.id());
            a.refresh_screens();
            let infos = ax::onscreen_windows();
            for info in infos {
                if cmd == "stash-all-but-front" && Some(info.id) == front {
                    continue;
                }
                if a.stash.entries.iter().any(|e| e.wid == info.id) {
                    continue;
                }
                if let Some(w) = ax::window_by_id(info.pid, info.id).filter(|w| w.is_standard()) {
                    let s = a.screens[screens::index_for(&a.screens, &info.bounds)].frame;
                    stash_window(a, w, nearest_edge(&info.bounds, &s));
                }
            }
        }
        c if c.starts_with("unstash-") => {
            let edge = match c {
                "unstash-left" => Edge::Left,
                "unstash-right" => Edge::Right,
                "unstash-up" => Edge::Up,
                _ => Edge::Down,
            };
            if let Some(i) = a.stash.entries.iter().rposition(|e| e.edge == edge) {
                unstash(a, i);
            }
        }
        c if c.starts_with("toggle-stash-") => {
            let edge = match c {
                "toggle-stash-left" => Edge::Left,
                "toggle-stash-right" => Edge::Right,
                "toggle-stash-up" => Edge::Up,
                _ => Edge::Down,
            };
            let idx: Vec<usize> = (0..a.stash.entries.len()).filter(|i| a.stash.entries[*i].edge == edge).collect();
            if idx.is_empty() {
                return;
            }
            let any_hidden = idx.iter().any(|i| !a.stash.entries[*i].shown);
            for i in idx {
                if any_hidden { show(a, i) } else { hide(a, i) }
            }
        }
        c if c.starts_with("cycle-stash-") => {
            let edge = match c {
                "cycle-stash-left" => Edge::Left,
                "cycle-stash-right" => Edge::Right,
                "cycle-stash-up" => Edge::Up,
                _ => Edge::Down,
            };
            let idx: Vec<usize> = (0..a.stash.entries.len()).filter(|i| a.stash.entries[*i].edge == edge).collect();
            if idx.is_empty() {
                return;
            }
            for i in &idx {
                hide(a, *i);
            }
            let pos = idx.iter().position(|i| *i == a.stash.cycle).unwrap_or(0);
            let next = idx[(pos + 1) % idx.len()];
            a.stash.cycle = next;
            show(a, next);
        }
        "cycle-stashed" => {
            let n = a.stash.entries.len();
            if n == 0 {
                return;
            }
            for i in 0..n {
                hide(a, i);
            }
            a.stash.cycle = (a.stash.cycle + 1) % n;
            show(a, a.stash.cycle);
        }
        "toggle-stashed" => {
            let any_hidden = a.stash.entries.iter().any(|e| !e.shown);
            for i in 0..a.stash.entries.len() {
                if any_hidden { show(a, i) } else { hide(a, i) }
            }
        }
        _ => {}
    }
}
