//! The status item and its menu, also shown as a context menu at the cursor.

use crate::actions::{ACTIONS, CATEGORIES};
use crate::app;
use crate::ax::Window;
use crate::config::{Config, Shortcut, CMD, CTRL, OPT, SHIFT};
use crate::geom::Rect;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{sel, MainThreadMarker, MainThreadOnly, Message};
use objc2_app_kit::{
    NSBezierPath, NSColor, NSEventModifierFlags, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
    NSVariableStatusItemLength,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::cell::RefCell;

thread_local! {
    static ITEM: RefCell<Option<Retained<NSStatusItem>>> = const { RefCell::new(None) };
    static TARGET: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
    /// Window the next menu action applies to (context menu); None = frontmost.
    pub static MENU_WINDOW: RefCell<Option<Window>> = const { RefCell::new(None) };
    static CLEAR: RefCell<Option<crate::timer::Timer>> = const { RefCell::new(None) };
}

pub fn set_target(t: Retained<AnyObject>) {
    TARGET.with(|x| *x.borrow_mut() = Some(t));
}

fn mtm() -> MainThreadMarker {
    MainThreadMarker::new().unwrap()
}

/// A small template icon: a window (title bar above a pane) with the action's region filled.
/// The frame is near-square on purpose: a wide, flat, left-filled box reads as a battery in the
/// menu bar, which is exactly what it was mistaken for.
pub fn icon(unit: Option<Rect>, size: f64) -> Retained<NSImage> {
    let block = RcBlock::new(move |r: NSRect| -> objc2::runtime::Bool {
        let side = r.size.height - 2.0;
        let screen = NSRect::new(
            NSPoint::new(r.origin.x + (r.size.width - side) / 2.0, r.origin.y + 1.0),
            NSSize::new(side, side),
        );
        NSColor::blackColor().set();
        let p = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(screen, 2.5, 2.5);
        p.setLineWidth(1.3);
        p.stroke();
        let bar = NSRect::new(
            NSPoint::new(screen.origin.x + 1.8, screen.origin.y + side - 4.0),
            NSSize::new(side - 3.6, 1.9),
        );
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bar, 0.9, 0.9).fill();
        if let Some(u) = unit {
            let inner = NSRect::new(NSPoint::new(screen.origin.x + 1.8, screen.origin.y + 1.8), NSSize::new(side - 3.6, side - 7.2));
            // Unit rects are top-left based; the image is bottom-left.
            let fr = NSRect::new(
                NSPoint::new(inner.origin.x + inner.size.width * u.x, inner.origin.y + inner.size.height * (1.0 - u.y - u.h)),
                NSSize::new((inner.size.width * u.w).max(1.5), (inner.size.height * u.h).max(1.5)),
            );
            NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(fr, 1.0, 1.0).fill();
        }
        objc2::runtime::Bool::YES
    });
    let img = NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(size * 1.18, size), false, &block);
    img.setTemplate(true);
    img
}

fn status_icon(variant: u8) -> Retained<NSImage> {
    match variant {
        1 => icon(Some(Rect::new(0.0, 0.0, 0.5, 1.0)), 15.0),
        2 => icon(Some(Rect::new(0.5, 0.0, 0.5, 0.5)), 15.0),
        _ => icon(Some(Rect::new(0.0, 0.0, 1.0 / 3.0, 1.0)), 15.0),
    }
}

fn key_equivalent(sc: &Shortcut) -> Option<String> {
    let s = match sc.key_code {
        123 => '\u{F702}'.to_string(),
        124 => '\u{F703}'.to_string(),
        125 => '\u{F701}'.to_string(),
        126 => '\u{F700}'.to_string(),
        36 => "\r".to_string(),
        51 => '\u{8}'.to_string(),
        117 => '\u{7F}'.to_string(),
        49 => " ".to_string(),
        code => {
            let n = crate::keys::key_name(code);
            if n.chars().count() == 1 { n.to_lowercase() } else { return None }
        }
    };
    Some(s)
}

