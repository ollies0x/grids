// A plain window for Grids's self-test to move around, so tests never touch real windows.
import AppKit

let app = NSApplication.shared
app.setActivationPolicy(.regular)
let w = NSWindow(contentRect: NSRect(x: 200, y: 200, width: 640, height: 420), styleMask: [.titled, .resizable, .closable, .miniaturizable], backing: .buffered, defer: false)
w.title = "Grids Self-Test"
w.minSize = NSSize(width: 80, height: 60)
w.isReleasedWhenClosed = false
// Float above normal windows so cursor-driven tests hit it without needing focus.
w.level = .floating
w.orderFrontRegardless()
// Launched with `open -n`, it may activate itself; tests rely on it being the front window.
if CommandLine.arguments.contains("--activate") || ProcessInfo.processInfo.environment["GRIDS_TEST_ACTIVATE"] != nil {
    w.makeKeyAndOrderFront(nil)
}
DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
    if !CommandLine.arguments.contains("--background") {
        app.activate()
        w.makeKeyAndOrderFront(nil)
    }
}
app.run()
