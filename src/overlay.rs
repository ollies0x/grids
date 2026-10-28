//! Click-through overlay windows: the footprint preview, the throw reticle, snap target
//! outlines, the snap panel and stash tabs. All drawn with layers, no custom drawing code.

use crate::geom::{Point, Rect};
use crate::screens::primary_height;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAnimatablePropertyContainer, NSBackingStoreType, NSColor, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_core_graphics::CGColor;
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::{CALayer, CATransaction};

pub fn mtm() -> MainThreadMarker {
    MainThreadMarker::new().expect("main thread")
}

/// Global top-left rect -> AppKit screen rect.
pub fn to_ns(r: &Rect) -> NSRect {
    let ph = primary_height();
    NSRect::new(NSPoint::new(r.x, ph - (r.y + r.h)), NSSize::new(r.w, r.h))
}

fn local(r: &Rect, h_parent: f64) -> NSRect {
    // Layer-space rect inside a parent of height h_parent (layers are bottom-left too).
    NSRect::new(NSPoint::new(r.x, h_parent - (r.y + r.h)), NSSize::new(r.w, r.h))
}

pub fn color(rgb: [f64; 3], a: f64) -> Retained<CGColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(rgb[0], rgb[1], rgb[2], a).CGColor()
}

fn overlay_window(r: &Rect, level: isize) -> Retained<NSWindow> {
    let mtm = mtm();
    let w = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            to_ns(r),
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { w.setReleasedWhenClosed(false) };
    w.setOpaque(false);
    w.setBackgroundColor(Some(&NSColor::clearColor()));
    w.setHasShadow(false);
    w.setIgnoresMouseEvents(true);
    w.setLevel(level);
    w.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    if let Some(v) = w.contentView() {
        v.setWantsLayer(true);
    }
    w
}

fn root_layer(w: &NSWindow) -> Retained<CALayer> {
    w.contentView().and_then(|v| v.layer()).expect("layer-backed view")
}

const STATUS_LEVEL: isize = 25;

/// The translucent preview of where a window will land.
pub struct Footprint {
    win: Retained<NSWindow>,
    tint: Retained<CALayer>,
    shown: Option<Rect>,
}

impl Footprint {
    pub fn new() -> Footprint {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        let win = overlay_window(&r, STATUS_LEVEL);
        let mtm = mtm();
        let content = win.contentView().unwrap();
        // Frosted glass under a tinted, bordered layer.
        let fx = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), content.bounds());
        fx.setMaterial(NSVisualEffectMaterial::HUDWindow);
        fx.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        fx.setState(NSVisualEffectState::Active);
        fx.setAutoresizingMask(objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable);
        fx.setWantsLayer(true);
        if let Some(l) = fx.layer() {
            l.setCornerRadius(12.0);
            l.setMasksToBounds(true);
        }
        content.addSubview(&fx);
        let tint = NSView::initWithFrame(NSView::alloc(mtm), content.bounds());
        tint.setAutoresizingMask(objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable);
        tint.setWantsLayer(true);
        content.addSubview(&tint);
        let tint_layer = tint.layer().unwrap();
        tint_layer.setCornerRadius(12.0);
        Footprint { win, tint: tint_layer, shown: None }
    }

    pub fn show(&mut self, r: Rect, t: &crate::config::Tweaks, animate: bool) {
        if self.shown.map(|s| s.approx_eq(&r, 0.5)).unwrap_or(false) {
            return;
        }
        self.tint.setBackgroundColor(Some(&color(t.footprint_color, t.footprint_alpha)));
        self.tint.setBorderColor(Some(&color(t.footprint_color, (t.footprint_alpha * 2.6).min(0.95))));
        self.tint.setBorderWidth(t.footprint_border_width);
        let was_hidden = self.shown.is_none();
        let animate = animate && t.footprint_animation > 0.0 || animate && !was_hidden;
        if was_hidden {
            self.win.setFrame_display(to_ns(&r), true);
            self.win.setAlphaValue(if t.footprint_fade { 0.0 } else { 1.0 });
            self.win.orderFrontRegardless();
            if t.footprint_fade {
                self.win.animator().setAlphaValue(1.0);
            }
        } else {
            self.win.setFrame_display_animate(to_ns(&r), true, animate);
        }
        self.shown = Some(r);
    }

    pub fn hide(&mut self) {
        if self.shown.take().is_some() {
            self.win.orderOut(None);
        }
    }

}

