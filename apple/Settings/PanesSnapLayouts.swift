import AppKit
import SwiftUI

struct SnapPane: View {
    @EnvironmentObject var store: Store
    @State private var portrait = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            PaneHeader(title: "Snap Areas", subtitle: "Drag a window to an edge or corner, onto a snap target, or onto the snap panel.")
            Glass {
                ToggleRow(title: "Snap windows by dragging", on: store.bool("snap.enabled"))
                ToggleRow(title: "Restore window size when unsnapped", on: store.bool("snap.restoreOnUnsnap"))
                ToggleRow(title: "Animate footprint", on: store.bool("snap.animate"))
                ToggleRow(title: "Haptic feedback", detail: "A tick on the trackpad when an area is picked.", on: store.bool("snap.haptic"))
                Row(title: "Only snap while holding", detail: "Leave empty to always snap.") { ModifierPicker(flags: store.int("snap.modifiers")) }
            }
            Glass {
                HStack {
                    SectionTitle(text: "Edges & corners")
                    Spacer()
                    Picker("", selection: $portrait) {
                        Text("Landscape").tag(false)
                        Text("Portrait").tag(true)
                    }.pickerStyle(.segmented).labelsHidden().frame(width: 200)
                }
                EdgeMap(kp: portrait ? "snap.portraitEdges" : "snap.edges")
            }
            Glass {
                SectionTitle(text: "Snap targets")
                Text("Custom Size & Position entries and layout entries marked “Use as a snap target” appear as drop zones while you drag.")
                    .font(.system(size: 12)).foregroundStyle(Palette.dim)
                Row(title: "Show targets") {
                    Picker("", selection: store.string("snap.targetsMode")) {
                        Text("Always").tag("always")
                        Text("While pressing").tag("whilePressing")
                        Text("While not pressing").tag("whileNotPressing")
                    }.labelsHidden().frame(width: 170)
                }
                if (store.value("snap.targetsMode") as? String ?? "always") != "always" {
                    Row(title: "Modifier keys") { ModifierPicker(flags: store.int("snap.targetsModifiers")) }
                }
            }
            Glass {
                HStack {
                    SectionTitle(text: "Snap panel")
                    Spacer()
                    Toggle("", isOn: store.bool("snap.panelEnabled")).toggleStyle(.switch).labelsHidden().controlSize(.small)
                }
                Row(title: "Position") {
                    Picker("", selection: store.string("snap.panelPosition")) {
                        ForEach([("above", "Above cursor"), ("below", "Below cursor"), ("left", "Left of cursor"), ("right", "Right of cursor"), ("top", "Top edge"), ("bottom", "Bottom edge"), ("topLeft", "Top left"), ("topRight", "Top right"), ("bottomLeft", "Bottom left"), ("bottomRight", "Bottom right")], id: \.0) { v, l in
                            Text(l).tag(v)
                        }
                    }.labelsHidden().frame(width: 170)
                }
                let items = store.stringList("snap.panelItems")
                ForEach(Array(items.enumerated()), id: \.offset) { i, _ in
                    HStack {
                        CommandPicker(cmd: Binding(get: { store.stringList("snap.panelItems")[safe: i] ?? "" }, set: { v in
                            var l = store.stringList("snap.panelItems"); l[i] = v; store.set("snap.panelItems", l)
                        }), allowNone: false)
                        Button { var l = store.stringList("snap.panelItems"); l.remove(at: i); store.set("snap.panelItems", l) } label: { Image(systemName: "minus.circle") }.buttonStyle(.plain)
                    }
                }
                Button { store.set("snap.panelItems", items + ["center-half"]) } label: { Label("Add tile", systemImage: "plus") }.buttonStyle(GlassButtonStyle())
            }
        }
    }
}

extension Array {
    subscript(safe i: Int) -> Element? { indices.contains(i) ? self[i] : nil }
}

