//! System events: display changes (layout triggers, Dock adjustment, restoring a display's
//! windows on reconnect), wake, app activation, new windows, config reloads.

use crate::app;
use crate::ax;
use crate::config::Config;
use crate::ffi::*;
use crate::geom::Rect;
use crate::screens::Screen;
use crate::timer;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSApplicationDidChangeScreenParametersNotification, NSRunningApplication, NSWorkspace, NSWorkspaceApplicationKey, NSWorkspaceDidActivateApplicationNotification, NSWorkspaceDidLaunchApplicationNotification, NSWorkspaceDidWakeNotification};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectProtocol};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::time::SystemTime;

type Token = Retained<ProtocolObject<dyn NSObjectProtocol>>;

thread_local! {
    static TOKENS: RefCell<Vec<Token>> = const { RefCell::new(Vec::new()) };
    static LAST_SCREENS: RefCell<Vec<Screen>> = const { RefCell::new(Vec::new()) };
    /// Window frames per display UUID, recorded before displays reconfigure, for restoring on reconnect.
    static SNAPSHOT: RefCell<HashMap<String, Vec<(i32, u32, Rect)>>> = RefCell::new(HashMap::new());
    static CONFIG_MTIME: RefCell<Option<SystemTime>> = const { RefCell::new(None) };
    static CONFIG_WATCH: RefCell<Option<ConfigWatch>> = const { RefCell::new(None) };
    static LAST_SNAPSHOT: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    static OBSERVERS: RefCell<HashMap<i32, Cf>> = RefCell::new(HashMap::new());
}

fn app_from(n: NonNull<NSNotification>) -> Option<Retained<NSRunningApplication>> {
    let n = unsafe { n.as_ref() };
    let info = n.userInfo()?;
    let obj = info.objectForKey(unsafe { NSWorkspaceApplicationKey })?;
    obj.downcast::<NSRunningApplication>().ok()
}

fn observe(center: &NSNotificationCenter, name: &objc2_foundation::NSNotificationName, f: impl Fn(NonNull<NSNotification>) + 'static) {
    let block = RcBlock::new(f);
    let token = unsafe { center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block) };
    TOKENS.with(|t| t.borrow_mut().push(token));
}

pub fn start() {
    let ws_center = NSWorkspace::sharedWorkspace().notificationCenter();
    let center = NSNotificationCenter::defaultCenter();
    LAST_SCREENS.with(|l| *l.borrow_mut() = app::with(|a| a.screens.clone()));
    observe(&center, unsafe { NSApplicationDidChangeScreenParametersNotification }, |_| {
        // Displays settle over a few hundred ms; act once they have.
        timer::after(0.6, screens_changed);
    });
    observe(&ws_center, unsafe { NSWorkspaceDidWakeNotification }, |_| {
        timer::after(1.5, || {
            app::with(|a| {
                let cfg = a.cfg.clone();
                crate::tap::install(&cfg);
                crate::layouts::woke(a);
            })
        });
    });
    observe(&ws_center, unsafe { NSWorkspaceDidActivateApplicationNotification }, |n| {
        if let Some(app) = app_from(n) {
            let pid = app.processIdentifier();
            app::try_with(|a| crate::stash::app_activated(a, pid));
        }
        // Piggyback on activity the user is already causing, rather than waking on a timer.
        if crate::ffi::now() - LAST_SNAPSHOT.with(|l| l.get()) > 30.0 {
            snapshot();
        }
    });
    observe(&ws_center, unsafe { objc2_app_kit::NSWorkspaceWillSleepNotification }, |_| snapshot());
    observe(&ws_center, unsafe { NSWorkspaceDidLaunchApplicationNotification }, |n| {
        if let Some(app) = app_from(n) {
            let pid = app.processIdentifier();
            if app::try_with(|a| a.cfg.layouts.iter().any(|l| l.on_window_opens)).unwrap_or(false) {
                timer::after(0.5, move || watch_windows(pid));
            }
        }
    });

    // Settings asks for reloads; a kqueue watch on the file catches iCloud edits from another Mac.
    let name = cfstr("app.grids.reload");
    unsafe {
        CFNotificationCenterAddObserver(CFNotificationCenterGetDistributedCenter(), std::ptr::null(), reload_cb, name.ptr(), std::ptr::null(), 4);
    }
    let susp = cfstr("app.grids.suspend");
    let res = cfstr("app.grids.resume");
    unsafe {
        CFNotificationCenterAddObserver(CFNotificationCenterGetDistributedCenter(), 1 as *const c_void, suspend_cb, susp.ptr(), std::ptr::null(), 4);
        CFNotificationCenterAddObserver(CFNotificationCenterGetDistributedCenter(), 2 as *const c_void, resume_cb, res.ptr(), std::ptr::null(), 4);
    }
    CONFIG_MTIME.with(|m| *m.borrow_mut() = mtime());
    watch_config();
    // Displays announce a reconfiguration before anything moves: the moment to record where
    // windows are, instead of polling for it.
    unsafe { CGDisplayRegisterReconfigurationCallback(display_reconfig, std::ptr::null_mut()) };
    snapshot();
    window_watch_setup();
}

