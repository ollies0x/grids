import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct StashPane: View {
    @EnvironmentObject var store: Store
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            PaneHeader(title: "Stash", subtitle: "Tuck windows into the screen edge; touch the edge to slide them back out.")
            Glass {
                SectionTitle(text: "Shortcuts")
                ForEach(["stash-left", "stash-right", "stash-up", "stash-down", "unstash", "stash-all", "stash-all-but-front", "cycle-stashed", "toggle-stashed", "unstash-all"], id: \.self) { c in
                    Row(title: store.name(of: c)) { ShortcutRecorder(cmd: c) }
                }
            }
            Glass {
                SectionTitle(text: "Show a stashed window")
                Row(title: "After the cursor reaches the edge") { delay("stash.showDelay") }
                ToggleRow(title: "Only while holding ⌘", on: store.bool("stash.showRequiresCmd"))
                ToggleRow(title: "When its app becomes frontmost", on: store.bool("stash.unhideOnActivate"))
                SectionTitle(text: "Hide it again")
                ToggleRow(title: "When the cursor leaves the window", on: store.bool("stash.hideOnLeave"))
                Row(title: "After") { delay("stash.hideDelay") }
                ToggleRow(title: "Only while holding ⌘", on: store.bool("stash.hideRequiresCmd"))
                SectionTitle(text: "Unstash it")
                ToggleRow(title: "When dragged clear of the edge", on: store.bool("stash.unstashOnMove"))
                ToggleRow(title: "When any other action runs on it", on: store.bool("stash.unstashOnAction"))
            }
            Glass {
                SectionTitle(text: "All stash actions")
                ToggleRow(title: "Animate window movement", on: store.bool("stash.animate"))
                ToggleRow(title: "Hide stashed windows in the corner if left visible", detail: "macOS keeps some windows partly on screen, notably at an edge next to the Dock or another display.", on: store.bool("stash.hideInCorner"))
                ToggleRow(title: "Colour tabs", detail: "A coloured tab per stashed window, so several can share one edge.", on: store.bool("stash.colorTabs"))
                Row(title: "Visible sliver") { Slider2(value: store.double("stash.sliver"), range: 1...30) }
            }
        }
    }

    private func delay(_ kp: String) -> some View {
        Picker("", selection: store.double(kp)) {
            Text("Immediately").tag(0.0)
            ForEach(1...10, id: \.self) { i in Text(String(format: "%.1f s", Double(i) / 10)).tag(Double(i) / 10) }
        }.labelsHidden().frame(width: 130)
    }
}

struct PinPane: View {
    @EnvironmentObject var store: Store
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            PaneHeader(title: "Pin Mode", subtitle: "Keep one app in a column at the side of your main screen; every other action uses the space that's left. Rectangle's Todo mode, grown up.")
            Glass {
                ToggleRow(title: "Show Pin Mode in the menu", on: store.bool("pin.showInMenu"))
                Row(title: "Application") { AppPicker(bundle: store.string("pin.app"), name: store.string("pin.appName")) }
                Row(title: "Side") {
                    Picker("", selection: store.string("pin.side")) {
                        Text("Left").tag("left")
                        Text("Right").tag("right")
                    }.pickerStyle(.segmented).labelsHidden().frame(width: 140)
                }
                Row(title: "Width", detail: "Points, or a fraction such as 1/4.") { NumberField(placeholder: "400", text: store.optNumber("pin.width"), width: 80) }
                Row(title: "Toggle Pin Mode") { ShortcutRecorder(cmd: "pin") }
                Row(title: "Reflow", detail: "Put the pinned window back and move overlapping windows out of its way.") { ShortcutRecorder(cmd: "reflow-pin") }
            }
        }
    }
}

