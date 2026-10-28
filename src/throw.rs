//! Cursor movement: Window Throw (pie of eight sectors plus Long Throw), Quick Throw, and
//! modifier-held Move & Resize.

use crate::app::App;
use crate::ax::{self, Window};
use crate::config::MOD_MASK;
use crate::engine;
use crate::geom::{Point, Rect};
use crate::overlay::{Footprint, Reticle};
use crate::screens;

pub struct Throw {
    win: Option<Window>,
    wid: u32,
    frame: Rect,
    origin: Point,
    origin_screen: usize,
    reticle: Option<Reticle>,
    shown: bool,
    pub target: Option<(String, Rect)>,
    scale: f64,
    /// Furthest the cursor got, for "click without throwing" detection.
    pub max_dist: f64,
}

/// Sector 0 is up, clockwise.
fn sector(d: Point) -> usize {
    let ang = d.y.atan2(d.x).to_degrees(); // 0 = right, 90 = down (y grows downward)
    let from_up = (ang + 90.0 + 360.0 + 22.5) % 360.0;
    (from_up / 45.0) as usize % 8
}

impl Throw {
    pub fn begin(a: &mut App, at: Point) -> Option<Throw> {
        let win = ax::window_at(at).filter(|w| w.is_standard())?;
        if a.cfg.throw.ignore_without_toolbar && !win.has_toolbar() {
            return None;
        }
        let frame = win.frame()?;
        a.refresh_screens();
        let origin_screen = screens::index_at(&a.screens, at).unwrap_or(0);
        Some(Throw {
            wid: win.id(),
            win: Some(win),
            frame,
            origin: at,
            origin_screen,
            reticle: None,
            shown: false,
            target: None,
            scale: 1.0,
            max_dist: 0.0,
        })
    }

    pub fn update(&mut self, a: &App, p: Point, fp: &mut Footprint) {
        let tc = &a.cfg.throw;
        let d = Point::new(p.x - self.origin.x, p.y - self.origin.y);
        let dist = (d.x * d.x + d.y * d.y).sqrt();
        self.max_dist = self.max_dist.max(dist);
        if !self.shown {
            if dist < tc.min_dist_to_appear {
                return;
            }
            let r = Reticle::new(tc.reticle_size, tc.reticle_color, tc.reticle_ring);
            r.show(self.origin);
            self.reticle = Some(r);
            self.shown = true;
        }
        let idx = screens::index_at(&a.screens, p).unwrap_or(self.origin_screen);
        let cmd = if idx != self.origin_screen && tc.other_display_center {
            Some(("center".to_string(), idx))
        } else if dist < tc.safe_area {
            None
        } else {
            let list = if tc.long_throw && dist >= tc.long_throw_distance { &tc.long_sectors } else { &tc.sectors };
            list.get(sector(d)).filter(|c| !c.is_empty()).map(|c| (c.clone(), self.origin_screen))
        };
        self.target = cmd.and_then(|(c, i)| {
            let r = crate::layouts::command_rect(a, &c, self.frame, self.wid, i)?;
            Some((c, scale_rect(r, self.scale, &a.screens[i].visible)))
        });
        match &self.target {
            Some((_, r)) => fp.show(*r, &a.cfg.tweaks, true),
            None => fp.hide(),
        }
    }

    /// Scrolling while aiming shrinks or grows the footprint.
    pub fn scroll(&mut self, a: &App, delta: f64, p: Point, fp: &mut Footprint) {
        if !a.cfg.throw.scroll_resizes {
            return;
        }
        self.scale = (self.scale * (1.0 + delta * 0.02)).clamp(0.3, 1.0);
        self.update(a, p, fp);
    }

    pub fn finish(mut self, a: &mut App, fp: &mut Footprint) {
        fp.hide();
        if let Some(r) = self.reticle.take() {
            r.hide();
        }
        let (Some(win), Some((cmd, rect))) = (self.win.take(), self.target.take()) else { return };
        let frame = self.frame;
        crate::timer::after(0.0, move || {
            crate::app::with(|a| {
                let geometric = crate::actions::find(&cmd).map(|x| !matches!(x.cat, "Control" | "Stash" | "Multiple Windows" | "Pin")).unwrap_or(true);
                if geometric && !cmd.starts_with("layout:") {
                    engine::apply(a, &win, frame, rect, &cmd);
                } else {
                    engine::run(a, &cmd, engine::Target::Window(win));
                }
            })
        });
        let _ = a;
    }