/// A screen diagram with a picker on each edge and corner.
struct EdgeMap: View {
    @EnvironmentObject var store: Store
    let kp: String
    var body: some View {
        let edgeModes: [(String, String)] = [
            ("compound:thirds", "Thirds, drag to centre for two thirds"),
            ("compound:halves", "Left or right half"),
            ("compound:sixths", "Sixths from corners; thirds"),
            ("compound:fourths", "Fourths columns"),
        ]
        let sideModes: [(String, String)] = [("compound:half-corners", "Half, top/bottom half near corners")]
        VStack(spacing: 10) {
            HStack {
                pick("topLeft", [])
                Spacer()
                pick("top", edgeModes)
                Spacer()
                pick("topRight", [])
            }
            HStack {
                pick("left", sideModes)
                Spacer()
                RoundedRectangle(cornerRadius: 10).strokeBorder(Palette.line, lineWidth: 1.5)
                    .background(RoundedRectangle(cornerRadius: 10).fill(Color.white.opacity(0.03)))
                    .frame(width: 180, height: 110)
                    .overlay(Image(systemName: "macwindow").font(.system(size: 30)).foregroundStyle(Palette.dim))
                Spacer()
                pick("right", sideModes)
            }
            HStack {
                pick("bottomLeft", [])
                Spacer()
                pick("bottom", edgeModes)
                Spacer()
                pick("bottomRight", [])
            }
        }
    }

    private func pick(_ key: String, _ extra: [(String, String)]) -> some View {
        CommandPicker(cmd: store.string("\(kp).\(key)"), extra: extra, width: 175)
    }
}

struct CustomPane: View {
    @EnvironmentObject var store: Store
    @State private var open: Int? = nil

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(alignment: .top) {
                PaneHeader(title: "Custom Size & Position", subtitle: "Your own sizes and positions, with optional extra steps on repeat. Use them as shortcuts, throw sectors, snap areas, targets or menu items.")
                Spacer()
                Menu {
                    Button("Blank") { add(nil) }
                    Button("From the front window") { add(frontWindowPlacement()) }
                } label: { Label("Add", systemImage: "plus") }.menuStyle(.borderlessButton).frame(width: 70)
            }
            ForEach(Array(store.list("custom").enumerated()), id: \.offset) { i, _ in
                let kp = "custom.\(i)"
                Glass {
                    HStack(spacing: 12) {
                        ActionIcon(unit: store.unit(of: "custom:" + (store.value(kp + ".name") as? String ?? "")), size: 20)
                        TextField("Name", text: store.nameBinding(kp + ".name", prefix: "custom:")).textFieldStyle(.plain).font(.system(size: 14, weight: .semibold))
                        Spacer()
                        ShortcutRecorder(cmd: "custom:" + (store.value(kp + ".name") as? String ?? ""))
                        Button { open = open == i ? nil : i } label: { Image(systemName: open == i ? "chevron.up" : "ellipsis") }.buttonStyle(GlassButtonStyle())
                    }
                    if open == i {
                        ToggleRow(title: "Use as a snap target", detail: "Needs width and height, X and Y for a custom origin, and a fixed display.", on: store.bool(kp + ".snapTarget"))
                        ForEach(Array(store.list(kp + ".steps").enumerated()), id: \.offset) { j, _ in
                            VStack(alignment: .leading, spacing: 6) {
                                HStack {
                                    SectionTitle(text: j == 0 ? "First run" : "Repeat \(j)")
                                    Spacer()
                                    if j > 0 {
                                        Button { var s = store.list(kp + ".steps"); s.remove(at: j); store.set(kp + ".steps", s) } label: { Image(systemName: "minus.circle") }.buttonStyle(.plain)
                                    }
                                }
                                StepEditor(kp: "\(kp).steps.\(j)")
                            }
                            .padding(10).background(RoundedRectangle(cornerRadius: 12).fill(Color.white.opacity(0.04)))
                        }
                        HStack {
                            Button { store.set(kp + ".steps", store.list(kp + ".steps") + [blankStep(nil)]) } label: { Label("Add repetition", systemImage: "plus") }.buttonStyle(GlassButtonStyle())
                            Spacer()
                            Button(role: .destructive) { var l = store.list("custom"); l.remove(at: i); store.set("custom", l); open = nil } label: { Label("Delete", systemImage: "trash") }.buttonStyle(GlassButtonStyle())
                        }
                    }
                }
            }
        }
    }

    private func blankStep(_ place: [String: Any]?) -> [String: Any] {
        ["command": NSNull(), "place": place ?? ["position": "center", "w": 0.6, "h": 0.8] as [String: Any], "display": ""]
    }

    private func add(_ place: [String: Any]?) {
        var n = store.list("custom").count + 1
        while store.customNames.contains("Custom \(n)") { n += 1 }
        let entry: [String: Any] = ["name": "Custom \(n)", "steps": [blankStep(place)], "snapTarget": false]
        store.set("custom", store.list("custom") + [entry])
        open = store.list("custom").count - 1
    }
}

