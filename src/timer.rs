//! Main run loop timers that call Rust closures.

use crate::ffi::*;
use std::ffi::c_void;

type Callback = Box<dyn FnMut()>;

extern "C" fn fire(_t: *const c_void, info: *mut c_void) {
    let cb = unsafe { &mut *(info as *mut Callback) };
    cb();
}

extern "C" fn release(info: *const c_void) {
    drop(unsafe { Box::from_raw(info as *mut Callback) });
}

/// A scheduled timer. Dropping the handle leaves it scheduled; call `cancel` to stop it.
pub struct Timer(Cf);

impl Timer {
    pub fn cancel(&self) {
        unsafe { CFRunLoopTimerInvalidate(self.0.ptr()) };
    }
}

fn schedule(delay: f64, interval: f64, cb: Callback) -> Timer {
    let info = Box::into_raw(Box::new(cb)) as *mut c_void;
    let mut ctx = CFRunLoopTimerContext {
        version: 0,
        info,
        retain: std::ptr::null(),
        release: release as *const c_void,
        copy_description: std::ptr::null(),
    };
    unsafe {
        let t = CFRunLoopTimerCreate(kCFAllocatorDefault, now() + delay, interval, 0, 0, fire, &mut ctx);
        CFRunLoopAddTimer(CFRunLoopGetMain(), t, kCFRunLoopCommonModes);
        Timer(Cf(t))
    }
}

pub fn after(delay: f64, f: impl FnOnce() + 'static) -> Timer {
    let mut f = Some(f);
    schedule(delay, 0.0, Box::new(move || {
        if let Some(f) = f.take() {
            f()
        }
    }))
}

pub fn every(interval: f64, f: impl FnMut() + 'static) -> Timer {
    schedule(interval, interval, Box::new(f))
}
