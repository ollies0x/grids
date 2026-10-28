//! Launch at login through SMAppService (no helper app, no LaunchAgent plist).

use objc2_service_management::{SMAppService, SMAppServiceStatus};

pub fn apply(on: bool) {
    let svc = unsafe { SMAppService::mainAppService() };
    let enabled = unsafe { svc.status() } == SMAppServiceStatus::Enabled;
    let res = match (on, enabled) {
        (true, false) => unsafe { svc.registerAndReturnError() },
        (false, true) => unsafe { svc.unregisterAndReturnError() },
        _ => Ok(()),
    };
    if let Err(e) = res {
        eprintln!("grids: launch at login: {}", e.localizedDescription());
    }
}
