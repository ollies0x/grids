//! Trackpad trigger for the Window Throw: rest N fingers, lift all but one, move it, lift it.
//! Uses the private MultitouchSupport framework, loaded only when the gesture is configured.

use std::ffi::{c_void, CString};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;

type DeviceRef = *const c_void;
type ContactCb = extern "C" fn(DeviceRef, *const c_void, i32, f64, i32) -> i32;

static FINGERS: AtomicU8 = AtomicU8::new(0);
static STATE: AtomicUsize = AtomicUsize::new(0); // 0 idle, 1 armed, 2 throwing
static ARMED_AT: Mutex<f64> = Mutex::new(0.0);
static STARTED: Mutex<bool> = Mutex::new(false);

unsafe extern "C" {
    fn dlopen(path: *const i8, mode: i32) -> *mut c_void;
    fn dlsym(h: *mut c_void, name: *const i8) -> *mut c_void;
    fn dispatch_async_f(q: *const c_void, ctx: *mut c_void, f: extern "C" fn(*mut c_void));
    static _dispatch_main_q: c_void;
}

extern "C" fn on_main(ctx: *mut c_void) {
    crate::tap::gesture_throw(ctx as usize == 1);
}

fn to_main(begin: bool) {
    unsafe { dispatch_async_f(&_dispatch_main_q as *const c_void, begin as usize as *mut c_void, on_main) };
}

fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

extern "C" fn contact(_dev: DeviceRef, _touches: *const c_void, n: i32, _ts: f64, _frame: i32) -> i32 {
    let want = FINGERS.load(Ordering::Relaxed) as i32;
    if want == 0 {
        return 0;
    }
    match STATE.load(Ordering::Relaxed) {
        0 if n == want => {
            *ARMED_AT.lock().unwrap() = now();
            STATE.store(1, Ordering::Relaxed);
        }
        1 if n == 1 => {
            // Fingers must have rested together briefly, then lifted to one.
            if now() - *ARMED_AT.lock().unwrap() >= 0.05 {
                STATE.store(2, Ordering::Relaxed);
                to_main(true);
            } else {
                STATE.store(0, Ordering::Relaxed);
            }
        }
        1 if n == 0 || n > want => STATE.store(0, Ordering::Relaxed),
        2 if n == 0 => {
            STATE.store(0, Ordering::Relaxed);
            to_main(false);
        }
        2 if n > 1 => {
            STATE.store(0, Ordering::Relaxed);
            to_main(false);
        }
        _ => {}
    }
    0
}

pub fn configure(fingers: Option<u8>) {
    FINGERS.store(fingers.filter(|f| (3..=5).contains(f)).unwrap_or(0), Ordering::Relaxed);
    if FINGERS.load(Ordering::Relaxed) == 0 {
        return;
    }
    let mut started = STARTED.lock().unwrap();
    if *started {
        return;
    }
    unsafe {
        let path = CString::new("/System/Library/PrivateFrameworks/MultitouchSupport.framework/MultitouchSupport").unwrap();
        let h = dlopen(path.as_ptr(), 1);
        if h.is_null() {
            eprintln!("grids: MultitouchSupport unavailable; trackpad throw off");
            return;
        }
        let sym = |n: &str| dlsym(h, CString::new(n).unwrap().as_ptr());
        let (list_f, reg_f, start_f) = (sym("MTDeviceCreateList"), sym("MTRegisterContactFrameCallback"), sym("MTDeviceStart"));
        if list_f.is_null() || reg_f.is_null() || start_f.is_null() {
            return;
        }
        let create_list: extern "C" fn() -> *const c_void = std::mem::transmute(list_f);
        let register: extern "C" fn(DeviceRef, ContactCb) = std::mem::transmute(reg_f);
        let start: extern "C" fn(DeviceRef, i32) = std::mem::transmute(start_f);
        let list = create_list();
        if list.is_null() {
            return;
        }
        let n = crate::ffi::CFArrayGetCount(list);
        for i in 0..n {
            let dev = crate::ffi::CFArrayGetValueAtIndex(list, i);
            register(dev, contact);
            start(dev, 0);
        }
    }
    *started = true;
}
