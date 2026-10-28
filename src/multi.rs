//! Commands that move several windows at once.

use crate::actions;
use crate::app::App;
use crate::ax::{self, Window};
use crate::engine::{self, Target};
use crate::geom::Rect;
use crate::screens;
use std::collections::HashMap;

/// Normal windows on screen `idx`, front to back, as AX windows.
fn windows_on(a: &App, idx: usize, pid: Option<i32>) -> Vec<(Window, Rect)> {
    let sf = a.screens[idx].frame;
    let mut by_pid: HashMap<i32, Vec<Window>> = HashMap::new();
    let mut out = Vec::new();
    for info in ax::onscreen_windows() {
        if pid.is_some_and(|p| p != info.pid) || screens::index_for(&a.screens, &info.bounds) != idx || !sf.intersects(&info.bounds) {
            continue;
        }
        let list = by_pid.entry(info.pid).or_insert_with(|| ax::app_windows(info.pid));
        if let Some(w) = list.iter().find(|w| w.id() == info.id) {
            if w.is_standard() && !w.minimized() {
                out.push((w.clone(), info.bounds));
            }
        }
    }
    out
}

fn app_windows_all(pid: i32) -> Vec<(Window, Rect)> {
    ax::app_windows(pid)
        .into_iter()
        .filter(|w| w.is_standard() && !w.minimized())
        .filter_map(|w| w.frame().map(|f| (w, f)))
        .collect()
}

fn grid_place(a: &mut App, wins: &[(Window, Rect)], area: Rect, cols: usize, rows: usize) {
    let g = a.cfg.general.gap;
    let e = a.cfg.general.gaps_on_edges;
    for (i, (w, f)) in wins.iter().take(cols * rows).enumerate() {
        let (c, r) = (i % cols, i / cols);
        // The last row spreads its windows across the full width.
        let in_row = if r == rows - 1 { wins.len().min(cols * rows) - r * cols } else { cols };
        let cw = 1.0 / in_row as f64;
        let cell = area.sub(c as f64 * cw, r as f64 / rows as f64, cw, 1.0 / rows as f64);
        engine::apply(a, w, *f, actions::apply_gaps(cell, &area, g, e), "");
    }
}

