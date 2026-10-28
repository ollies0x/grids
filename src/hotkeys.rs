//! Global shortcuts through Carbon's RegisterEventHotKey: no event tap, no keystroke
//! visibility, and the system delivers press and release separately (used for hold-repeat).

use crate::config::{Config, Shortcut};
use crate::ffi::*;
use crate::keys::{self, carbon_mods};
use crate::timer::{self, Timer};
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr;

struct Registered {
    r: EventHotKeyRef,
    cmd: String,
}

thread_local! {
    static KEYS: RefCell<Vec<Registered>> = const { RefCell::new(Vec::new()) };
    static HOLD: RefCell<Option<Timer>> = const { RefCell::new(None) };
    static INSTALLED: RefCell<bool> = const { RefCell::new(false) };
}

const SIG: u32 = u32::from_be_bytes(*b"GRID");

extern "C" fn handler(_call: EventHandlerCallRef, ev: EventRef, _user: *mut c_void) -> OSStatus {
    let mut id = EventHotKeyID::default();
    let st = unsafe {
        GetEventParameter(
            ev,
            kEventParamDirectObject,
            typeEventHotKeyID,
            ptr::null_mut(),
            std::mem::size_of::<EventHotKeyID>(),
            ptr::null_mut(),
            &mut id as *mut _ as *mut c_void,
        )
    };
    if st != 0 || id.signature != SIG {
        return -9874; // eventNotHandledErr
    }
    let kind = unsafe { GetEventKind(ev) };
    let cmd = KEYS.with(|k| k.borrow().get(id.id as usize).map(|r| r.cmd.clone()));
    let Some(cmd) = cmd else { return 0 };
    if kind == kEventHotKeyPressed {
        crate::tap::note_hotkey();
        fire(&cmd);
        if crate::actions::holdable(&cmd) {
            let c = cmd.clone();
            let t = timer::after(0.35, move || {
                let t = timer::every(0.045, move || fire(&c));
                HOLD.with(|h| *h.borrow_mut() = Some(t));
            });
            HOLD.with(|h| {
                if let Some(old) = h.borrow_mut().replace(t) {
                    old.cancel();
                }
            });
        }
    } else if kind == kEventHotKeyReleased {
        HOLD.with(|h| {
            if let Some(t) = h.borrow_mut().take() {
                t.cancel();
            }
        });
    }
    0
}

fn fire(cmd: &str) {
    let cmd = cmd.to_string();
    // Hotkey events arrive outside any state borrow, so run directly.
    crate::app::try_with(|a| crate::engine::run(a, &cmd, crate::engine::Target::Front));
}

fn install() {
    INSTALLED.with(|i| {
        if *i.borrow() {
            return;
        }
        let types = [
            EventTypeSpec { event_class: kEventClassKeyboard, event_kind: kEventHotKeyPressed },
            EventTypeSpec { event_class: kEventClassKeyboard, event_kind: kEventHotKeyReleased },
        ];
        let mut h: EventHandlerRef = ptr::null();
        unsafe { InstallEventHandler(GetApplicationEventTarget(), handler, 2, types.as_ptr(), ptr::null_mut(), &mut h) };
        *i.borrow_mut() = true;
    });
}

pub fn unregister_all() {
    KEYS.with(|k| {
        for r in k.borrow_mut().drain(..) {
            if !r.r.is_null() {
                unsafe { UnregisterEventHotKey(r.r) };
            }
        }
    });
}

/// (Re)register every shortcut in the config. Returns commands whose shortcut was refused
/// (usually taken by the system or another app).
pub fn register(cfg: &Config) -> Vec<String> {
    install();
    unregister_all();
    let mut list: Vec<(String, Shortcut)> = cfg.shortcuts.iter().filter(|(_, s)| s.key_code != 0 || s.modifier_flags != 0).map(|(c, s)| (c.clone(), *s)).collect();
    if cfg.win_arrow_modifiers != 0 {
        for (dir, code) in [("left", keys::K_LEFT), ("right", keys::K_RIGHT), ("up", keys::K_UP), ("down", keys::K_DOWN)] {
            list.push((format!("win-{dir}"), Shortcut { key_code: code, modifier_flags: cfg.win_arrow_modifiers }));
        }
    }
    let mut failed = Vec::new();
    KEYS.with(|k| {
        let mut k = k.borrow_mut();
        for (cmd, sc) in list {
            // Without "allow any shortcut", refuse bare keys that would swallow typing.
            if !cfg.general.allow_any_shortcut && sc.modifier_flags & (crate::config::CMD | crate::config::CTRL | crate::config::OPT) == 0 && !(122..=126).contains(&sc.key_code) && sc.key_code < 96 {
                failed.push(cmd);
                continue;
            }
            let id = EventHotKeyID { signature: SIG, id: k.len() as u32 };
            let mut r: EventHotKeyRef = ptr::null();
            let st = unsafe { RegisterEventHotKey(sc.key_code, carbon_mods(sc.modifier_flags), id, GetApplicationEventTarget(), 0, &mut r) };
            if st == 0 {
                k.push(Registered { r, cmd });
            } else {
                failed.push(cmd);
                // Keep ids dense: push a placeholder that never fires.
                k.push(Registered { r: ptr::null(), cmd: String::new() });
            }
        }
    });
    failed
}
