// Synthetic input for live tests against the self-test window: drive <cmd> ...
import AppKit

let args = CommandLine.arguments.dropFirst().map { $0 }
let src = CGEventSource(stateID: .hidSystemState)
func post(_ e: CGEvent?) { e?.post(tap: .cghidEventTap); usleep(12000) }
func mouse(_ t: CGEventType, _ x: Double, _ y: Double, flags: CGEventFlags = []) {
    let e = CGEvent(mouseEventSource: src, mouseType: t, mouseCursorPosition: CGPoint(x: x, y: y), mouseButton: .left)
    e?.flags = flags
    post(e)
}
func flags(_ f: CGEventFlags, key: CGKeyCode) {
    let e = CGEvent(keyboardEventSource: src, virtualKey: key, keyDown: true)
    e?.type = .flagsChanged
    e?.flags = f
    post(e)
}
func testWindow() -> CGRect? {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
    for w in list where (w[kCGWindowName as String] as? String) == "Grids Self-Test" {
        if let b = w[kCGWindowBounds as String] as? [String: Any] { return CGRect(dictionaryRepresentation: b as CFDictionary) }
    }
    return nil
}
func testWindowID() -> Int? {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
    return list.first { ($0[kCGWindowName as String] as? String) == "Grids Self-Test" }?[kCGWindowNumber as String] as? Int
}
func d(_ i: Int) -> Double { Double(args[i])! }

switch args.first ?? "" {
case "bounds":
    if let r = testWindow() { print(Int(r.minX), Int(r.minY), Int(r.width), Int(r.height)) } else { print("none") }
case "wid":
    print(testWindowID() ?? 0)
case "latency":
    // Press a shortcut and time until the window server shows the window moved.
    let code = CGKeyCode(args[1])!, f = CGEventFlags(rawValue: UInt64(args[2])!)
    let before = testWindow()
    let t0 = DispatchTime.now().uptimeNanoseconds
    let down = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: true); down?.flags = f; down?.post(tap: .cghidEventTap)
    let up = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: false); up?.flags = f; up?.post(tap: .cghidEventTap)
    var ms = -1.0
    while DispatchTime.now().uptimeNanoseconds - t0 < 2_000_000_000 {
        if let now = testWindow(), now != before { ms = Double(DispatchTime.now().uptimeNanoseconds - t0) / 1e6; break }
        usleep(500)
    }
    print(String(format: "%.1f", ms))
case "keyhold":
    let code = CGKeyCode(args[1])!, f = CGEventFlags(rawValue: UInt64(args[2])!), secs = d(3)
    let down = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: true); down?.flags = f; post(down)
    usleep(useconds_t(secs * 1_000_000))
    let up = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: false); up?.flags = f; post(up)
case "topat":
    // Window number of the topmost normal window under a point (0 if none).
    let pt = CGPoint(x: d(1), y: d(2))
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
    let hit = list.first { w in
        guard let layer = w[kCGWindowLayer as String] as? Int, layer < 20, let b = w[kCGWindowBounds as String] as? [String: Any],
              let r = CGRect(dictionaryRepresentation: b as CFDictionary) else { return false }
        return r.contains(pt)
    }
    print(hit?[kCGWindowNumber as String] as? Int ?? 0)
case "front":
    print(NSWorkspace.shared.frontmostApplication?.bundleIdentifier ?? "")
case "screen":
    let s = NSScreen.screens[0], v = s.visibleFrame, h = s.frame.height
    print(Int(v.minX), Int(h - v.maxY), Int(v.width), Int(v.height))
case "activate":
    NSRunningApplication.runningApplications(withBundleIdentifier: args[1]).first?.activate()
    usleep(300000)
case "key":
    let code = CGKeyCode(args[1])!, f = CGEventFlags(rawValue: UInt64(args[2])!)
    let down = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: true); down?.flags = f; post(down)
    let up = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: false); up?.flags = f; post(up)
case "drag":
    let (x1, y1, x2, y2) = (d(1), d(2), d(3), d(4))
    mouse(.mouseMoved, x1, y1)
    mouse(.leftMouseDown, x1, y1)
    for i in 1...30 { let t = Double(i) / 30; mouse(.leftMouseDragged, x1 + (x2 - x1) * t, y1 + (y2 - y1) * t); usleep(8000) }
    usleep(150000)
    mouse(.leftMouseUp, x2, y2)
case "throw":
    let (x, y, dx, dy) = (d(1), d(2), d(3), d(4))
    mouse(.mouseMoved, x, y)
    flags([.maskControl], key: 59)
    flags([.maskControl, .maskCommand], key: 55)
    for i in 1...20 { let t = Double(i) / 20; mouse(.mouseMoved, x + dx * t, y + dy * t, flags: [.maskControl, .maskCommand]); usleep(10000) }
    usleep(120000)
    flags([.maskControl], key: 55)
    flags([], key: 59)
case "quick":
    // Tap modifiers while sweeping the cursor: quick x y dx dy flags keycode
    let (x, y, dx, dy) = (d(1), d(2), d(3), d(4)); let f = CGEventFlags(rawValue: UInt64(args[5])!); let k = CGKeyCode(args[6])!
    mouse(.mouseMoved, x, y)
    flags(f, key: k)
    for i in 1...8 { let t = Double(i) / 8; mouse(.mouseMoved, x + dx * t, y + dy * t, flags: f); usleep(8000) }
    flags([], key: k)
case "hold":
    // Hold modifiers and move: hold x y dx dy flags keycode
    let (x, y, dx, dy) = (d(1), d(2), d(3), d(4)); let f = CGEventFlags(rawValue: UInt64(args[5])!); let k = CGKeyCode(args[6])!
    mouse(.mouseMoved, x, y)
    flags(f, key: k); usleep(60000)
    for i in 1...25 { let t = Double(i) / 25; mouse(.mouseMoved, x + dx * t, y + dy * t, flags: f); usleep(16000) }
    usleep(60000)
    flags([], key: k)
case "dclick":
    let (x, y) = (d(1), d(2))
    for n in 1...2 {
        for t in [CGEventType.leftMouseDown, .leftMouseUp] {
            let e = CGEvent(mouseEventSource: src, mouseType: t, mouseCursorPosition: CGPoint(x: x, y: y), mouseButton: .left)
            e?.setIntegerValueField(.mouseEventClickState, value: Int64(n)); post(e)
        }
    }
case "dragto":
    // Drag along a path of points: dragto x1 y1 x2 y2 [x3 y3 ...]
    let pts = stride(from: 1, to: args.count - 1, by: 2).map { CGPoint(x: d($0), y: d($0 + 1)) }
    mouse(.mouseMoved, pts[0].x, pts[0].y); mouse(.leftMouseDown, pts[0].x, pts[0].y)
    for (a, b) in zip(pts, pts.dropFirst()) {
        for i in 1...20 { let t = Double(i) / 20; mouse(.leftMouseDragged, a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t); usleep(8000) }
        usleep(120000)
    }
    mouse(.leftMouseUp, pts.last!.x, pts.last!.y)
case "move":
    mouse(.mouseMoved, d(1), d(2))
default:
    print("drive bounds|screen|activate <bundle>|key <code> <flags>|drag x1 y1 x2 y2|throw x y dx dy|move x y")
}
