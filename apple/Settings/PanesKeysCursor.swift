import AppKit
import Combine
import SwiftUI

struct WelcomePane: View {
    @EnvironmentObject var store: Store
    @State private var trusted = false
    let timer = Timer.publish(every: 1, on: .main, in: .common).autoconnect()

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            PaneHeader(title: "Welcome to Grids", subtitle: "Every Rectangle action and the whole Rectangle Pro set, in a lean native app.")
            Glass {
                HStack(spacing: 14) {
                    Image(systemName: trusted ? "checkmark.seal.fill" : "hand.raised.fill")
                        .font(.system(size: 26)).foregroundStyle(trusted ? Palette.mint : Palette.ember)
                    VStack(alignment: .leading, spacing: 3) {
                        Text(trusted ? "Accessibility is on" : "Grids needs Accessibility access").font(.system(size: 15, weight: .semibold))
                        Text(trusted ? "Grids can move windows. Nothing else needs granting." : "It is how any window manager moves other apps' windows. Turn on Grids in System Settings › Privacy & Security › Accessibility.")
                            .font(.system(size: 12)).foregroundStyle(Palette.dim)
                    }
                    Spacer()
                    if !trusted {
                        Button("Open System Settings") {
                            NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")!)
                        }.buttonStyle(GlassButtonStyle(prominent: true))
                    }
                }
            }
            Glass {
                SectionTitle(text: "Ways to move a window")
                tip("keyboard", "Shortcuts", "⌃⌥ with arrows, U I J K, D F G, E R T and ↩ out of the box. Repeat a shortcut to cycle sizes or displays.")
                tip("hand.point.up.left", "Window Throw", "Hold ⌃⌘ over any window and flick the cursor toward where it should go. Go further for the Long Throw.")
                tip("rectangle.dashed", "Snap", "Drag a window to a screen edge or corner, onto a snap target, or onto the snap panel.")
                tip("sidebar.squares.left", "Stash & Pin", "Tuck windows into the screen edge, or pin one app in a column beside everything else.")
                tip("square.grid.3x1.below.line.grid.1x2", "Layouts", "Save a whole workspace and bring it back with one shortcut, or when a display connects.")
            }
            Glass {
                HStack {
                    VStack(alignment: .leading, spacing: 3) {
                        Text("Already use Rectangle?").font(.system(size: 13, weight: .semibold))
                        Text("Your Rectangle shortcuts, gaps and repeat behaviour were imported on first launch. Quit Rectangle so the two don't fight over the same keys.")
                            .font(.system(size: 12)).foregroundStyle(Palette.dim)
                    }
                    Spacer()
                    if NSRunningApplication.runningApplications(withBundleIdentifier: "com.knollsoft.Rectangle").first != nil {
                        Button("Quit Rectangle") {
                            NSRunningApplication.runningApplications(withBundleIdentifier: "com.knollsoft.Rectangle").forEach { $0.terminate() }
                        }.buttonStyle(GlassButtonStyle())
                    }
                }
            }
        }
        .onAppear { check() }
        .onReceive(timer) { _ in check() }
    }

    private func check() {
        // The daemon reports its own trust; fall back to asking the system about it.
        trusted = FileManager.default.fileExists(atPath: NSHomeDirectory() + "/Library/Application Support/Grids/.trusted")
    }

    private func tip(_ symbol: String, _ title: String, _ text: String) -> some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: symbol).frame(width: 22).foregroundStyle(Palette.accent)
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.system(size: 13, weight: .semibold))
                Text(text).font(.system(size: 12)).foregroundStyle(Palette.dim)
            }
        }
    }
}