pub fn command(a: &mut App, cmd: &str, target: Target) {
    a.refresh_screens();
    let front = target.resolve();
    let front_frame = front.as_ref().and_then(|w| w.frame());
    let idx = front_frame.map(|f| screens::index_for(&a.screens, &f)).unwrap_or(0);
    let pin = engine::pin_rect(a);
    let area = screens::usable(&a.screens[idx], &a.cfg, pin.as_ref());
    let pid = front.as_ref().map(|w| w.pid);
    match cmd {
        "tile2x2" | "tile2x3" | "tile-all" | "tile-app" => {
            let wins = if cmd == "tile-app" { windows_on(a, idx, pid) } else { windows_on(a, idx, None) };
            if wins.is_empty() {
                return;
            }
            let (cols, rows) = match cmd {
                "tile2x2" => (2, 2.min(wins.len().div_ceil(2))),
                "tile2x3" => (3, 2.min(wins.len().div_ceil(3))),
                _ => {
                    let c = (wins.len() as f64).sqrt().ceil() as usize;
                    (c, wins.len().div_ceil(c))
                }
            };
            let cols = cols.min(wins.len());
            grid_place(a, &wins, area, cols, rows.max(1));
        }
        "cascade-all" | "cascade-app" => {
            let mut wins = if cmd == "cascade-app" { windows_on(a, idx, pid) } else { windows_on(a, idx, None) };
            wins.reverse(); // back to front, so the front window ends on top
            let off = a.cfg.tweaks.cascade_offset;
            let n = wins.len() as f64;
            let w = (area.w - off * (n - 1.0)).min(area.w * 0.7).max(area.w * 0.4);
            let h = (area.h - off * (n - 1.0)).min(area.h * 0.75).max(area.h * 0.4);
            for (i, (win, f)) in wins.iter().enumerate() {
                let r = Rect::new(area.x + off * i as f64, area.y + off * i as f64, w, h).clamped_into(&area);
                engine::apply(a, win, *f, r, "");
                if cmd == "cascade-app" {
                    win.raise();
                }
            }
            if let Some(f) = &front {
                f.raise();
            }
        }
        "app-next-display" | "app-prev-display" => {
            let Some(pid) = pid else { return };
            let dir = if cmd == "app-next-display" { 1 } else { -1 };
            for (w, _) in app_windows_all(pid) {
                engine::move_display(a, &w, dir, false);
            }
        }
        "app-left-half" | "app-right-half" => {
            let Some(pid) = pid else { return };
            let act = if cmd == "app-left-half" { "left-half" } else { "right-half" };
            for (w, f) in app_windows_all(pid) {
                let wid = w.id();
                let dest = screens::index_for(&a.screens, &f);
                if let Some(to) = engine::compute(a, act, f, wid, dest, 0, None) {
                    engine::apply(a, &w, f, to, act);
                }
            }
        }
        "tile1x2" | "tile2x1" | "tile1x3" | "tile1x4" => {
            let wins = windows_on(a, idx, None);
            if wins.is_empty() {
                return;
            }
            let (cols, rows) = match cmd {
                "tile2x1" => (2, 1),
                "tile1x3" => (1, 3),
                "tile1x4" => (1, 4),
                _ => (1, 2),
            };
            grid_place(a, &wins, area, cols, rows);
        }
        "reverse-all" => {
            let mut wins = windows_on(a, idx, None);
            if wins.is_empty() {
                return;
            }
            wins.reverse();
            let c = (wins.len() as f64).sqrt().ceil() as usize;
            grid_place(a, &wins, area, c, wins.len().div_ceil(c).max(1));
        }
        "app-collate-left" | "app-collate-right" => {
            let Some(pid) = pid else { return };
            let wins = windows_on(a, idx, Some(pid));
            if wins.is_empty() {
                return;
            }
            let half = if cmd == "app-collate-left" { area.sub(0.0, 0.0, 0.5, 1.0) } else { area.sub(0.5, 0.0, 0.5, 1.0) };
            grid_place(a, &wins, half, 1, wins.len());
        }
        "left-and-quarters" | "right-and-quarters" => {
            let Some(pid) = pid else { return };
            let mut wins = windows_on(a, idx, Some(pid));
            if wins.is_empty() {
                return;
            }
            let left = cmd == "left-and-quarters";
            let (main, rest) = if left {
                (area.sub(0.0, 0.0, 0.5, 1.0), area.sub(0.5, 0.0, 0.5, 1.0))
            } else {
                (area.sub(0.5, 0.0, 0.5, 1.0), area.sub(0.0, 0.0, 0.5, 1.0))
            };
            let (w, f) = wins.remove(0);
            let g = a.cfg.general.gap;
            let e = a.cfg.general.gaps_on_edges;
            engine::apply(a, &w, f, actions::apply_gaps(main, &area, g, e), "");
            if !wins.is_empty() {
                grid_place(a, &wins, rest, 2, 2);
            }
        }
        "hide-all" => {
            let front = pid;
            let mut seen = Vec::new();
            for info in ax::onscreen_windows() {
                if Some(info.pid) == front || seen.contains(&info.pid) {
                    continue;
                }
                seen.push(info.pid);
                if let Some(app) = engine::running_app(info.pid) {
                    app.hide();
                }
            }
        }
        "unminimize-all-windows" => {
            let mut seen = Vec::new();
            for info in ax::onscreen_windows() {
                if seen.contains(&info.pid) {
                    continue;
                }
                seen.push(info.pid);
                for w in ax::app_windows(info.pid) {
                    if w.is_standard() && w.minimized() {
                        w.set_minimized(false);
                    }
                }
            }
        }
        "reveal-desktop-edge" => reveal(a, idx),
        "tidy" => tidy(a, idx),
        _ => {}
    }
}

/// Toggle: push windows covering the right-hand strip (where desktop icons live) clear of it,
/// then put them back.
fn reveal(a: &mut App, idx: usize) {
    if let Some(saved) = a.reveal.take() {
        for (pid, id, r) in saved {
            if let Some(w) = ax::window_by_id(pid, id) {
                w.set_frame(r);
            }
        }
        return;
    }
    let s = a.screens[idx].visible;
    let edge = s.max_x() - a.cfg.tweaks.reveal_desktop_width;
    let mut saved = Vec::new();
    for (w, f) in windows_on(a, idx, None) {
        if f.max_x() <= edge {
            continue;
        }
        let to = if f.w <= edge - s.x {
            Rect { x: edge - f.w, ..f }
        } else {
            Rect::new(s.x.max(f.x), f.y, edge - s.x.max(f.x), f.h)
        };
        saved.push((w.pid, w.id(), f));
        w.set_frame(to);
    }
    a.reveal = Some(saved);
}

/// Snap each window on the screen to the preset it already most resembles.
fn tidy(a: &mut App, idx: usize) {
    const PRESETS: &[&str] = &[
        "maximize", "left-half", "right-half", "top-half", "bottom-half", "top-left", "top-right", "bottom-left",
        "bottom-right", "first-third", "center-third", "last-third", "first-two-thirds", "last-two-thirds",
        "center-half", "almost-maximize",
    ];
    for (w, f) in windows_on(a, idx, None) {
        let wid = w.id();
        let best = PRESETS
            .iter()
            .filter_map(|p| engine::compute(a, p, f, wid, idx, 0, None).map(|r| (p, r)))
            .max_by(|x, y| f.iou(&x.1).partial_cmp(&f.iou(&y.1)).unwrap());
        if let Some((p, r)) = best {
            if f.iou(&r) > 0.4 && !r.approx_eq(&f, 2.0) {
                engine::apply(a, &w, f, r, p);
            }
        }
    }
}
