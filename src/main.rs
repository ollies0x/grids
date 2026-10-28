//! Grids: a window manager for macOS in Rust. Rectangle's actions plus the Rectangle Pro
//! feature set, as a menu bar app with no Swift runtime and no background helpers.

mod actions;
mod app;
mod ax;
mod config;
mod engine;
mod ffi;
mod geom;
mod hotkeys;
mod import;
mod keys;
mod layouts;
mod login;
mod menu;
mod multi;
mod overlay;
mod pin;
mod screens;
mod selftest;
mod settings;
mod snap;
mod stash;
mod tap;
mod throw;
mod timer;
mod touch;
mod watchers;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSMenu, NSMenuDelegate, NSMenuItem};
use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventManager, NSString};

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "GridsDelegate"]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl NSApplicationDelegate for Delegate {
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn reopen(&self, _app: &NSApplication, _visible: bool) -> bool {
            // Launching Grids again while it runs (e.g. with the icon hidden) opens Settings.
            settings::open(None);
            true
        }
    }

    unsafe impl NSMenuDelegate for Delegate {
        #[unsafe(method(menuNeedsUpdate:))]
        fn menu_needs_update(&self, menu: &NSMenu) {
            menu::will_open(menu);
        }

        #[unsafe(method(menuDidClose:))]
        fn menu_did_close(&self, menu: &NSMenu) {
            menu::did_close(menu);
        }
    }

    impl Delegate {
        #[unsafe(method(menuAction:))]
        fn menu_action(&self, sender: &NSMenuItem) {
            let Some(cmd) = represented(sender) else { return };
            let win = menu::MENU_WINDOW.with(|m| m.borrow_mut().take());
            let target = win.map(engine::Target::Window).unwrap_or(engine::Target::Front);
            app::with(|a| engine::run(a, &cmd, target));
        }

        #[unsafe(method(toggleIgnore:))]
        fn toggle_ignore(&self, sender: &NSMenuItem) {
            let Some(bundle) = represented(sender) else { return };
            app::with(|a| {
                let list = &mut a.cfg.general.ignored_apps;
                if let Some(i) = list.iter().position(|b| *b == bundle) {
                    list.remove(i);
                } else {
                    list.push(bundle);
                }
                let _ = a.cfg.save();
            });
            config::announce_change();
        }

        #[unsafe(method(openSettings:))]
        fn open_settings(&self, _sender: &NSMenuItem) {
            settings::open(None);
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: &NSMenuItem) {
            std::process::exit(0);
        }

        #[unsafe(method(handleGetURL:withReplyEvent:))]
        fn handle_url(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            let key = u32::from_be_bytes(*b"----");
            let d: Option<Retained<NSAppleEventDescriptor>> = unsafe { msg_send![event, paramDescriptorForKeyword: key] };
            let Some(url) = d.and_then(|d| d.stringValue()) else { return };
            handle_url(&url.to_string());
        }
    }
);

impl Delegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

fn represented(item: &NSMenuItem) -> Option<String> {
    let obj = item.representedObject()?;
    obj.downcast::<NSString>().ok().map(|s| s.to_string())
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 3 <= b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// grids://execute-action?name=left-half (also rectangle-pro:// and rectangle://),
/// execute-layout?name=…, execute-custom?name=…, settings, reload.
fn handle_url(url: &str) {
    let rest = url.split_once("://").map(|x| x.1).unwrap_or(url);
    let (verb, query) = rest.split_once('?').unwrap_or((rest, ""));
    let param = |key: &str| query.split('&').filter_map(|kv| kv.split_once('=')).find(|(k, _)| *k == key).map(|(_, v)| percent_decode(v));
    let name = param("name").unwrap_or_default();
    // Optional explicit target: &window=<CGWindowID>, otherwise the front window.
    let window: Option<u32> = param("window").and_then(|w| w.parse().ok());
    let cmd = match verb.trim_end_matches('/') {
        "execute-action" => name,
        "execute-layout" => format!("layout:{name}"),
        "execute-custom" => format!("custom:{name}"),
        "settings" => {
            settings::open(if name.is_empty() { None } else { Some(&name) });
            return;
        }
        "reload" => {
            watchers::reload();
            return;
        }
        "debug" => {
            timer::after(0.0, || {
                let menu_items = menu::probe();
                app::with(|a| debug_dump(a, menu_items))
            });
            return;
        }
        other => {
            eprintln!("grids: unknown URL verb {other}");
            return;
        }
    };
    if cmd.is_empty() {
        return;
    }
    timer::after(0.0, move || {
        let target = match window {
            Some(id) => {
                let Some(pid) = ax::owner_of(id) else { return };
                match ax::window_by_id(pid, id) {
                    Some(w) => engine::Target::Window(w),
                    None => return,
                }
            }
            None => engine::Target::Front,
        };
        app::with(|a| engine::run(a, &cmd, target))
    });
}

/// grids://debug writes what Grids remembers to support/debug.json.
fn debug_dump(a: &app::App, menu_items: Option<usize>) {
    let wins: Vec<serde_json::Value> = ax::onscreen_windows()
        .into_iter()
        .filter_map(|w| {
            let h = a.hist.get(&w.id)?;
            Some(serde_json::json!({
                "id": w.id, "owner": w.owner, "now": [w.bounds.x, w.bounds.y, w.bounds.w, w.bounds.h],
                "original": h.original.map(|r| [r.x, r.y, r.w, r.h]), "last": h.cmd, "steps": h.stack.len(),
            }))
        })
        .collect();
    let v = serde_json::json!({
        "windows": wins,
        "historyEntries": a.hist.len(),
        "last": a.last.as_ref().map(|l| serde_json::json!({"cmd": l.cmd, "wid": l.wid, "count": l.count})),
        "stashed": a.stash.entries.len(),
        "pinned": a.pin.rect.is_some(),
        "gap": a.cfg.general.gap,
        "menuItems": menu_items,
        "screens": a.screens.iter().map(|s| serde_json::json!({"name": s.name, "frame": [s.frame.x, s.frame.y, s.frame.w, s.frame.h]})).collect::<Vec<_>>(),
    });
    let _ = std::fs::write(config::support_dir().join("debug.json"), serde_json::to_vec_pretty(&v).unwrap());
}

/// JSON the Settings app reads: the action catalog with icon geometry, and defaults.
fn catalog() -> String {
    let acts: Vec<serde_json::Value> = actions::ACTIONS
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "name": a.name,
                "category": a.cat,
                "holdable": actions::holdable(a.id),
                "unit": actions::unit_preview(a.id).map(|u| [u.x, u.y, u.w, u.h]),
            })
        })
        .collect();
    serde_json::json!({
        "actions": acts,
        "categories": actions::CATEGORIES,
        "defaults": config::Config::default(),
        "configPath": config::active_path(),
        "version": env!("CARGO_PKG_VERSION"),
    })
    .to_string()
}