struct MenuPane: View {
    @EnvironmentObject var store: Store
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            PaneHeader(title: "Menu & Icon", subtitle: "What the menu bar menu holds, and how else to open it.")
            Glass {
                ToggleRow(title: "Hide the menu bar icon", detail: "Open Grids again from Finder or Spotlight to get back here.", on: store.bool("menu.hideIcon"))
                Row(title: "Icon") {
                    Picker("", selection: store.int("menu.iconVariant")) {
                        Text("Thirds").tag(0)
                        Text("Half").tag(1)
                        Text("Corner").tag(2)
                    }.pickerStyle(.segmented).labelsHidden().frame(width: 200)
                }
                ToggleRow(title: "Show icons beside actions", on: store.bool("menu.showIcons"))
            }
            Glass {
                SectionTitle(text: "Show the menu outside the menu bar")
                Row(title: "As a context menu with a shortcut", detail: "Acts on the front window.") { ShortcutRecorder(cmd: "show-menu") }
                Row(title: "With left click plus", detail: "Acts on the window under the cursor.") { ModifierPicker(flags: store.int("menu.clickModifiers")) }
            }
            Glass {
                SectionTitle(text: "Categories")
                ForEach((store.catalog?.categories ?? []).filter { $0 != "App" && $0 != "Pin" }, id: \.self) { cat in
                    HStack {
                        Toggle(cat, isOn: Binding(
                            get: { !store.stringList("menu.hiddenCategories").contains(cat) },
                            set: { on in toggleIn("menu.hiddenCategories", cat, !on) }))
                        Spacer()
                        Toggle("Submenu", isOn: Binding(
                            get: { store.stringList("menu.submenuCategories").contains(cat) },
                            set: { on in toggleIn("menu.submenuCategories", cat, on) })).font(.system(size: 11.5)).foregroundStyle(Palette.dim)
                    }
                }
                Divider().opacity(0.3)
                ToggleRow(title: "Custom Size & Position entries", on: store.bool("menu.showCustom"))
                ToggleRow(title: "Layouts", on: store.bool("menu.showLayouts"))
            }
        }
    }

    private func toggleIn(_ kp: String, _ v: String, _ include: Bool) {
        var l = store.stringList(kp)
        l.removeAll { $0 == v }
        if include { l.append(v) }
        store.set(kp, l)
    }
}