fn item(title: &str, cmd: &str, cfg: &Config, unit: Option<Rect>, show_icon: bool) -> Retained<NSMenuItem> {
    let mtm = mtm();
    let it = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(NSMenuItem::alloc(mtm), &NSString::from_str(title), Some(sel!(menuAction:)), &NSString::from_str(""))
    };
    TARGET.with(|t| unsafe { it.setTarget(t.borrow().as_deref()) });
    unsafe { it.setRepresentedObject(Some(&NSString::from_str(cmd))) };
    if let Some(sc) = cfg.shortcuts.get(cmd).filter(|s| s.key_code != 0) {
        if let Some(k) = key_equivalent(sc) {
            it.setKeyEquivalent(&NSString::from_str(&k));
            let mut m = NSEventModifierFlags::empty();
            for (f, nf) in [(CMD, NSEventModifierFlags::Command), (OPT, NSEventModifierFlags::Option), (CTRL, NSEventModifierFlags::Control), (SHIFT, NSEventModifierFlags::Shift)] {
                if sc.modifier_flags & f != 0 {
                    m |= nf;
                }
            }
            it.setKeyEquivalentModifierMask(m);
        }
    }
    if show_icon {
        it.setImage(Some(&icon(unit, 12.0)));
    }
    it
}

fn plain(title: &str, action: Sel) -> Retained<NSMenuItem> {
    let it = unsafe { NSMenuItem::initWithTitle_action_keyEquivalent(NSMenuItem::alloc(mtm()), &NSString::from_str(title), Some(action), &NSString::from_str("")) };
    TARGET.with(|t| unsafe { it.setTarget(t.borrow().as_deref()) });
    it
}

fn submenu(title: &str) -> (Retained<NSMenuItem>, Retained<NSMenu>) {
    let mtm = mtm();
    let it = NSMenuItem::new(mtm);
    it.setTitle(&NSString::from_str(title));
    let m = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(title));
    it.setSubmenu(Some(&m));
    (it, m)
}

pub fn build(a: &app::App) -> Retained<NSMenu> {
    let menu = NSMenu::new(mtm());
    fill(&menu, a);
    menu
}

/// Populate `menu` from the current state. The status menu is filled only as it opens and
/// emptied after it closes, so about a hundred items and their icons are not kept resident.
pub fn fill(menu: &NSMenu, a: &app::App) {
    let cfg = &a.cfg;
    let mtm = mtm();
    menu.removeAllItems();
    menu.setAutoenablesItems(false);
    let icons = cfg.menu.show_icons;
    let hidden_cats: Vec<&str> = cfg.menu.hidden_categories.iter().map(|s| s.as_str()).collect();
    let mut first = true;
    for cat in CATEGORIES {
        if hidden_cats.contains(cat) || *cat == "App" || *cat == "Pin" {
            continue;
        }
        let acts: Vec<_> = ACTIONS.iter().filter(|x| x.cat == *cat && !cfg.menu.hidden_actions.iter().any(|h| h == x.id)).collect();
        if acts.is_empty() {
            continue;
        }
        let use_sub = cfg.menu.submenu_categories.iter().any(|c| c == cat);
        let target = if use_sub {
            let (it, m) = submenu(cat);
            menu.addItem(&it);
            m
        } else {
            if !first {
                menu.addItem(&NSMenuItem::separatorItem(mtm));
            }
            menu.retain()
        };
        first = false;
        for x in acts {
            target.addItem(&item(x.name, x.id, cfg, crate::actions::unit_preview(x.id), icons));
        }
    }
    if cfg.menu.show_custom && !cfg.custom.is_empty() {
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        for c in &cfg.custom {
            let cmd = format!("custom:{}", c.name);
            menu.addItem(&item(&c.name, &cmd, cfg, crate::layouts::preview_unit(a, &cmd), icons));
        }
    }
    if cfg.menu.show_layouts {
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        let (it, m) = submenu("Layouts");
        for l in &cfg.layouts {
            m.addItem(&item(&l.name, &format!("layout:{}", l.name), cfg, None, false));
        }
        if !cfg.layouts.is_empty() {
            m.addItem(&NSMenuItem::separatorItem(mtm));
        }
        m.addItem(&item("Save Current Layout…", "save-layout", cfg, None, false));
        menu.addItem(&it);
    }
    if cfg.pin.show_in_menu && !cfg.pin.app.is_empty() {
        let label = if a.pin.rect.is_some() { format!("Unpin {}", cfg.pin.app_name) } else { format!("Pin {}", cfg.pin.app_name) };
        menu.addItem(&item(&label, "pin", cfg, None, false));
        if a.pin.rect.is_some() {
            menu.addItem(&item("Reflow Pin", "reflow-pin", cfg, None, false));
        }
    }
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    let front = crate::engine::frontmost_pid().map(crate::engine::bundle_id).unwrap_or_default();
    if !front.is_empty() && front != "app.grids" {
        let name = crate::engine::running_app(crate::engine::frontmost_pid().unwrap_or(0))
            .and_then(|a| a.localizedName())
            .map(|s| s.to_string())
            .unwrap_or(front.clone());
        let ignored = cfg.general.ignored_apps.contains(&front);
        let it = plain(&format!("{} {name}", if ignored { "Unignore" } else { "Ignore" }), sel!(toggleIgnore:));
        unsafe { it.setRepresentedObject(Some(&NSString::from_str(&front))) };
        menu.addItem(&it);
    }
    menu.addItem(&plain("Settings…", sel!(openSettings:)));
    menu.addItem(&plain("Quit Grids", sel!(quit:)));
}