/// The front window's frame (not this one) as a custom placement.
func frontWindowPlacement() -> [String: Any]? {
    guard let info = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else { return nil }
    let own = ProcessInfo.processInfo.processIdentifier
    for w in info {
        guard (w[kCGWindowLayer as String] as? Int) == 0, (w[kCGWindowOwnerPID as String] as? Int32) != own,
              let b = w[kCGWindowBounds as String] as? [String: Any], let r = CGRect(dictionaryRepresentation: b as CFDictionary) else { continue }
        guard let screen = NSScreen.screens.first(where: { s in
            let f = s.frame
            let top = NSScreen.screens[0].frame.height - f.maxY
            return CGRect(x: f.minX, y: top, width: f.width, height: f.height).contains(CGPoint(x: r.midX, y: r.midY))
        }) else { continue }
        let v = screen.visibleFrame
        let vt = NSScreen.screens[0].frame.height - v.maxY
        return ["position": "custom", "x": (r.minX - v.minX) / v.width, "y": (r.minY - vt) / v.height, "w": r.width / v.width, "h": r.height / v.height]
    }
    return nil
}

struct LayoutsPane: View {
    @EnvironmentObject var store: Store
    @Binding var focus: String?
    @State private var open: Int? = nil

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(alignment: .top) {
                PaneHeader(title: "Layouts", subtitle: "Arrange every app in your workspace at once, from a shortcut, the menu, a throw sector, or when something changes.")
                Spacer()
                Button { add() } label: { Label("Layout", systemImage: "plus") }.buttonStyle(GlassButtonStyle(prominent: true))
            }
            Glass {
                Row(title: "Save current layout", detail: "Also in the menu bar menu. Captures every window on the display in front.") { ShortcutRecorder(cmd: "save-layout") }
                ToggleRow(title: "Save as presets where possible instead of absolute dimensions", on: store.bool("layoutSaveAsPresets"))
            }
            ForEach(Array(store.list("layouts").enumerated()), id: \.offset) { i, _ in
                layoutCard(i)
            }
        }
        .onAppear {
            if let f = focus, let i = store.layoutNames.firstIndex(of: f) { open = i; focus = nil }
        }
    }

    private func add() {
        var n = store.list("layouts").count + 1
        while store.layoutNames.contains("Layout \(n)") { n += 1 }
        let l: [String: Any] = ["name": "Layout \(n)", "entries": [], "others": "none", "onWake": false, "onWindowOpens": false, "frontmostOnly": false, "allWindows": false, "bringToFront": true, "launchApps": false]
        store.set("layouts", store.list("layouts") + [l])
        open = store.list("layouts").count - 1
    }

    @ViewBuilder private func layoutCard(_ i: Int) -> some View {
        let kp = "layouts.\(i)"
        let name = store.value(kp + ".name") as? String ?? ""
        Glass {
            HStack(spacing: 12) {
                Image(systemName: "square.grid.3x1.below.line.grid.1x2").foregroundStyle(Palette.accent)
                TextField("Name", text: store.nameBinding(kp + ".name", prefix: "layout:")).textFieldStyle(.plain).font(.system(size: 14, weight: .semibold))
                Text("\(store.list(kp + ".entries").count) windows").font(.system(size: 11)).foregroundStyle(Palette.dim)
                Spacer()
                ShortcutRecorder(cmd: "layout:" + name)
                Button { open = open == i ? nil : i } label: { Image(systemName: open == i ? "chevron.up" : "ellipsis") }.buttonStyle(GlassButtonStyle())
            }
            if open == i {
                SectionTitle(text: "Apply when")
                HStack(spacing: 16) {
                    trigger("Display connects", kp + ".onDisplayConnected")
                    trigger("Display disconnects", kp + ".onDisplayDisconnected")
                }
                HStack(spacing: 16) {
                    Toggle("Waking from sleep", isOn: store.bool(kp + ".onWake"))
                    Toggle("A window opens", isOn: store.bool(kp + ".onWindowOpens"))
                }
                SectionTitle(text: "Apply to")
                HStack(spacing: 16) {
                    Toggle("Frontmost app only", isOn: store.bool(kp + ".frontmostOnly"))
                    Toggle("All windows, beyond first match", isOn: store.bool(kp + ".allWindows"))
                }
                SectionTitle(text: "Behaviour")
                HStack(spacing: 16) {
                    Toggle("Bring windows to front", isOn: store.bool(kp + ".bringToFront"))
                    Toggle("Launch closed or minimised apps", isOn: store.bool(kp + ".launchApps"))
                }
                Row(title: "Apps not in this layout") {
                    Picker("", selection: store.string(kp + ".others")) {
                        Text("Leave alone").tag("none")
                        Text("Hide").tag("hide")
                        Text("Minimise").tag("minimize")
                        Text("Quit").tag("quit")
                    }.labelsHidden().frame(width: 140)
                }
                SectionTitle(text: "Windows")
                ForEach(Array(store.list(kp + ".entries").enumerated()), id: \.offset) { j, _ in
                    entryEditor("\(kp).entries.\(j)", remove: {
                        var l = store.list(kp + ".entries"); l.remove(at: j); store.set(kp + ".entries", l)
                    })
                }
                HStack {
                    Button {
                        let e: [String: Any] = ["app": "*", "appName": "Any app", "steps": [["command": "left-half", "display": ""]], "snapTarget": false, "titleMode": "any", "title": ""]
                        store.set(kp + ".entries", store.list(kp + ".entries") + [e])
                    } label: { Label("Add window", systemImage: "plus") }.buttonStyle(GlassButtonStyle())
                    Button { open_url("grids://execute-layout?name=" + (name.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? "")) } label: { Label("Apply now", systemImage: "play.fill") }.buttonStyle(GlassButtonStyle())
                    Spacer()
                    Button(role: .destructive) { var l = store.list("layouts"); l.remove(at: i); store.set("layouts", l); open = nil } label: { Label("Delete", systemImage: "trash") }.buttonStyle(GlassButtonStyle())
                }
            }
        }
    }

    private func trigger(_ title: String, _ kp: String) -> some View {
        HStack(spacing: 6) {
            Toggle(title, isOn: Binding(get: { store.value(kp) is String }, set: { store.set(kp, $0 ? "*" : nil) }))
            if store.value(kp) is String {
                Picker("", selection: store.string(kp)) {
                    Text("Any display").tag("*")
                    ForEach(NSScreen.screens, id: \.localizedName) { s in Text(s.localizedName).tag(s.localizedName) }
                    if let v = store.value(kp) as? String, v != "*", !NSScreen.screens.contains(where: { $0.localizedName == v }) { Text(v).tag(v) }
                }.labelsHidden().frame(width: 150)
            }
        }
    }

    private func entryEditor(_ kp: String, remove: @escaping () -> Void) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                AppPicker(bundle: store.string(kp + ".app"), name: store.string(kp + ".appName"), allowAny: true)
                Picker("", selection: store.string(kp + ".titleMode")) {
                    Text("Any title").tag("any")
                    Text("Title loosely matches").tag("loose")
                    Text("Title contains").tag("contains")
                    Text("Title matches regex").tag("regex")
                }.labelsHidden().frame(width: 170)
                if (store.value(kp + ".titleMode") as? String ?? "any") != "any" {
                    TextField("Title", text: store.string(kp + ".title")).textFieldStyle(.roundedBorder).frame(width: 160)
                }
                Spacer()
                Toggle("Snap target", isOn: store.bool(kp + ".snapTarget")).font(.system(size: 11.5))
                Button(action: remove) { Image(systemName: "minus.circle") }.buttonStyle(.plain)
            }
            ForEach(Array(store.list(kp + ".steps").enumerated()), id: \.offset) { j, _ in
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        SectionTitle(text: j == 0 ? "Placement" : "On repeat \(j)")
                        Spacer()
                        if j > 0 {
                            Button { var s = store.list(kp + ".steps"); s.remove(at: j); store.set(kp + ".steps", s) } label: { Image(systemName: "minus.circle") }.buttonStyle(.plain)
                        }
                    }
                    StepEditor(kp: "\(kp).steps.\(j)")
                }
            }
            Button { store.set(kp + ".steps", store.list(kp + ".steps") + [["command": "right-half", "display": ""]]) } label: { Label("Add repetition", systemImage: "plus") }
                .buttonStyle(.plain).font(.system(size: 11.5)).foregroundStyle(Palette.dim)
        }
        .padding(10).background(RoundedRectangle(cornerRadius: 12).fill(Color.white.opacity(0.04)))
    }
}

func open_url(_ s: String) {
    if let u = URL(string: s) {
        let cfg = NSWorkspace.OpenConfiguration()
        cfg.activates = false
        NSWorkspace.shared.open(u, configuration: cfg)
    }
}