extern "C" fn display_reconfig(_d: u32, flags: u32, _u: *mut c_void) {
    if flags & kCGDisplayBeginConfigurationFlag != 0 && now() - LAST_SNAPSHOT.with(|l| l.get()) > 1.0 {
        snapshot();
    }
}

/// A kqueue on the config file and its folder (saves replace the file by rename), delivered
/// on the main run loop. Costs nothing until something writes.
struct ConfigWatch {
    cf: Cf,
    _files: Vec<std::fs::File>,
}

impl Drop for ConfigWatch {
    fn drop(&mut self) {
        unsafe { CFFileDescriptorInvalidate(self.cf.ptr()) };
    }
}

fn watch_config() {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    CONFIG_WATCH.with(|w| *w.borrow_mut() = None);
    let path = crate::config::active_path();
    let open = |p: &std::path::Path| std::fs::OpenOptions::new().read(true).custom_flags(O_EVTONLY).open(p).ok();
    let files: Vec<std::fs::File> = [path.parent().map(|p| p.to_path_buf()), Some(path.clone())].into_iter().flatten().filter_map(|p| open(&p)).collect();
    let kq = unsafe { kqueue() };
    if kq < 0 || files.is_empty() {
        return;
    }
    let changes: Vec<KEvent> = files
        .iter()
        .map(|f| KEvent {
            ident: f.as_raw_fd() as usize,
            filter: EVFILT_VNODE,
            flags: EV_ADD | EV_CLEAR,
            fflags: NOTE_WRITE | NOTE_EXTEND | NOTE_DELETE | NOTE_RENAME | NOTE_ATTRIB,
            data: 0,
            udata: std::ptr::null_mut(),
        })
        .collect();
    let zero = [0i64; 2];
    unsafe { kevent(kq, changes.as_ptr(), changes.len() as i32, std::ptr::null_mut(), 0, &zero) };
    let Some(cf) = Cf::own(unsafe { CFFileDescriptorCreate(kCFAllocatorDefault, kq, true, config_event, std::ptr::null()) }) else { return };
    unsafe {
        CFFileDescriptorEnableCallBacks(cf.ptr(), 1);
        let src = Cf(CFFileDescriptorCreateRunLoopSource(kCFAllocatorDefault, cf.ptr(), 0));
        CFRunLoopAddSource(CFRunLoopGetMain(), src.ptr(), kCFRunLoopCommonModes);
    }
    CONFIG_WATCH.with(|w| *w.borrow_mut() = Some(ConfigWatch { cf, _files: files }));
}