/// The Window Throw's aiming mark.
pub struct Reticle {
    win: Retained<NSWindow>,
}

impl Reticle {
    pub fn new(size: f64, rgb: [f64; 3], ring: bool) -> Reticle {
        let win = overlay_window(&Rect::new(0.0, 0.0, size, size), STATUS_LEVEL + 1);
        let l = root_layer(&win);
        l.setCornerRadius(size / 2.0);
        if ring {
            l.setBorderWidth(3.0);
            l.setBorderColor(Some(&color(rgb, 0.95)));
            l.setBackgroundColor(Some(&color(rgb, 0.18)));
        } else {
            l.setBackgroundColor(Some(&color(rgb, 0.85)));
        }
        let dot = CALayer::new();
        let d = 6.0;
        dot.setFrame(local(&Rect::new(size / 2.0 - d / 2.0, size / 2.0 - d / 2.0, d, d), size));
        dot.setCornerRadius(d / 2.0);
        dot.setBackgroundColor(Some(&color([1.0, 1.0, 1.0], 0.95)));
        l.addSublayer(&dot);
        Reticle { win }
    }
    pub fn show(&self, at: Point) {
        let s = self.win.frame().size.width;
        self.win.setFrame_display(to_ns(&Rect::new(at.x - s / 2.0, at.y - s / 2.0, s, s)), true);
        self.win.orderFrontRegardless();
    }
    pub fn hide(&self) {
        self.win.orderOut(None);
    }
}

/// Faint outlines of the snap targets while a drag is in progress.
pub struct Outlines {
    wins: Vec<Retained<NSWindow>>,
}

impl Outlines {
    pub fn show(rects: &[Rect], rgb: [f64; 3]) -> Outlines {
        let wins = rects
            .iter()
            .map(|r| {
                let w = overlay_window(r, STATUS_LEVEL - 1);
                let l = root_layer(&w);
                l.setCornerRadius(10.0);
                l.setBorderWidth(2.0);
                l.setBorderColor(Some(&color(rgb, 0.55)));
                l.setBackgroundColor(Some(&color(rgb, 0.06)));
                w.orderFrontRegardless();
                w
            })
            .collect();
        Outlines { wins }
    }
}

impl Drop for Outlines {
    fn drop(&mut self) {
        for w in &self.wins {
            w.orderOut(None);
        }
    }
}

/// A glass panel of action tiles that appears near the cursor during a drag.
pub struct Panel {
    win: Retained<NSWindow>,
    pub tiles: Vec<(String, Rect)>,
    layers: Vec<Retained<CALayer>>,
    hot: Option<usize>,
}

pub const TILE: f64 = 56.0;
const PAD: f64 = 10.0;

