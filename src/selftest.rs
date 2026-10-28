//! `Grids --selftest`: drives every single-window action against a throwaway window in a
//! child process (never the user's windows) and checks where it lands. Needs Accessibility.

use crate::actions::{self, ACTIONS};
use crate::app;
use crate::ax;
use crate::engine::{self, Target};
use crate::geom::Rect;
use crate::screens;
use objc2::MainThreadMarker;
use objc2::MainThreadOnly;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::process::{Command, Stdio};
use std::time::Instant;

pub const TITLE: &str = "Grids Self-Test";

struct TestApp(i32);

impl Drop for TestApp {
    fn drop(&mut self) {
        if let Some(app) = objc2_app_kit::NSRunningApplication::runningApplicationWithProcessIdentifier(self.0) {
            app.forceTerminate();
        }
    }
}

/// Child mode: show one ordinary window and idle.
pub fn test_window() -> ! {
    let mtm = MainThreadMarker::new().unwrap();
    let nsapp = NSApplication::sharedApplication(mtm);
    // A regular app: accessory apps don't publish their windows to Accessibility until
    // activated, and the test must not steal focus.
    nsapp.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Resizable | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable;
    let w = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(640.0, 420.0)),
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { w.setReleasedWhenClosed(false) };
    w.setTitle(&NSString::from_str(TITLE));
    w.setMinSize(NSSize::new(80.0, 60.0));
    w.orderFrontRegardless();
    nsapp.run();
    std::process::exit(0)
}

