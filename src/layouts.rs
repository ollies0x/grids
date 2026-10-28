//! Layouts: arrange a whole workspace, save the current one, and fire on display, wake and
//! window-open events. Also the shared "where would this command put a window" helpers.

use crate::app::{App, Last};
use crate::ax::{self, Window};
use crate::config::{Layout, LayoutEntry, Placement, Step};
use crate::engine;
use crate::geom::Rect;
use crate::screens;
use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};
use objc2_foundation::NSString;
use std::collections::HashSet;

/// The rect `cmd` would give a window on screen `idx` (for footprints).
pub fn command_rect(a: &App, cmd: &str, frame: Rect, wid: u32, idx: usize) -> Option<Rect> {
    if let Some(name) = cmd.strip_prefix("custom:") {
        let spec = a.cfg.custom(name)?;
        let f = if screens::index_for(&a.screens, &frame) == idx { frame } else { frame.centered_in(&a.screens[idx].visible) };
        return engine::step_target(a, spec.steps.first()?, f, wid, 0);
    }
    let cat = crate::actions::find(cmd).map(|x| x.cat);
    match cat {
        Some("Control" | "Stash" | "Pin" | "App" | "Multiple Windows" | "Display") | None => {
            Some(a.screens.get(idx)?.visible.inset(60.0, 60.0, 60.0, 60.0))
        }
        _ => engine::compute(a, cmd, frame, wid, idx, 0, None),
    }
}

/// Unit-square preview for icons (built-ins and custom entries).
pub fn preview_unit(a: &App, cmd: &str) -> Option<Rect> {
    if let Some(name) = cmd.strip_prefix("custom:") {
        let step = a.cfg.custom(name)?.steps.first()?;
        if let Some(c) = &step.command {
            return crate::actions::unit_preview(c);
        }
        let area = Rect::new(0.0, 0.0, 1600.0, 1000.0);
        let r = engine::place(step.place.as_ref()?, Rect::new(500.0, 300.0, 600.0, 400.0), area);
        return Some(area.to_unit(&r));
    }
    crate::actions::unit_preview(cmd)
}

fn lev(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            cur.push((prev[j] + (ca != *cb) as usize).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// 0 = no match, higher is better.
fn title_score(e: &LayoutEntry, title: &str) -> u32 {
    match e.title_mode.as_str() {
        "contains" => title.contains(&e.title) as u32 * 2,
        "regex" => regex_lite::Regex::new(&e.title).map(|r| r.is_match(title) as u32 * 2).unwrap_or(0),
        "loose" => {
            if title == e.title {
                3
            } else {
                let n = e.title.chars().count().max(title.chars().count()).max(1);
                if (lev(&e.title, title) as f64) / (n as f64) < 0.35 { 2 } else { 1 }
            }
        }
        _ => 1,
    }
}

struct Cand {
    win: Window,
    bundle: String,
    title: String,
}

fn candidates(frontmost_only: bool) -> Vec<Cand> {
    if frontmost_only {
        return ax::focused_window()
            .map(|w| vec![Cand { bundle: engine::bundle_id(w.pid), title: w.title(), win: w }])
            .unwrap_or_default();
    }
    let ws = NSWorkspace::sharedWorkspace();
    let mut out = Vec::new();
    for app in ws.runningApplications().iter() {
        if app.activationPolicy() != NSApplicationActivationPolicy::Regular {
            continue;
        }
        let pid = app.processIdentifier();
        let bundle = app.bundleIdentifier().map(|s| s.to_string()).unwrap_or_default();
        for w in ax::app_windows(pid) {
            if w.is_standard() {
                out.push(Cand { title: w.title(), bundle: bundle.clone(), win: w });
            }
        }
    }
    out
}

fn launch(bundle: &str) {
    let ws = NSWorkspace::sharedWorkspace();
    if let Some(url) = ws.URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle)) {
        let cfg = objc2_app_kit::NSWorkspaceOpenConfiguration::configuration();
        cfg.setActivates(false);
        ws.openApplicationAtURL_configuration_completionHandler(&url, &cfg, None);
    }
}

pub fn apply_named(a: &mut App, name: &str) {
    let Some(l) = a.cfg.layout(name).cloned() else { return };
    let cmd = format!("layout:{name}");
    let count = match &a.last {
        Some(last) if last.cmd == cmd && !(a.cfg.general.reset_repeat_on_release && a.released_since_last) => last.count + 1,
        _ => 0,
    };
    apply(a, &l, count, None);
    a.last = Some(Last { cmd, wid: 0, rect: Rect::default(), count });
    a.released_since_last = false;
}