impl Panel {
    pub fn new(items: &[(String, Option<Rect>)], anchor: Point, position: &str, screen: &Rect, rgb: [f64; 3]) -> Panel {
        let n = items.len().max(1) as f64;
        let w = n * TILE + (n + 1.0) * PAD;
        let h = TILE + 2.0 * PAD;
        let off = 40.0;
        let (x, y) = match position {
            "below" => (anchor.x - w / 2.0, anchor.y + off),
            "left" => (anchor.x - w - off, anchor.y - h / 2.0),
            "right" => (anchor.x + off, anchor.y - h / 2.0),
            "top" => (screen.mid_x() - w / 2.0, screen.y + 12.0),
            "bottom" => (screen.mid_x() - w / 2.0, screen.max_y() - h - 12.0),
            "topLeft" => (screen.x + 12.0, screen.y + 12.0),
            "topRight" => (screen.max_x() - w - 12.0, screen.y + 12.0),
            "bottomLeft" => (screen.x + 12.0, screen.max_y() - h - 12.0),
            "bottomRight" => (screen.max_x() - w - 12.0, screen.max_y() - h - 12.0),
            _ => (anchor.x - w / 2.0, anchor.y - h - off),
        };
        let frame = Rect::new(x, y, w, h).clamped_into(screen);
        let win = overlay_window(&frame, STATUS_LEVEL);
        let mtm = mtm();
        let content = win.contentView().unwrap();
        let fx = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), content.bounds());
        fx.setMaterial(NSVisualEffectMaterial::HUDWindow);
        fx.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        fx.setState(NSVisualEffectState::Active);
        fx.setWantsLayer(true);
        let fl = fx.layer().unwrap();
        fl.setCornerRadius(16.0);
        fl.setMasksToBounds(true);
        fl.setBorderWidth(1.0);
        fl.setBorderColor(Some(&color([1.0, 1.0, 1.0], 0.18)));
        content.addSubview(&fx);
        let root = fl;
        let mut tiles = Vec::new();
        let mut layers = Vec::new();
        for (i, (cmd, preview)) in items.iter().enumerate() {
            let tr = Rect::new(PAD + i as f64 * (TILE + PAD), PAD, TILE, TILE);
            let tile = CALayer::new();
            tile.setFrame(local(&tr, h));
            tile.setCornerRadius(10.0);
            tile.setBackgroundColor(Some(&color([1.0, 1.0, 1.0], 0.08)));
            let screen_r = Rect::new(8.0, 12.0, TILE - 16.0, TILE - 24.0);
            let sl = CALayer::new();
            sl.setFrame(local(&screen_r, TILE));
            sl.setCornerRadius(3.0);
            sl.setBorderWidth(1.5);
            sl.setBorderColor(Some(&color([1.0, 1.0, 1.0], 0.55)));
            tile.addSublayer(&sl);
            if let Some(u) = preview {
                let inner = screen_r.inset(2.0, 2.0, 2.0, 2.0);
                let rr = inner.sub(u.x, u.y, u.w, u.h);
                let region = CALayer::new();
                region.setFrame(local(&rr, TILE));
                region.setCornerRadius(2.0);
                region.setBackgroundColor(Some(&color(rgb, 0.9)));
                tile.addSublayer(&region);
            }
            root.addSublayer(&tile);
            tiles.push((cmd.clone(), Rect::new(frame.x + tr.x, frame.y + tr.y, TILE, TILE)));
            layers.push(tile);
        }
        win.orderFrontRegardless();
        Panel { win, tiles, layers, hot: None }
    }

    /// Highlight and return the tile under the cursor.
    pub fn hover(&mut self, p: Point) -> Option<String> {
        let hit = self.tiles.iter().position(|(_, r)| r.contains(p));
        if hit != self.hot {
            CATransaction::begin();
            CATransaction::setDisableActions(true);
            if let Some(i) = self.hot {
                self.layers[i].setBackgroundColor(Some(&color([1.0, 1.0, 1.0], 0.08)));
            }
            if let Some(i) = hit {
                self.layers[i].setBackgroundColor(Some(&color([1.0, 1.0, 1.0], 0.28)));
            }
            CATransaction::commit();
            self.hot = hit;
        }
        hit.map(|i| self.tiles[i].0.clone())
    }
}

impl Drop for Panel {
    fn drop(&mut self) {
        self.win.orderOut(None);
    }
}

/// A coloured tab on the screen edge marking a stashed window.
pub struct Tab {
    win: Retained<NSWindow>,
}

impl Tab {
    pub fn new(r: Rect, rgb: [f64; 3]) -> Tab {
        let win = overlay_window(&r, STATUS_LEVEL);
        let l = root_layer(&win);
        l.setCornerRadius(r.w.min(r.h) / 2.0);
        l.setBackgroundColor(Some(&color(rgb, 0.92)));
        l.setBorderWidth(1.0);
        l.setBorderColor(Some(&color([1.0, 1.0, 1.0], 0.5)));
        win.orderFrontRegardless();
        Tab { win }
    }
}

impl Drop for Tab {
    fn drop(&mut self) {
        self.win.orderOut(None);
    }
}