fn wait_window(pid: i32) -> Option<ax::Window> {
    for _ in 0..100 {
        if let Some(w) = ax::app_windows(pid).into_iter().find(|w| w.title() == TITLE) {
            return Some(w);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    None
}

/// Print and also leave the result where a LaunchServices-launched run can be read.
fn report(s: &str) {
    println!("{s}");
    let _ = std::fs::create_dir_all(crate::config::support_dir());
    let _ = std::fs::write(crate::config::support_dir().join("selftest.txt"), format!("{s}\n"));
}

pub fn run() -> i32 {
    if !ax::trusted(true) {
        report("selftest: Grids is not trusted for Accessibility; grant it in System Settings first");
        return 2;
    }
    let mut cfg = crate::config::Config::default();
    cfg.general.gap = 0.0;
    cfg.custom.push(crate::config::CustomSpec {
        name: "Centre 60".into(),
        steps: vec![
            crate::config::Step {
                command: None,
                place: Some(crate::config::Placement { position: "center".into(), w: Some(0.6), h: Some(0.6), ..Default::default() }),
                display: String::new(),
            },
            crate::config::Step { command: Some("left-half".into()), place: None, display: String::new() },
        ],
        snap_target: false,
    });
    app::init(cfg);
    // The test window is a separate tiny app launched through LaunchServices: windows of
    // processes we spawn directly are not served to Accessibility.
    let exe = std::env::current_exe().unwrap();
    let tw = exe.ancestors().map(|d| d.join("build/obj/TestWindow.app")).find(|p| p.exists());
    let Some(tw) = tw else {
        report("selftest: build/obj/TestWindow.app not found; run scripts/build.sh");
        return 2;
    };
    let _ = Command::new("open").arg("-g").arg("-n").arg(&tw).args(["--args", "--background"]).stdout(Stdio::null()).status();
    let bundle = objc2_foundation::NSString::from_str("app.grids.testwindow");
    let mut pid = 0;
    for _ in 0..60 {
        if let Some(app) = objc2_app_kit::NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle).iter().next() {
            pid = app.processIdentifier();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let child = TestApp(pid);
    let Some(win) = wait_window(pid) else {
        report("selftest: test window never appeared");
        return 2;
    };
    let mut pass = 0;
    let mut fail = Vec::new();
    let mut times = Vec::new();
    let skip_cats = ["Display", "Control", "Multiple Windows", "Stash", "Pin", "App"];
    let start = Rect::new(300.0, 200.0, 640.0, 420.0);

    app::with(|a| {
        a.refresh_screens();
        let main = a.screens.iter().position(|s| s.main).unwrap_or(0);
        let home = start.centered_in(&a.screens[main].visible);
        for act in ACTIONS.iter().filter(|x| !skip_cats.contains(&x.cat) && !matches!(x.id, "restore" | "last")) {
            win.set_frame(home);
            a.last = None;
            a.hist.clear();
            let before = win.frame().unwrap();
            let idx = screens::index_for(&a.screens, &before);
            let expect = engine::compute(a, act.id, before, win.id(), idx, 0, None);
            let t = Instant::now();
            engine::run(a, act.id, Target::Window(win.clone()));
            times.push(t.elapsed().as_secs_f64() * 1000.0);
            let after = win.frame().unwrap();
            match expect {
                Some(e) if after.approx_eq(&e.round(), 2.0) => pass += 1,
                Some(e) => fail.push(format!("{:<26} expected {:?} got {:?}", act.id, e.round(), after)),
                None => fail.push(format!("{:<26} no geometry", act.id)),
            }
        }

        // Repeat cycling: First Third three times walks across the screen.
        win.set_frame(home);
        a.last = None;
        let mut xs = Vec::new();
        for _ in 0..3 {
            engine::run(a, "first-third", Target::Window(win.clone()));
            xs.push(win.frame().unwrap().x);
        }
        if xs[0] < xs[1] && xs[1] < xs[2] { pass += 1 } else { fail.push(format!("first-third repeat cycle x={xs:?}")) }

        // Half cycling in cycleSizes mode: ½ then ⅔.
        win.set_frame(home);
        a.last = None;
        engine::run(a, "left-half", Target::Window(win.clone()));
        let w1 = win.frame().unwrap().w;
        engine::run(a, "left-half", Target::Window(win.clone()));
        let w2 = win.frame().unwrap().w;
        if w2 > w1 * 1.2 { pass += 1 } else { fail.push(format!("left-half size cycle {w1} -> {w2}")) }

        // Restore and Last.
        win.set_frame(home);
        a.last = None;
        a.hist.clear();
        let orig = win.frame().unwrap();
        engine::run(a, "maximize", Target::Window(win.clone()));
        engine::run(a, "left-half", Target::Window(win.clone()));
        engine::run(a, "last", Target::Window(win.clone()));
        let after_last = win.frame().unwrap();
        let max = engine::compute(a, "maximize", orig, win.id(), main, 0, None).unwrap();
        if after_last.approx_eq(&max, 2.0) { pass += 1 } else { fail.push(format!("last: expected maximize {max:?} got {after_last:?}")) }
        engine::run(a, "restore", Target::Window(win.clone()));
        if win.frame().unwrap().approx_eq(&orig, 2.0) { pass += 1 } else { fail.push(format!("restore: expected {orig:?} got {:?}", win.frame())) }

        // Custom entry with a repeat step.
        win.set_frame(home);
        a.last = None;
        engine::run(a, "custom:Centre 60", Target::Window(win.clone()));
        let c1 = win.frame().unwrap();
        let vis = screens::usable(&a.screens[main], &a.cfg, None);
        if (c1.w - vis.w * 0.6).abs() < 2.0 && (c1.mid_x() - vis.mid_x()).abs() < 2.0 { pass += 1 } else { fail.push(format!("custom step 1 got {c1:?}")) }
        engine::run(a, "custom:Centre 60", Target::Window(win.clone()));
        let c2 = win.frame().unwrap();
        if (c2.x - vis.x).abs() < 2.0 && (c2.w - vis.w / 2.0).abs() < 2.0 { pass += 1 } else { fail.push(format!("custom repeat got {c2:?}")) }

        // Stash left and back.
        win.set_frame(home);
        a.cfg.stash.animate = false;
        a.cfg.stash.color_tabs = false;
        engine::run(a, "stash-left", Target::Window(win.clone()));
        let st = win.frame().unwrap();
        let sf = a.screens[main].frame;
        let visible_w = st.intersect(&sf).w;
        if visible_w <= 40.0 { pass += 1 } else { fail.push(format!("stash-left left {visible_w}px visible ({st:?})")) }
        engine::run(a, "unstash", Target::Window(win.clone()));
        if win.frame().unwrap().approx_eq(&home, 2.0) { pass += 1 } else { fail.push(format!("unstash got {:?}", win.frame())) }

        // Next display, when there is one.
        if a.screens.len() > 1 {
            win.set_frame(home);
            a.last = None;
            engine::run(a, "maximize", Target::Window(win.clone()));
            engine::run(a, "next-display", Target::Window(win.clone()));
            let f = win.frame().unwrap();
            let i = screens::index_for(&a.screens, &f);
            if i != main { pass += 1 } else { fail.push("next-display stayed on the same screen".into()) }
        }
    });
    drop(child);
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = times.get(times.len() / 2).copied().unwrap_or(0.0);
    let p95 = times.get(times.len() * 95 / 100).copied().unwrap_or(0.0);
    let mut out = format!("selftest: {pass} passed, {} failed; per action median {med:.1} ms, p95 {p95:.1} ms over {} actions", fail.len(), times.len());
    for f in &fail {
        out.push_str(&format!("\n  FAIL {f}"));
    }
    report(&out);
    let _ = actions::find;
    if fail.is_empty() { 0 } else { 1 }
}