struct GeneralPane: View {
    @EnvironmentObject var store: Store
    @State private var notice: String?
    @AppStorage("settingsAlwaysOnTop") private var onTop = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            PaneHeader(title: "General", subtitle: "Grids \(store.catalog?.version ?? "")")
            Glass {
                ToggleRow(title: "Launch at login", on: store.bool("general.launchAtLogin"))
                ToggleRow(title: "Keep Settings on top", detail: "Floats this window above other apps while it is open.", on: $onTop)
            }
            Glass {
                SectionTitle(text: "Repeated commands")
                Picker("", selection: store.string("general.repeatMode")) {
                    Text("Do nothing").tag("none")
                    Text("Cycle through displays").tag("cycleDisplays")
                    Text("Cycle sizes on half actions").tag("cycleSizes")
                    Text("Move to adjacent display on left or right").tag("adjacentDisplay")
                    Text("Move to adjacent on left/right, or cycle size on half").tag("adjacentOrCycle")
                }.labelsHidden().pickerStyle(.radioGroup)
                let mode = store.value("general.repeatMode") as? String ?? ""
                if mode == "cycleSizes" || mode == "adjacentOrCycle" {
                    HStack(spacing: 6) {
                        Text("Sizes").foregroundStyle(Palette.dim)
                        ForEach(["2/3", "1/2", "1/3", "1/4", "3/4"], id: \.self) { f in
                            let on = store.stringList("general.cycleSizes").contains(f)
                            Button {
                                var l = store.stringList("general.cycleSizes")
                                if on { l.removeAll { $0 == f } } else { l.append(f) }
                                let order = ["2/3", "1/2", "1/3", "1/4", "3/4"]
                                store.set("general.cycleSizes", order.filter { l.contains($0) })
                            } label: { Chip(text: f, on: on) }.buttonStyle(.plain)
                        }
                    }
                }
                ToggleRow(title: "Reset repetitions when modifiers are released", on: store.bool("general.resetRepeatOnRelease"))
                ToggleRow(title: "Allow any keyboard shortcut", detail: "Removes the safeguard against single-key shortcuts that would swallow typing.", on: store.bool("general.allowAnyShortcut"))
            }
            Glass {
                SectionTitle(text: "Windows")
                Row(title: "Gaps between windows") { Slider2(value: store.double("general.gap"), range: 0...40) }
                ToggleRow(title: "Apply gaps to screen edges", on: store.bool("general.gapsOnEdges"))
                ToggleRow(title: "Move cursor with a window across displays", on: store.bool("general.moveCursorAcrossDisplays"))
                ToggleRow(title: "Double-click a title bar to maximise or restore", detail: "A true maximise, unlike macOS's Zoom. Set macOS's own title bar double-click to Do Nothing.", on: store.bool("general.doubleClickTitleBar"))
                ToggleRow(title: "Resize adjacent windows after dragging a window edge", on: store.bool("general.resizeAdjacent"))
                ToggleRow(title: "Adjust windows when the Dock size or position changes", on: store.bool("general.adjustForDock"))
                ToggleRow(title: "Restore a display's windows when it reconnects", on: store.bool("general.restoreDisplayLayout"))
                Row(title: "Stage Manager strip", detail: "Kept clear when Stage Manager is on. 0 ignores it; ≤ 1 is a fraction of the width.") {
                    NumberField(placeholder: "190", text: store.optNumber("general.stageManagerArea"), width: 70)
                }
            }
            Glass {
                SectionTitle(text: "Ignored apps")
                let ignored = store.stringList("general.ignoredApps")
                if ignored.isEmpty { Text("None. Use the menu's “Ignore” item while an app is in front.").font(.system(size: 12)).foregroundStyle(Palette.dim) }
                ForEach(ignored, id: \.self) { b in
                    HStack {
                        Text(b).font(.system(size: 12.5, design: .monospaced))
                        Spacer()
                        Button { store.set("general.ignoredApps", ignored.filter { $0 != b }) } label: { Image(systemName: "minus.circle") }.buttonStyle(.plain)
                    }
                }
            }
            Glass {
                SectionTitle(text: "Configuration")
                ToggleRow(title: "Sync configuration over iCloud", detail: "Keeps the settings file in iCloud Drive so every Mac with Grids shares it.", on: Binding(get: { icloudOn() }, set: { setICloud($0) }))
                HStack {
                    Button("Export…") { export() }.buttonStyle(GlassButtonStyle())
                    Button("Import…") { importConfig() }.buttonStyle(GlassButtonStyle())
                    Spacer()
                    Button("Restore default shortcuts & snap areas") {
                        store.root["shortcuts"] = store.defaults["shortcuts"]
                        store.root["snap"] = store.defaults["snap"]
                        store.root["winArrowModifiers"] = 0
                        store.save()
                    }.buttonStyle(GlassButtonStyle())
                }
                if let n = notice { Text(n).font(.system(size: 11.5)).foregroundStyle(Palette.mint) }
                Text(store.path.path).font(.system(size: 10.5, design: .monospaced)).foregroundStyle(Palette.dim).textSelection(.enabled)
            }
        }
    }

    private var support: URL { URL(fileURLWithPath: NSHomeDirectory() + "/Library/Application Support/Grids") }
    private var cloud: URL { URL(fileURLWithPath: NSHomeDirectory() + "/Library/Mobile Documents/com~apple~CloudDocs/Grids") }

    private func icloudOn() -> Bool { FileManager.default.fileExists(atPath: support.appendingPathComponent("icloud-sync").path) }

    private func setICloud(_ on: Bool) {
        let fm = FileManager.default
        let local = support.appendingPathComponent("config.json"), remote = cloud.appendingPathComponent("config.json")
        let flag = support.appendingPathComponent("icloud-sync")
        do {
            if on {
                try fm.createDirectory(at: cloud, withIntermediateDirectories: true)
                if !fm.fileExists(atPath: remote.path) { try fm.copyItem(at: local, to: remote) }
                fm.createFile(atPath: flag.path, contents: Data())
                notice = fm.fileExists(atPath: remote.path) ? "Syncing through iCloud Drive." : nil
            } else {
                if fm.fileExists(atPath: remote.path) {
                    if fm.fileExists(atPath: local.path) { try fm.removeItem(at: local) }
                    try fm.copyItem(at: remote, to: local)
                }
                try? fm.removeItem(at: flag)
                notice = "Using the settings on this Mac only."
            }
            post("app.grids.reload")
            store.loadCatalog()
            store.reload()
        } catch {
            notice = error.localizedDescription
        }
    }

    private func export() {
        let p = NSSavePanel()
        p.allowedContentTypes = [.json]
        p.nameFieldStringValue = "Grids.json"
        if p.runModal() == .OK, let url = p.url {
            try? FileManager.default.copyItem(at: store.path, to: url)
            notice = "Exported."
        }
    }

    private func importConfig() {
        let p = NSOpenPanel()
        p.allowedContentTypes = [.json]
        if p.runModal() == .OK, let url = p.url, let d = try? Data(contentsOf: url), let obj = try? JSONSerialization.jsonObject(with: d) as? [String: Any] {
            store.root = obj
            store.save()
            notice = "Imported."
        }
    }
}