    pub fn cancel(mut self, fp: &mut Footprint) {
        fp.hide();
        if let Some(r) = self.reticle.take() {
            r.hide();
        }
    }
}

fn scale_rect(r: Rect, s: f64, bounds: &Rect) -> Rect {
    if s >= 0.999 {
        return r;
    }
    let (w, h) = (r.w * s, r.h * s);
    Rect::new(r.mid_x() - w / 2.0, r.mid_y() - h / 2.0, w, h).clamped_into(bounds)
}

/// Quick Throw: press and release a modifier while sweeping the cursor across a window.
pub struct Quick {
    mods: u64,
    start: Point,
    at: f64,
    pub cancelled: bool,
}

impl Quick {
    pub fn arm(a: &App, flags: u64, p: Point) -> Option<Quick> {
        let m = flags & MOD_MASK;
        a.cfg.quick_throw.iter().any(|r| r.modifiers == m && m != 0).then(|| Quick { mods: m, start: p, at: crate::ffi::now(), cancelled: false })
    }

    /// Modifiers released: work out the sweep direction and fire its command.
    pub fn release(self, a: &mut App, p: Point) {
        if self.cancelled || crate::ffi::now() - self.at > 0.9 {
            return;
        }
        let (dx, dy) = (p.x - self.start.x, p.y - self.start.y);
        if dx.abs().max(dy.abs()) < 12.0 {
            return;
        }
        let Some(rule) = a.cfg.quick_throw.iter().find(|r| r.modifiers == self.mods) else { return };
        let cmd = if dx.abs() > dy.abs() {
            if dx < 0.0 { &rule.left } else { &rule.right }
        } else if dy < 0.0 {
            &rule.up
        } else {
            &rule.down
        };
        let Some(cmd) = cmd.clone().filter(|c| !c.is_empty()) else { return };
        let Some(win) = ax::window_at(self.start).or_else(|| ax::window_at(p)) else { return };
        if !win.is_standard() {
            return;
        }
        crate::timer::after(0.0, move || crate::app::with(|a| engine::run(a, &cmd, engine::Target::Window(win))));
        let _ = a;
    }
}

/// Hold modifiers and move the cursor to drag (or resize) the window under it.
pub struct MoveResize {
    win: Window,
    start_cursor: Point,
    start: Rect,
    resize: bool,
    /// Which corner follows the cursor when resizing: (-1|1, -1|1).
    corner: (i32, i32),
    last_apply: f64,
}

impl MoveResize {
    pub fn begin(at: Point, resize: bool) -> Option<MoveResize> {
        let win = ax::window_at(at).filter(|w| w.is_standard())?;
        let start = win.frame()?;
        let corner = (if at.x < start.mid_x() { -1 } else { 1 }, if at.y < start.mid_y() { -1 } else { 1 });
        Some(MoveResize { win, start_cursor: at, start, resize, corner, last_apply: 0.0 })
    }

    pub fn update(&mut self, p: Point) {
        let now = crate::ffi::now();
        if now - self.last_apply < 1.0 / 90.0 {
            return;
        }
        self.last_apply = now;
        let (dx, dy) = (p.x - self.start_cursor.x, p.y - self.start_cursor.y);
        let s = self.start;
        if !self.resize {
            self.win.set_position(Point::new(s.x + dx, s.y + dy));
            return;
        }
        let mut r = s;
        if self.corner.0 < 0 {
            r.x = s.x + dx;
            r.w = s.w - dx;
        } else {
            r.w = s.w + dx;
        }
        if self.corner.1 < 0 {
            r.y = s.y + dy;
            r.h = s.h - dy;
        } else {
            r.h = s.h + dy;
        }
        if r.w > 120.0 && r.h > 80.0 {
            if self.corner.0 < 0 || self.corner.1 < 0 {
                self.win.set_position(Point::new(r.x, r.y));
            }
            self.win.set_size(r.w, r.h);
        }
    }
}