extern "C" fn config_event(_f: CFFileDescriptorRef, _t: usize, _i: *mut c_void) {
    // Saves arrive as a burst of events; settle, then re-arm (the file may have been replaced)
    // and reload if the contents' timestamp moved.
    timer::after(0.25, || {
        watch_config();
        let now = mtime();
        let changed = CONFIG_MTIME.with(|m| *m.borrow() != now);
        if changed {
            reload();
        }
    });
}

fn mtime() -> Option<SystemTime> {
    std::fs::metadata(crate::config::active_path()).and_then(|m| m.modified()).ok()
}

extern "C" fn reload_cb(_c: *const c_void, _o: *mut c_void, _n: CFStringRef, _obj: *const c_void, _i: CFDictionaryRef) {
    timer::after(0.0, reload);
}
extern "C" fn suspend_cb(_c: *const c_void, _o: *mut c_void, _n: CFStringRef, _obj: *const c_void, _i: CFDictionaryRef) {
    crate::hotkeys::unregister_all();
}
extern "C" fn resume_cb(_c: *const c_void, _o: *mut c_void, _n: CFStringRef, _obj: *const c_void, _i: CFDictionaryRef) {
    timer::after(0.0, || {
        app::try_with(|a| crate::hotkeys::register(&a.cfg));
    });
}

/// Re-read the config and re-arm everything that depends on it.
pub fn reload() {
    let cfg = Config::load();
    CONFIG_MTIME.with(|m| *m.borrow_mut() = mtime());
    let changed = app::with(|a| {
        if a.cfg == cfg {
            return false;
        }
        a.cfg = cfg.clone();
        a.refresh_screens();
        true
    });
    if !changed {
        return;
    }
    let failed = crate::hotkeys::register(&cfg);
    if !failed.is_empty() {
        eprintln!("grids: shortcuts not registered: {}", failed.join(", "));
    }
    crate::tap::install(&cfg);
    crate::touch::configure(cfg.throw.trackpad_fingers);
    crate::login::apply(cfg.general.launch_at_login);
    window_watch_setup();
    crate::menu::rebuild();
    // iCloud sync switches which file is live.
    watch_config();
}

fn snapshot() {
    LAST_SNAPSHOT.with(|l| l.set(now()));
    let enabled = app::try_with(|a| a.cfg.general.restore_display_layout).unwrap_or(false);
    if !enabled {
        return;
    }
    let Some(screens) = app::try_with(|a| a.screens.clone()) else { return };
    let mut map: HashMap<String, Vec<(i32, u32, Rect)>> = HashMap::new();
    for w in ax::onscreen_windows() {
        let i = crate::screens::index_for(&screens, &w.bounds);
        if let Some(s) = screens.get(i) {
            map.entry(s.uuid.clone()).or_default().push((w.pid, w.id, w.bounds));
        }
    }
    SNAPSHOT.with(|s| {
        let mut s = s.borrow_mut();
        // Keep entries for displays that are not connected now: those are what we restore.
        for (k, v) in map {
            s.insert(k, v);
        }
    });
}

fn screens_changed() {
    let Some((old, new)) = app::try_with(|a| {
        let old = LAST_SCREENS.with(|l| l.borrow().clone());
        a.refresh_screens();
        (old, a.screens.clone())
    }) else {
        return;
    };
    LAST_SCREENS.with(|l| *l.borrow_mut() = new.clone());
    let added: Vec<&Screen> = new.iter().filter(|s| !old.iter().any(|o| o.uuid == s.uuid)).collect();
    let removed: Vec<&Screen> = old.iter().filter(|s| !new.iter().any(|n| n.uuid == s.uuid)).collect();
    let added_names: Vec<String> = added.iter().map(|s| s.name.clone()).collect();
    let removed_names: Vec<String> = removed.iter().map(|s| s.name.clone()).collect();
    app::with(|a| {
        // Restore windows that lived on a display that just came back.
        if a.cfg.general.restore_display_layout {
            for s in &added {
                if let Some(list) = SNAPSHOT.with(|m| m.borrow().get(&s.uuid).cloned()) {
                    for (pid, id, r) in list {
                        if let Some(w) = ax::window_by_id(pid, id) {
                            w.set_frame(r);
                        }
                    }
                }
            }
        }
        if !added_names.is_empty() || !removed_names.is_empty() {
            crate::layouts::displays_changed(a, &added_names, &removed_names);
        }
        // Dock shown/hidden/moved: same displays, different visible frames.
        if a.cfg.general.adjust_for_dock && added.is_empty() && removed.is_empty() {
            for s in &new {
                if let Some(o) = old.iter().find(|o| o.uuid == s.uuid) {
                    if !o.visible.approx_eq(&s.visible, 0.5) {
                        adjust_for_dock(o.visible, s.visible);
                    }
                }
            }
        }
        if a.pin.rect.is_some() {
            crate::pin::reflow(a);
        }
    });
    snapshot();
}