/// Show, hide or restyle the status item. Its menu is filled on demand by `will_open`.
pub fn rebuild() {
    let Some((hide, variant)) = app::try_with(|a| (a.cfg.menu.hide_icon, a.cfg.menu.icon_variant)) else {
        rebuild_later();
        return;
    };
    ITEM.with(|it| {
        let mut it = it.borrow_mut();
        if hide {
            if let Some(i) = it.take() {
                NSStatusBar::systemStatusBar().removeStatusItem(&i);
            }
            return;
        }
        let si = it.get_or_insert_with(|| {
            let si = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
            let menu = NSMenu::new(mtm());
            TARGET.with(|t| {
                if let Some(t) = t.borrow().as_deref() {
                    let _: () = unsafe { objc2::msg_send![&menu, setDelegate: t] };
                }
            });
            si.setMenu(Some(&menu));
            si
        });
        if let Some(b) = si.button(mtm()) {
            b.setImage(Some(&status_icon(variant)));
            b.setToolTip(Some(&NSString::from_str("Grids")));
        }
    });
}

/// NSMenuDelegate menuNeedsUpdate: for the status menu.
pub fn will_open(menu: &NSMenu) {
    CLEAR.with(|c| {
        if let Some(t) = c.borrow_mut().take() {
            t.cancel();
        }
    });
    app::try_with(|a| fill(menu, a));
}

/// NSMenuDelegate menuDidClose:. The chosen item's action is sent after this, so empty the
/// menu a moment later rather than now.
pub fn did_close(menu: &NSMenu) {
    let menu = menu.retain();
    let t = crate::timer::after(1.0, move || {
        menu.removeAllItems();
        CLEAR.with(|c| *c.borrow_mut() = None);
        crate::app::trim_memory();
    });
    CLEAR.with(|c| *c.borrow_mut() = Some(t));
}

pub fn rebuild_later() {
    crate::timer::after(0.0, rebuild);
}

/// Pop the menu at the cursor; its actions apply to `win` (or the frontmost window).
pub fn popup_at_cursor(win: Option<Window>) {
    let Some(menu) = app::try_with(|a| build(a)) else { return };
    MENU_WINDOW.with(|m| *m.borrow_mut() = win);
    let p = crate::tap::cursor();
    let ph = crate::screens::primary_height();
    menu.popUpMenuPositioningItem_atLocation_inView(None, NSPoint::new(p.x, ph - p.y), None);
    // Actions run from within the menu's tracking loop; clear afterwards.
    crate::timer::after(0.5, || MENU_WINDOW.with(|m| *m.borrow_mut() = None));
}

/// For `grids://debug`: open the status menu the way AppKit does (through its delegate)
/// without showing it, and report how many items it filled with.
pub fn probe() -> Option<usize> {
    let menu = ITEM.with(|it| it.borrow().as_ref().and_then(|si| si.menu(mtm())))?;
    let delegate = menu.delegate()?;
    let _: () = unsafe { objc2::msg_send![&*delegate, menuNeedsUpdate: &*menu] };
    let n = menu.numberOfItems() as usize;
    menu.removeAllItems();
    Some(n)
}
