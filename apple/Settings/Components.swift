import AppKit
import SwiftUI

/// Click, then press a key combination. Esc cancels, Delete clears.
struct ShortcutRecorder: View {
    @EnvironmentObject var store: Store
    let cmd: String
    @State private var recording = false
    @State private var monitor: Any?
    @State private var conflict: String?

    var body: some View {
        let sc = store.shortcut(cmd)
        HStack(spacing: 6) {
            if let c = conflict {
                Text("used by \(store.name(of: c))").font(.system(size: 10.5)).foregroundStyle(Palette.ember)
            }
            Button {
                recording ? stop() : start()
            } label: {
                Text(recording ? "Type shortcut…" : sc.map { modsString($0.1) + keyName($0.0) } ?? "Record")
                    .font(.system(size: 12, weight: sc == nil ? .regular : .semibold, design: .rounded))
                    .foregroundStyle(sc == nil && !recording ? Palette.dim : .white)
                    .frame(minWidth: 92)
            }
            .buttonStyle(GlassButtonStyle(prominent: recording))
            if sc != nil && !recording {
                Button { store.setShortcut(cmd, nil); conflict = nil } label: { Image(systemName: "xmark.circle.fill").foregroundStyle(Palette.dim) }
                    .buttonStyle(.plain)
            }
        }
        .onDisappear { stop() }
    }

    private func start() {
        recording = true
        post("app.grids.suspend")
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { e in
            let mods = Int(e.modifierFlags.intersection(.deviceIndependentFlagsMask).rawValue) & (CMD | OPT | CTRL | SHIFT | FN)
            if e.keyCode == 53 && mods == 0 { stop(); return nil }
            if (e.keyCode == 51 || e.keyCode == 117) && mods == 0 { store.setShortcut(cmd, nil); stop(); return nil }
            let sc = (Int(e.keyCode), mods & ~FN)
            conflict = store.owner(of: sc, except: cmd)
            if let c = conflict { store.setShortcut(c, nil) }
            store.setShortcut(cmd, sc)
            stop()
            return nil
        }
    }

    private func stop() {
        if let m = monitor { NSEvent.removeMonitor(m) }
        monitor = nil
        if recording { post("app.grids.resume") }
        recording = false
    }
}

func post(_ name: String) {
    DistributedNotificationCenter.default().postNotificationName(Notification.Name(name), object: nil, userInfo: nil, deliverImmediately: true)
}

/// Modifier-only setting (Throw, Quick Throw, Move & Resize, snap requirements).
struct ModifierPicker: View {
    @Binding var flags: Int
    var allowFn = true
    var body: some View {
        HStack(spacing: 4) {
            ForEach(Array(zip(["⌃", "⌥", "⇧", "⌘", "fn"], [CTRL, OPT, SHIFT, CMD, FN])), id: \.1) { label, bit in
                if bit != FN || allowFn {
                    Button { flags ^= bit } label: { Chip(text: label, on: flags & bit != 0) }.buttonStyle(.plain)
                }
            }
        }
    }
}

/// Every command, grouped by category, with icons; plus custom entries and layouts.
struct CommandPicker: View {
    @EnvironmentObject var store: Store
    @Binding var cmd: String
    var allowNone = true
    var extra: [(String, String)] = []
    var width: CGFloat = 210

    var body: some View {
        Menu {
            if allowNone { Button("None") { cmd = "" } }
            if !extra.isEmpty {
                Section { ForEach(extra, id: \.0) { e in Button(e.1) { cmd = e.0 } } }
            }
            ForEach(store.catalog?.categories ?? [], id: \.self) { cat in
                let acts = store.catalog?.actions.filter { $0.category == cat } ?? []
                if !acts.isEmpty && cat != "App" {
                    Menu(cat) { ForEach(acts) { a in Button(a.name) { cmd = a.id } } }
                }
            }
            if !store.customNames.isEmpty {
                Menu("Custom") { ForEach(store.customNames, id: \.self) { n in Button(n) { cmd = "custom:" + n } } }
            }
            if !store.layoutNames.isEmpty {
                Menu("Layouts") { ForEach(store.layoutNames, id: \.self) { n in Button(n) { cmd = "layout:" + n } } }
            }
        } label: {
            HStack(spacing: 8) {
                ActionIcon(unit: store.unit(of: cmd), size: 12)
                Text(store.name(of: cmd)).lineLimit(1).truncationMode(.tail)
                Spacer(minLength: 2)
                Image(systemName: "chevron.up.chevron.down").font(.system(size: 9, weight: .semibold)).foregroundStyle(Palette.dim)
            }
            .font(.system(size: 12.5))
            .foregroundStyle(.white)
            .padding(.horizontal, 8).padding(.vertical, 5)
            .frame(width: width, alignment: .leading)
            .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(Color.white.opacity(0.07)))
            .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(Color.white.opacity(0.1)))
            .contentShape(Rectangle())
        }
        .menuStyle(.button)
        .buttonStyle(.plain)
        .menuIndicator(.hidden)
        .fixedSize()
    }
}

/// unchanged | next | previous | name:<n> | number:<n>
struct DisplayPicker: View {
    @Binding var sel: String
    var body: some View {
        Picker("", selection: Binding(get: { sel == "unchanged" ? "" : sel }, set: { sel = $0 })) {
            Text("Current display").tag("")
            Text("Next display").tag("next")
            Text("Previous display").tag("previous")
            ForEach(Array(NSScreen.screens.enumerated()), id: \.offset) { i, s in
                Text("Display \(i + 1)").tag("number:\(i + 1)")
                Text(s.localizedName).tag("name:" + s.localizedName)
            }
            if !sel.isEmpty && !known(sel) { Text(sel.replacingOccurrences(of: "name:", with: "")).tag(sel) }
        }
        .labelsHidden().frame(width: 170)
    }
    private func known(_ s: String) -> Bool {
        ["", "unchanged", "next", "previous"].contains(s) || NSScreen.screens.enumerated().contains { "number:\($0.offset + 1)" == s || "name:" + $0.element.localizedName == s }
    }
}