/// Move window edges that were flush with the old visible frame to the new one.
fn adjust_for_dock(old: Rect, new: Rect) {
    let t = 3.0;
    for w in ax::onscreen_windows() {
        let b = w.bounds;
        if !b.intersects(&old) {
            continue;
        }
        let (mut l, mut r, mut top, mut bot) = (b.x, b.max_x(), b.y, b.max_y());
        if (b.x - old.x).abs() < t {
            l = new.x;
        }
        if (b.max_x() - old.max_x()).abs() < t {
            r = new.max_x();
        }
        if (b.y - old.y).abs() < t {
            top = new.y;
        }
        if (b.max_y() - old.max_y()).abs() < t {
            bot = new.max_y();
        }
        let n = Rect::new(l, top, r - l, bot - top);
        if !n.approx_eq(&b, 0.5) {
            if let Some(win) = ax::window_by_id(w.pid, w.id) {
                win.set_frame(n);
            }
        }
    }
}

extern "C" fn window_created(_o: AXObserverRef, el: AXUIElementRef, _n: CFStringRef, _r: *mut c_void) {
    let Some(el) = Cf::retain(el) else { return };
    let Some(w) = ax::Window::from_element(el) else { return };
    timer::after(0.25, move || {
        if w.is_standard() {
            app::with(|a| crate::layouts::window_opened(a, &w));
        }
    });
}

fn watch_windows(pid: i32) {
    if OBSERVERS.with(|o| o.borrow().contains_key(&pid)) {
        return;
    }
    let mut obs: AXObserverRef = std::ptr::null();
    if unsafe { AXObserverCreate(pid, window_created, &mut obs) } != kAXErrorSuccess {
        return;
    }
    let obs = Cf(obs);
    let app_el = ax::app_element(pid);
    unsafe {
        AXObserverAddNotification(obs.ptr(), app_el.ptr(), ax::attr("AXWindowCreated"), std::ptr::null_mut());
        CFRunLoopAddSource(CFRunLoopGetMain(), AXObserverGetRunLoopSource(obs.ptr()), kCFRunLoopCommonModes);
    }
    OBSERVERS.with(|o| o.borrow_mut().insert(pid, obs));
}

/// Window-open triggers need an observer per app; only pay for that when a layout uses it.
fn window_watch_setup() {
    let want = app::try_with(|a| a.cfg.layouts.iter().any(|l| l.on_window_opens)).unwrap_or(false);
    if !want {
        OBSERVERS.with(|o| {
            for (_, obs) in o.borrow_mut().drain() {
                unsafe { CFRunLoopRemoveSource(CFRunLoopGetMain(), AXObserverGetRunLoopSource(obs.ptr()), kCFRunLoopCommonModes) };
            }
        });
        return;
    }
    for app in NSWorkspace::sharedWorkspace().runningApplications().iter() {
        if app.activationPolicy() == objc2_app_kit::NSApplicationActivationPolicy::Regular {
            watch_windows(app.processIdentifier());
        }
    }
}