struct ShortcutsPane: View {
    @EnvironmentObject var store: Store
    @State private var query = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            PaneHeader(title: "Keyboard Shortcuts", subtitle: "Every size and position. Repeat a shortcut to cycle; hold Nudge and Size shortcuts to keep going.")
            HStack {
                Image(systemName: "magnifyingglass").foregroundStyle(Palette.dim)
                TextField("Search actions", text: $query).textFieldStyle(.plain)
            }
            .padding(8).background(RoundedRectangle(cornerRadius: 10).fill(Color.white.opacity(0.07)))
            Glass {
                Row(title: "Win arrow keys", detail: "These modifiers plus the arrow keys run a Windows-style state machine: → then ↑ goes right half, then top right.") {
                    ModifierPicker(flags: store.int("winArrowModifiers"), allowFn: false)
                }
            }
            ForEach(store.catalog?.categories ?? [], id: \.self) { cat in
                let acts = (store.catalog?.actions ?? []).filter { $0.category == cat && (query.isEmpty || $0.name.localizedCaseInsensitiveContains(query)) }
                if !acts.isEmpty {
                    Glass {
                        SectionTitle(text: cat)
                        ForEach(acts) { a in
                            HStack(spacing: 12) {
                                ActionIcon(unit: a.unit, size: 16)
                                Text(a.name).font(.system(size: 13))
                                if a.holdable { Text("hold").font(.system(size: 10)).foregroundStyle(Palette.dim) }
                                Spacer()
                                ShortcutRecorder(cmd: a.id)
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Eight pickers around a dial, clockwise from up.
struct SectorDial: View {
    @EnvironmentObject var store: Store
    let kp: String
    let label: String
    var body: some View {
        let dirs = ["↑", "↗", "→", "↘", "↓", "↙", "←", "↖"]
        ZStack {
            Circle().strokeBorder(Palette.line, lineWidth: 1).frame(width: 150, height: 150)
            Circle().fill(Palette.accent.opacity(0.25)).frame(width: 18, height: 18)
            Text(label).font(.system(size: 10, weight: .semibold)).foregroundStyle(Palette.dim).offset(y: 22)
            ForEach(0..<8, id: \.self) { i in
                let ang = Double(i) * .pi / 4 - .pi / 2
                VStack(spacing: 2) {
                    Text(dirs[i]).font(.system(size: 11)).foregroundStyle(Palette.dim)
                    CommandPicker(cmd: Binding(
                        get: { (store.list(kp).compactMap { $0 as? String }.dropFirst(i).first) ?? "" },
                        set: { v in
                            var arr = store.list(kp).compactMap { $0 as? String }
                            while arr.count < 8 { arr.append("") }
                            arr[i] = v
                            store.set(kp, arr)
                        }), width: 150)
                }
                .offset(x: cos(ang) * 205, y: sin(ang) * 118)
            }
        }
        .frame(height: 330).frame(maxWidth: .infinity)
    }
}

struct CursorPane: View {
    @EnvironmentObject var store: Store
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            PaneHeader(title: "Cursor Movement", subtitle: "Window Throw, Long Throw and Move & Resize. The window under the cursor doesn't need to be in front.")
            Glass {
                SectionTitle(text: "Window Throw triggers")
                Row(title: "Modifier keys", detail: "Hold, move the cursor toward a sector, release. Clear them to turn this trigger off.") {
                    ModifierPicker(flags: store.int("throw.modifiers"))
                }
                Row(title: "Mouse button", detail: "Hold a mouse button instead (3 is the middle button). A click without a throw still reaches the app.") {
                    Picker("", selection: Binding(
                        get: { (store.value("throw.mouseButton") as? NSNumber)?.intValue ?? 0 },
                        set: { store.set("throw.mouseButton", $0 == 0 ? nil : $0) })) {
                        Text("Off").tag(0)
                        ForEach(2..<8, id: \.self) { b in Text("Button \(b + 1)").tag(b) }
                    }.labelsHidden().frame(width: 120)
                }
                Row(title: "Trackpad", detail: "Rest this many fingers, lift all but one, then move it.") {
                    Picker("", selection: Binding(
                        get: { (store.value("throw.trackpadFingers") as? NSNumber)?.intValue ?? 0 },
                        set: { store.set("throw.trackpadFingers", $0 == 0 ? nil : $0) })) {
                        Text("Off").tag(0)
                        Text("3 fingers").tag(3)
                        Text("4 fingers").tag(4)
                        Text("5 fingers").tag(5)
                    }.labelsHidden().frame(width: 120)
                }
            }
            Glass {
                SectionTitle(text: "Throw sectors")
                SectorDial(kp: "throw.sectors", label: "THROW")
            }
            Glass {
                HStack {
                    SectionTitle(text: "Long Throw sectors")
                    Spacer()
                    Toggle("", isOn: store.bool("throw.longThrow")).toggleStyle(.switch).labelsHidden().controlSize(.small)
                }
                SectorDial(kp: "throw.longSectors", label: "LONG")
                Row(title: "Distance to trigger") { Slider2(value: store.double("throw.longThrowDistance"), range: 80...600) }
            }
            Glass {
                SectionTitle(text: "Feel")
                ToggleRow(title: "Scrolling resizes the footprint", on: store.bool("throw.scrollResizes"))
                ToggleRow(title: "Moving to another display centres the window there", on: store.bool("throw.otherDisplayCenter"))
                ToggleRow(title: "Ignore windows without toolbars", on: store.bool("throw.ignoreWithoutToolbar"))
                Row(title: "Safe area", detail: "Distance from the start point before a sector is chosen.") { Slider2(value: store.double("throw.safeArea"), range: 0...120) }
                Row(title: "Distance before the reticle appears", detail: "Raise this when the throw is on the middle button, so plain clicks still work.") {
                    Slider2(value: store.double("throw.minDistToAppear"), range: 0...80)
                }
                Row(title: "Reticle size") { Slider2(value: store.double("throw.reticleSize"), range: 12...64) }
                ToggleRow(title: "Ring-style reticle", on: store.bool("throw.reticleRing"))
            }
            Glass {
                SectionTitle(text: "Move & Resize")
                Row(title: "Move", detail: "Hold these and move the cursor to drag the window under it.") { ModifierPicker(flags: store.int("moveResize.moveModifiers")) }
                Row(title: "Resize", detail: "Hold these and move to resize from the nearest corner.") { ModifierPicker(flags: store.int("moveResize.resizeModifiers")) }
            }
        }
    }
}

struct QuickThrowPane: View {
    @EnvironmentObject var store: Store
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            PaneHeader(title: "Quick Throw", subtitle: "Sweep the cursor across a window and tap a modifier. The sweep's direction picks the action. No footprint, just done.")
            ForEach(Array(store.list("quickThrow").enumerated()), id: \.offset) { i, _ in
                Glass {
                    HStack {
                        SectionTitle(text: "Modifier")
                        ModifierPicker(flags: store.int("quickThrow.\(i).modifiers"))
                        Spacer()
                        Button(role: .destructive) {
                            var l = store.list("quickThrow"); l.remove(at: i); store.set("quickThrow", l)
                        } label: { Image(systemName: "trash") }.buttonStyle(GlassButtonStyle())
                    }
                    Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 8) {
                        ForEach([("up", "Sweep up"), ("down", "Sweep down"), ("left", "Sweep left"), ("right", "Sweep right")], id: \.0) { k, label in
                            GridRow {
                                Text(label).foregroundStyle(Palette.dim)
                                CommandPicker(cmd: store.optString("quickThrow.\(i).\(k)"))
                            }
                        }
                    }
                }
            }
            Button {
                var l = store.list("quickThrow")
                l.append(["modifiers": CTRL, "left": "fill-left", "right": "fill-right", "up": "maximize", "down": "minimize"] as [String: Any])
                store.set("quickThrow", l)
            } label: { Label("Add a modifier", systemImage: "plus") }.buttonStyle(GlassButtonStyle(prominent: true))
            Text("Tip from Rectangle Pro's author: Fill Left, Fill Right, Maximize and Minimize on ⌃, and Hide App on ⌃⌥⌘.")
                .font(.system(size: 11.5)).foregroundStyle(Palette.dim)
        }
    }
}