struct NumberField: View {
    let placeholder: String
    @Binding var text: String
    var width: CGFloat = 64
    @State private var draft = ""
    var body: some View {
        TextField(placeholder, text: $draft, onCommit: { text = draft })
            .textFieldStyle(.roundedBorder).frame(width: width)
            .onAppear { draft = text }
            .onChange(of: text) { _, v in draft = v }
            .onChange(of: draft) { _, v in if v.isEmpty || parseFraction(v) != nil { text = v } }
    }
}

struct Slider2: View {
    @Binding var value: Double
    let range: ClosedRange<Double>
    var step: Double = 1
    var format: String = "%.0f"
    var body: some View {
        HStack {
            Slider(value: $value, in: range, step: step).frame(width: 160).controlSize(.small)
            Text(String(format: format, value)).font(.system(size: 11.5, design: .monospaced)).foregroundStyle(Palette.dim).frame(width: 46, alignment: .trailing)
        }
    }
}

/// Pick a running app or browse for one; stores a bundle id.
struct AppPicker: View {
    @Binding var bundle: String
    @Binding var name: String
    var allowAny = false
    var body: some View {
        Menu {
            if allowAny { Button("Any app") { bundle = "*"; name = "Any app" } }
            let apps = NSWorkspace.shared.runningApplications.filter { $0.activationPolicy == .regular && $0.bundleIdentifier != nil }
                .sorted { ($0.localizedName ?? "") < ($1.localizedName ?? "") }
            ForEach(apps, id: \.processIdentifier) { a in
                Button(a.localizedName ?? a.bundleIdentifier!) { bundle = a.bundleIdentifier!; name = a.localizedName ?? "" }
            }
            Divider()
            Button("Choose…") {
                let p = NSOpenPanel()
                p.allowedContentTypes = [.application]
                p.directoryURL = URL(fileURLWithPath: "/Applications")
                if p.runModal() == .OK, let url = p.url, let b = Bundle(url: url), let id = b.bundleIdentifier {
                    bundle = id
                    name = FileManager.default.displayName(atPath: url.path).replacingOccurrences(of: ".app", with: "")
                }
            }
        } label: {
            HStack(spacing: 6) {
                if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundle) {
                    Image(nsImage: NSWorkspace.shared.icon(forFile: url.path)).resizable().frame(width: 16, height: 16)
                }
                Text(bundle == "*" ? "Any app" : (name.isEmpty ? (bundle.isEmpty ? "Choose app" : bundle) : name))
            }
        }
        .menuStyle(.borderlessButton).frame(width: 190, alignment: .leading)
    }
}

/// One execution step: a preset, or a custom size and position, and where.
struct StepEditor: View {
    @EnvironmentObject var store: Store
    let kp: String
    var body: some View {
        let isPreset = store.value(kp + ".command") is String
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Picker("", selection: Binding(
                    get: { isPreset ? 0 : 1 },
                    set: { v in
                        if v == 0 { store.set(kp + ".command", "left-half"); store.set(kp + ".place", nil) }
                        else { store.set(kp + ".command", nil); store.set(kp + ".place", ["position": "center", "w": 0.5, "h": 0.5]) }
                    })) {
                    Text("Preset").tag(0)
                    Text("Custom size & position").tag(1)
                }
                .pickerStyle(.segmented).labelsHidden().frame(width: 260)
                Spacer()
                DisplayPicker(sel: store.string(kp + ".display"))
            }
            if isPreset {
                CommandPicker(cmd: store.string(kp + ".command"), allowNone: false)
            } else {
                HStack(spacing: 10) {
                    Picker("Position", selection: store.string(kp + ".place.position")) {
                        ForEach(["center", "top", "bottom", "left", "right", "topLeft", "topRight", "bottomLeft", "bottomRight", "custom", "centerOffset"], id: \.self) { p in
                            Text(positionName(p)).tag(p)
                        }
                    }
                    .frame(width: 210)
                    let pos = store.value(kp + ".place.position") as? String ?? "center"
                    if pos == "custom" || pos == "centerOffset" {
                        Text(pos == "custom" ? "X" : "ΔX").foregroundStyle(Palette.dim)
                        NumberField(placeholder: "keep", text: store.optNumber(kp + ".place.x"))
                        Text(pos == "custom" ? "Y" : "ΔY").foregroundStyle(Palette.dim)
                        NumberField(placeholder: "keep", text: store.optNumber(kp + ".place.y"))
                    }
                }
                HStack(spacing: 10) {
                    Text("Width").foregroundStyle(Palette.dim)
                    NumberField(placeholder: "keep", text: store.optNumber(kp + ".place.w"))
                    Text("Height").foregroundStyle(Palette.dim)
                    NumberField(placeholder: "keep", text: store.optNumber(kp + ".place.h"))
                    Text("≤ 1 is a fraction of the screen (1/3 works), > 1 is points, blank keeps the current value")
                        .font(.system(size: 10.5)).foregroundStyle(Palette.dim)
                }
                ActionIcon(unit: placementUnit(store.value(kp + ".place") as? [String: Any]), size: 34)
            }
        }
    }
}

func positionName(_ p: String) -> String {
    switch p {
    case "topLeft": return "Top Left"
    case "topRight": return "Top Right"
    case "bottomLeft": return "Bottom Left"
    case "bottomRight": return "Bottom Right"
    case "custom": return "Custom Origin"
    case "centerOffset": return "Center Offset"
    default: return p.capitalized
    }
}
