//! The Settings window is a separate SwiftUI app bundled inside Grids.app, so its memory is
//! only used while it is open. It edits the same config file and asks the daemon to reload.

use objc2_app_kit::{NSWorkspace, NSWorkspaceOpenConfiguration};
use objc2_foundation::{NSArray, NSBundle, NSString, NSURL};
use std::path::PathBuf;

fn helper_path() -> Option<PathBuf> {
    let bundle = NSBundle::mainBundle().bundlePath().to_string();
    let inside = PathBuf::from(&bundle).join("Contents/Helpers/Grids Settings.app");
    if inside.exists() {
        return Some(inside);
    }
    // Running unbundled from the build tree.
    let exe = std::env::current_exe().ok()?;
    let dev = exe.parent()?.join("../../build/Grids.app/Contents/Helpers/Grids Settings.app");
    dev.exists().then_some(dev)
}

pub fn open(pane: Option<&str>) {
    // An already-open Settings window ignores launch arguments, so also say it directly.
    if let Some(p) = pane {
        use crate::ffi::*;
        let (n, obj) = (cfstr("app.grids.settings.pane"), cfstr(p));
        unsafe { CFNotificationCenterPostNotification(CFNotificationCenterGetDistributedCenter(), n.ptr(), obj.ptr(), std::ptr::null(), 1) };
    }
    let Some(path) = helper_path() else {
        eprintln!("grids: Settings app not found; edit {}", crate::config::active_path().display());
        return;
    };
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    let cfg = NSWorkspaceOpenConfiguration::configuration();
    cfg.setActivates(true);
    let mut args = vec![NSString::from_str("--daemon"), NSString::from_str(&std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default())];
    if let Some(p) = pane {
        args.push(NSString::from_str("--pane"));
        args.push(NSString::from_str(p));
    }
    cfg.setArguments(&NSArray::from_retained_slice(&args));
    NSWorkspace::sharedWorkspace().openApplicationAtURL_configuration_completionHandler(&url, &cfg, None);
}