/// Lets the Settings app show whether the daemon has Accessibility access.
fn mark_trusted(on: bool) {
    let p = config::support_dir().join(".trusted");
    if on {
        let _ = std::fs::create_dir_all(config::support_dir());
        let _ = std::fs::write(p, b"");
    } else {
        let _ = std::fs::remove_file(p);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--catalog") {
        println!("{}", catalog());
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--ax-probe") {
        let pid: i32 = args.get(i + 1).and_then(|p| p.parse().ok()).unwrap_or(0);
        let ws = ax::app_windows(pid);
        println!("pid {pid}: {} windows {:?}", ws.len(), ws.iter().map(|w| (w.title(), w.role(), w.subrole(), w.frame(), w.id())).collect::<Vec<_>>());
        return;
    }
    if args.iter().any(|a| a == "--test-window") {
        selftest::test_window();
    }
    if args.iter().any(|a| a == "--selftest") {
        let _mtm = MainThreadMarker::new().expect("main thread");
        let _ = NSApplication::sharedApplication(_mtm);
        std::process::exit(selftest::run());
    }
    if args.iter().any(|a| a == "--version") {
        println!("grids {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let mtm = MainThreadMarker::new().expect("main thread");
    let nsapp = NSApplication::sharedApplication(mtm);
    nsapp.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let first_run = !config::active_path().exists();
    let cfg = config::Config::load();
    if let Err(e) = cfg.save() {
        eprintln!("grids: cannot write config: {e}");
    }
    app::init(cfg.clone());

    let delegate = Delegate::new(mtm);
    nsapp.setDelegate(Some(objc2::runtime::ProtocolObject::from_ref(&*delegate)));
    let target: Retained<AnyObject> = delegate.clone().into();
    menu::set_target(target);

    let gurl = u32::from_be_bytes(*b"GURL");
    unsafe {
        let mgr = NSAppleEventManager::sharedAppleEventManager();
        let _: () = msg_send![&mgr, setEventHandler: &*delegate, andSelector: sel!(handleGetURL:withReplyEvent:), forEventClass: gurl, andEventID: gurl];
    }

    let failed = hotkeys::register(&cfg);
    if !failed.is_empty() {
        eprintln!("grids: shortcuts not registered: {}", failed.join(", "));
    }
    menu::rebuild();
    watchers::start();
    touch::configure(cfg.throw.trackpad_fingers);
    login::apply(cfg.general.launch_at_login);

    // Taps need Accessibility; ask once, then wait for the grant.
    if ax::trusted(true) {
        mark_trusted(true);
        tap::install(&cfg);
    } else {
        mark_trusted(false);
        let t = std::rc::Rc::new(std::cell::RefCell::new(None::<timer::Timer>));
        let t2 = t.clone();
        *t.borrow_mut() = Some(timer::every(1.0, move || {
            if ax::trusted(false) {
                mark_trusted(true);
                let cfg = app::with(|a| a.cfg.clone());
                tap::install(&cfg);
                if let Some(t) = t2.borrow_mut().take() {
                    t.cancel();
                }
            }
        }));
    }
    if first_run {
        settings::open(Some("welcome"));
    }
    // Launch leaves a few MB of freed pages behind; give them back once things settle.
    timer::after(3.0, app::trim_memory);
    let _keep = delegate;
    nsapp.run();
}
