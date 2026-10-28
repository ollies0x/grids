//! Rectangles in global top-left coordinates (the space AX and CGWindowList use).

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Rect { x, y, w, h }
    }
    pub fn max_x(&self) -> f64 {
        self.x + self.w
    }
    pub fn max_y(&self) -> f64 {
        self.y + self.h
    }
    pub fn mid_x(&self) -> f64 {
        self.x + self.w / 2.0
    }
    pub fn mid_y(&self) -> f64 {
        self.y + self.h / 2.0
    }
    pub fn center(&self) -> Point {
        Point::new(self.mid_x(), self.mid_y())
    }
    pub fn area(&self) -> f64 {
        self.w.max(0.0) * self.h.max(0.0)
    }
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.max_x() && p.y >= self.y && p.y < self.max_y()
    }
    pub fn intersect(&self, o: &Rect) -> Rect {
        let x = self.x.max(o.x);
        let y = self.y.max(o.y);
        let mx = self.max_x().min(o.max_x());
        let my = self.max_y().min(o.max_y());
        if mx <= x || my <= y {
            Rect::default()
        } else {
            Rect::new(x, y, mx - x, my - y)
        }
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.intersect(o).area() > 0.0
    }
    /// Intersection over union, for "which preset is this window closest to".
    pub fn iou(&self, o: &Rect) -> f64 {
        let i = self.intersect(o).area();
        let u = self.area() + o.area() - i;
        if u <= 0.0 { 0.0 } else { i / u }
    }
    pub fn inset(&self, l: f64, t: f64, r: f64, b: f64) -> Rect {
        Rect::new(self.x + l, self.y + t, (self.w - l - r).max(1.0), (self.h - t - b).max(1.0))
    }
    pub fn approx_eq(&self, o: &Rect, tol: f64) -> bool {
        (self.x - o.x).abs() <= tol
            && (self.y - o.y).abs() <= tol
            && (self.w - o.w).abs() <= tol
            && (self.h - o.h).abs() <= tol
    }
    /// Map a unit-space rect (0..1) into this rect.
    pub fn sub(&self, ux: f64, uy: f64, uw: f64, uh: f64) -> Rect {
        Rect::new(self.x + self.w * ux, self.y + self.h * uy, self.w * uw, self.h * uh)
    }
    /// Express `inner` as fractions of `self`.
    pub fn to_unit(&self, inner: &Rect) -> Rect {
        Rect::new(
            (inner.x - self.x) / self.w,
            (inner.y - self.y) / self.h,
            inner.w / self.w,
            inner.h / self.h,
        )
    }
    pub fn centered_in(&self, area: &Rect) -> Rect {
        let w = self.w.min(area.w);
        let h = self.h.min(area.h);
        Rect::new(area.mid_x() - w / 2.0, area.mid_y() - h / 2.0, w, h)
    }
    /// Shift (not resize) so the rect lies inside `area` where it can.
    pub fn clamped_into(&self, area: &Rect) -> Rect {
        let w = self.w.min(area.w);
        let h = self.h.min(area.h);
        let x = self.x.clamp(area.x, area.max_x() - w);
        let y = self.y.clamp(area.y, area.max_y() - h);
        Rect::new(x, y, w, h)
    }
    pub fn round(&self) -> Rect {
        Rect::new(self.x.round(), self.y.round(), self.w.round(), self.h.round())
    }
    pub fn lerp(&self, o: &Rect, t: f64) -> Rect {
        Rect::new(
            self.x + (o.x - self.x) * t,
            self.y + (o.y - self.y) * t,
            self.w + (o.w - self.w) * t,
            self.h + (o.h - self.h) * t,
        )
    }
}
