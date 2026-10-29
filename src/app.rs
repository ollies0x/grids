//! Process-wide state, owned by the main thread.

use crate::config::Config;
use crate::geom::Rect;
use crate::screens::{self, Screen};
use std::cell::RefCell;
use std::collections::HashMap;

/// What Grids remembers about a window it has moved.
#[derive(Clone, Debug, Default)]
pub struct WinHist {
    /// Frame before Grids first touched it (Restore).
    pub original: Option<Rect>,
    /// Frames before each action, newest last (Last).
    pub stack: Vec<Rect>,
    /// The last geometry command applied and where it put the window.
    pub cmd: String,
    pub rect: Rect,
    /// Placed by a drag-to-snap, so dragging it away restores `original` size.
    pub snapped: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Last {
    pub cmd: String,
    pub wid: u32,
    pub rect: Rect,
    pub count: usize,
}

pub struct App {
    pub cfg: Config,
    pub screens: Vec<Screen>,
    pub hist: HashMap<u32, WinHist>,
    pub last: Option<Last>,
    /// Set when all modifiers were released since the last action.
    pub released_since_last: bool,
    pub pin: crate::pin::PinState,
    pub stash: crate::stash::StashState,
    /// Frames before the last tile/grid action, for Undo Grid.
    pub grid_undo: Option<Vec<(i32, u32, Rect)>>,
    pub reveal: Option<Vec<(i32, u32, Rect)>>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

pub fn init(cfg: Config) {
    let screens = screens::all(cfg.tweaks.screens_ordered_by_x);
    APP.with(|a| {
        *a.borrow_mut() = Some(App {
            cfg,
            screens,
            hist: HashMap::new(),
            last: None,
            released_since_last: true,
            pin: Default::default(),
            stash: Default::default(),
            grid_undo: None,
            reveal: None,
        })
    });
}

/// Run `f` with the state. Not re-entrant: anything that can spin a nested run loop (menus,
/// alerts) must be deferred with `timer::after(0.0, ..)` rather than called inside.
pub fn with<R>(f: impl FnOnce(&mut App) -> R) -> R {
    APP.with(|a| {
        let mut g = a.try_borrow_mut().expect("grids state re-entered");
        f(g.as_mut().expect("state initialised"))
    })
}

/// Like `with`, but skips (returns None) when the state is already borrowed further up the
/// stack, for callbacks that can arrive while an action is running.
pub fn try_with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| {
        let mut g = a.try_borrow_mut().ok()?;
        g.as_mut().map(f)
    })
}

impl App {
    pub fn refresh_screens(&mut self) {
        self.screens = screens::all(self.cfg.tweaks.screens_ordered_by_x);
    }
}

unsafe extern "C" {
    fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
}

/// Hand freed malloc pages back to the system. Launch, menus and Settings round trips leave
/// a few MB of empty pages that otherwise count against the footprint until memory pressure.
pub fn trim_memory() {
    unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) };
}