/// Rectangle's Terminal-only preferences, as plain controls.
struct AdvancedPane: View {
    @EnvironmentObject var store: Store
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            PaneHeader(title: "Advanced", subtitle: "The settings Rectangle keeps behind Terminal commands.")
            Glass {
                SectionTitle(text: "Sizing")
                Row(title: "Almost Maximize width") { Slider2(value: store.double("tweaks.almostMaximizeWidth"), range: 0.5...1, step: 0.01, format: "%.2f") }
                Row(title: "Almost Maximize height") { Slider2(value: store.double("tweaks.almostMaximizeHeight"), range: 0.5...1, step: 0.01, format: "%.2f") }
                Row(title: "Larger/Smaller step") { Slider2(value: store.double("tweaks.sizeOffset"), range: 5...200) }
                Row(title: "Nudge step") { Slider2(value: store.double("tweaks.nudgeOffset"), range: 1...200) }
                Row(title: "Smallest width for Smaller") { Slider2(value: store.double("tweaks.minimumWindowWidth"), range: 0.05...0.9, step: 0.01, format: "%.2f") }
                Row(title: "Smallest height for Smaller") { Slider2(value: store.double("tweaks.minimumWindowHeight"), range: 0.05...0.9, step: 0.01, format: "%.2f") }
                ToggleRow(title: "Curtain resize", detail: "Larger/Smaller keep edges that touch the screen edge in place.", on: store.bool("tweaks.curtainResize"))
                Row(title: "Specified size") {
                    NumberField(placeholder: "1680", text: store.optNumber("tweaks.specifiedWidth"), width: 70)
                    Text("×").foregroundStyle(Palette.dim)
                    NumberField(placeholder: "1050", text: store.optNumber("tweaks.specifiedHeight"), width: 70)
                }
                Row(title: "Reveal Desktop Edge strip") { Slider2(value: store.double("tweaks.revealDesktopWidth"), range: 40...600) }
                Row(title: "Cascade offset") { Slider2(value: store.double("tweaks.cascadeOffset"), range: 8...80) }
            }
            Glass {
                SectionTitle(text: "Behaviour")
                ToggleRow(title: "Centre on the edge for Move Left/Right/Up/Down", on: store.bool("tweaks.centeredDirectionalMove"))
                ToggleRow(title: "Resize on directional move (½ → ⅔ → ⅓)", on: store.bool("tweaks.resizeOnDirectionalMove"))
                ToggleRow(title: "Center Half cycles ½ → ⅔ → ⅓", on: store.bool("tweaks.centerHalfCycles"))
                ToggleRow(title: "Gaps apply to Maximize", on: store.bool("tweaks.applyGapsToMaximize"))
                ToggleRow(title: "Gaps apply to Maximize Height", on: store.bool("tweaks.applyGapsToMaximizeHeight"))
                ToggleRow(title: "Double-click again restores", on: store.bool("tweaks.doubleClickRestore"))
                ToggleRow(title: "Order displays by x position only", detail: "For Next/Previous Display. Otherwise left to right, row by row.", on: store.bool("tweaks.screensOrderedByX"))
            }
            Glass {
                SectionTitle(text: "Screen edge gaps")
                HStack(spacing: 12) {
                    ForEach(Array(["Top", "Bottom", "Left", "Right"].enumerated()), id: \.offset) { i, l in
                        Text(l).foregroundStyle(Palette.dim)
                        NumberField(placeholder: "0", text: arrayNumber("tweaks.screenEdgeGaps", i), width: 46)
                    }
                }
                Row(title: "Top gap on screens with a notch") { NumberField(placeholder: "same", text: store.optNumber("tweaks.screenEdgeGapTopNotch"), width: 60) }
                ToggleRow(title: "Edge gaps on the main screen only", on: store.bool("tweaks.screenEdgeGapsMainOnly"))
            }
            Glass {
                SectionTitle(text: "Snap areas")
                HStack(spacing: 12) {
                    Text("Edge margin").foregroundStyle(Palette.dim)
                    ForEach(Array(["Top", "Bottom", "Left", "Right"].enumerated()), id: \.offset) { i, l in
                        Text(l).font(.system(size: 11)).foregroundStyle(Palette.dim)
                        NumberField(placeholder: "5", text: arrayNumber("tweaks.snapEdgeMargin", i), width: 40)
                    }
                }
                Row(title: "Corner size") { Slider2(value: store.double("tweaks.cornerSize"), range: 5...80) }
                Row(title: "Near-corner edge length") { Slider2(value: store.double("tweaks.shortEdge"), range: 40...400) }
            }
            Glass {
                SectionTitle(text: "Footprint")
                Row(title: "Opacity") { Slider2(value: store.double("tweaks.footprintAlpha"), range: 0.05...0.9, step: 0.01, format: "%.2f") }
                Row(title: "Border width") { Slider2(value: store.double("tweaks.footprintBorderWidth"), range: 0...8) }
                Row(title: "Colour") {
                    ColorPicker("", selection: Binding(
                        get: {
                            let c = store.list("tweaks.footprintColor").compactMap { ($0 as? NSNumber)?.doubleValue }
                            return c.count == 3 ? Color(red: c[0], green: c[1], blue: c[2]) : Palette.accent
                        },
                        set: { col in
                            let ns = NSColor(col).usingColorSpace(.sRGB) ?? .systemBlue
                            store.set("tweaks.footprintColor", [ns.redComponent, ns.greenComponent, ns.blueComponent])
                        }), supportsOpacity: false).labelsHidden()
                }
                ToggleRow(title: "Fade in", on: store.bool("tweaks.footprintFade"))
            }
            Glass {
                SectionTitle(text: "Spaces")
                Text("Next/Previous Space hold the window by its title bar and press the macOS Space-switch shortcut. Change these if you changed that shortcut.")
                    .font(.system(size: 12)).foregroundStyle(Palette.dim)
                Row(title: "Switch left") { Text(spaceKey("tweaks.spaceSwitchLeft")).foregroundStyle(Palette.dim) }
                Row(title: "Switch right") { Text(spaceKey("tweaks.spaceSwitchRight")).foregroundStyle(Palette.dim) }
            }
        }
    }

    private func spaceKey(_ kp: String) -> String {
        let k = (store.value(kp + ".keyCode") as? NSNumber)?.intValue ?? 0
        let m = (store.value(kp + ".modifierFlags") as? NSNumber)?.intValue ?? 0
        return modsString(m) + keyName(k)
    }

    private func arrayNumber(_ kp: String, _ i: Int) -> Binding<String> {
        Binding(
            get: { (store.list(kp)[safe: i] as? NSNumber).map { String(Int($0.doubleValue)) } ?? "0" },
            set: { t in
                var l = store.list(kp).compactMap { ($0 as? NSNumber)?.doubleValue }
                while l.count < 4 { l.append(0) }
                l[i] = Double(t) ?? 0
                store.set(kp, l)
            })
    }
}
