//! Pin Mode (Rectangle's Todo mode): one app's window holds a column at the side of the main
//! screen, and every other action tiles into what is left.

use crate::app::App;
use crate::ax::{self, Window};
use crate::geom::Rect;
use crate::screens;
use objc2_app_kit::NSRunningApplication;
use objc2_foundation::NSString;

#[derive(Default)]
pub struct PinState {
    /// The pinned column while Pin Mode is on.
    pub rect: Option<Rect>,
}

fn pinned_window(bundle: &str) -> Option<Window> {
    let apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(bundle));
    let app = apps.iter().next()?;
    let pid = app.processIdentifier();
    ax::app_windows(pid).into_iter().find(|w| w.is_standard() && !w.minimized())
}

fn column(a: &App) -> Option<Rect> {
    let s = a.screens.iter().find(|s| s.main).or(a.screens.first())?;
    let area = screens::usable(s, &a.cfg, None);
    let p = &a.cfg.pin;
    let w = if p.width <= 1.0 { area.w * p.width } else { p.width.min(area.w * 0.8) };
    Some(if p.side == "left" { Rect::new(area.x, area.y, w, area.h) } else { Rect::new(area.max_x() - w, area.y, w, area.h) })
}

/// Rectangle Pro's left/right Todo actions pin the front app to that side.
pub fn toggle_side(a: &mut App, side: &str) {
    if a.cfg.pin.app.is_empty() {
        if let Some(pid) = crate::engine::frontmost_pid() {
            a.cfg.pin.app = crate::engine::bundle_id(pid);
        }
    }
    a.cfg.pin.side = side.to_string();
    toggle(a);
}

pub fn toggle(a: &mut App) {
    if a.pin.rect.take().is_some() {
        return;
    }
    if a.cfg.pin.app.is_empty() {
        eprintln!("grids: Pin Mode has no app chosen");
        return;
    }
    a.refresh_screens();
    a.pin.rect = column(a);
    reflow(a);
}

/// Put the pinned window back in its column and move overlapping windows out of it.
pub fn reflow(a: &mut App) {
    let Some(col) = a.pin.rect.or_else(|| column(a)) else { return };
    if a.pin.rect.is_none() {
        return;
    }
    let Some(pw) = pinned_window(&a.cfg.pin.app) else { return };
    pw.set_frame(col);
    let pid = pw.pid;
    let pinned_id = pw.id();
    for info in ax::onscreen_windows() {
        if info.id == pinned_id || !info.bounds.intersects(&col) {
            continue;
        }
        let idx = screens::index_for(&a.screens, &info.bounds);
        let area = screens::usable(&a.screens[idx], &a.cfg, Some(&col));
        let mut r = info.bounds;
        if r.w > area.w {
            r.w = area.w;
        }
        let r = r.clamped_into(&area);
        if let Some(w) = ax::window_by_id(info.pid, info.id) {
            if w.is_standard() {
                w.set_frame(r);
            }
        }
    }
    let _ = pid;
}