/// Apply a layout. `only` limits it to one newly opened window.
pub fn apply(a: &mut App, l: &Layout, rep: usize, only: Option<&Window>) {
    a.refresh_screens();
    let mut cands = match only {
        Some(w) => vec![Cand { bundle: engine::bundle_id(w.pid), title: w.title(), win: w.clone() }],
        None => candidates(l.frontmost_only),
    };
    let mut used: HashSet<u32> = HashSet::new();
    let mut in_layout: HashSet<String> = HashSet::new();
    let mut launched = false;
    for e in &l.entries {
        if e.steps.is_empty() {
            continue;
        }
        in_layout.insert(e.app.clone());
        let step = &e.steps[rep % e.steps.len()];
        let mut matches: Vec<(u32, usize)> = cands
            .iter()
            .enumerate()
            .filter(|(_, c)| (e.app == "*" || c.bundle == e.app) && !used.contains(&c.win.id()))
            .map(|(i, c)| (title_score(e, &c.title), i))
            .filter(|(s, _)| *s > 0)
            .collect();
        matches.sort_by(|x, y| y.0.cmp(&x.0));
        if matches.is_empty() {
            if l.launch_apps && e.app != "*" && only.is_none() {
                let running = !NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(&e.app)).is_empty();
                if !running {
                    launch(&e.app);
                    launched = true;
                }
            }
            continue;
        }
        let take = if l.all_windows { matches.len() } else { 1 };
        for (_, i) in matches.into_iter().take(take) {
            let c = &cands[i];
            let wid = c.win.id();
            used.insert(wid);
            if c.win.minimized() && l.launch_apps {
                c.win.set_minimized(false);
            }
            let Some(frame) = c.win.frame() else { continue };
            if let Some(to) = engine::step_target(a, step, frame, wid, 0) {
                let cmd = step.command.clone().unwrap_or_default();
                engine::apply(a, &c.win, frame, to, &cmd);
            }
            if l.bring_to_front {
                c.win.raise();
            }
        }
    }
    if only.is_none() && l.others != "none" && !l.others.is_empty() {
        let ws = NSWorkspace::sharedWorkspace();
        let own = std::process::id() as i32;
        for app in ws.runningApplications().iter() {
            if app.activationPolicy() != NSApplicationActivationPolicy::Regular || app.processIdentifier() == own {
                continue;
            }
            let bundle = app.bundleIdentifier().map(|s| s.to_string()).unwrap_or_default();
            if in_layout.contains(&bundle) || in_layout.contains("*") {
                continue;
            }
            match l.others.as_str() {
                "hide" => {
                    app.hide();
                }
                "minimize" => {
                    for w in ax::app_windows(app.processIdentifier()) {
                        w.set_minimized(true);
                    }
                }
                "quit" if bundle != "com.apple.finder" => {
                    app.terminate();
                }
                _ => {}
            }
        }
    }
    cands.clear();
    if launched {
        // Give launched apps a moment to open windows, then place them.
        let l = l.clone();
        crate::timer::after(2.5, move || crate::app::with(|a| apply(a, &Layout { launch_apps: false, ..l }, rep, None)));
    }
}

/// Save every window on the focused window's display as a new layout.
pub fn save_current(a: &mut App) {
    a.refresh_screens();
    let idx = ax::focused_window().and_then(|w| w.frame()).map(|f| screens::index_for(&a.screens, &f)).unwrap_or(0);
    let s = a.screens[idx].clone();
    let area = screens::usable(&s, &a.cfg, a.pin.rect.as_ref());
    let mut entries = Vec::new();
    for info in ax::onscreen_windows() {
        if screens::index_for(&a.screens, &info.bounds) != idx {
            continue;
        }
        let Some(w) = ax::window_by_id(info.pid, info.id).filter(|w| w.is_standard()) else { continue };
        let f = info.bounds;
        let preset = a
            .hist
            .get(&info.id)
            .filter(|h| a.cfg.layout_save_as_presets && !h.cmd.is_empty() && h.rect.approx_eq(&f, 4.0))
            .map(|h| h.cmd.clone());
        let step = Step {
            command: preset.clone(),
            place: preset.is_none().then(|| {
                let u = area.to_unit(&f);
                Placement { position: "custom".into(), x: Some(u.x.clamp(0.0, 1.0)), y: Some(u.y.clamp(0.0, 1.0)), w: Some(u.w.min(1.0)), h: Some(u.h.min(1.0)) }
            }),
            display: format!("name:{}", s.name),
        };
        entries.push(LayoutEntry {
            app: engine::bundle_id(info.pid),
            app_name: info.owner.clone(),
            steps: vec![step],
            snap_target: false,
            title_mode: "loose".into(),
            title: w.title(),
        });
    }
    entries.reverse(); // back to front, so applying raises in the original stacking order
    let mut n = a.cfg.layouts.len() + 1;
    while a.cfg.layouts.iter().any(|l| l.name == format!("Layout {n}")) {
        n += 1;
    }
    let name = format!("Layout {n}");
    a.cfg.layouts.push(Layout { name: name.clone(), others: "none".into(), entries, ..Default::default() });
    let _ = a.cfg.save();
    crate::config::announce_change();
    crate::settings::open(Some(&format!("layouts/{name}")));
}

/// Displays changed: fire layouts whose connect/disconnect trigger matches.
pub fn displays_changed(a: &mut App, added: &[String], removed: &[String]) {
    let layouts = a.cfg.layouts.clone();
    for l in layouts {
        let hit = |want: &Option<String>, set: &[String]| want.as_ref().is_some_and(|w| !set.is_empty() && (w == "*" || set.iter().any(|n| n == w)));
        if hit(&l.on_display_connected, added) || hit(&l.on_display_disconnected, removed) {
            apply(a, &l, 0, None);
        }
    }
}

pub fn woke(a: &mut App) {
    let layouts: Vec<Layout> = a.cfg.layouts.iter().filter(|l| l.on_wake).cloned().collect();
    for l in layouts {
        apply(a, &l, 0, None);
    }
}

pub fn window_opened(a: &mut App, w: &Window) {
    let layouts: Vec<Layout> = a.cfg.layouts.iter().filter(|l| l.on_window_opens).cloned().collect();
    for l in layouts {
        apply(a, &l, 0, Some(w));
    }
}
