import AppKit
import SwiftUI

// The config is edited as plain JSON so the Rust daemon stays the single owner of its shape;
// every key exists because the daemon writes the full document on start.

let CMD = 1 << 20, OPT = 1 << 19, CTRL = 1 << 18, SHIFT = 1 << 17, FN = 1 << 23

struct ActionInfo: Decodable, Identifiable, Hashable {
    let id: String
    let name: String
    let category: String
    let holdable: Bool
    let unit: [Double]?
}

struct Catalog: Decodable {
    let actions: [ActionInfo]
    let categories: [String]
    let configPath: String
    let version: String
}

final class Store: ObservableObject {
    @Published var root: [String: Any] = [:]
    @Published var catalog: Catalog?
    @Published var defaults: [String: Any] = [:]
    @Published var error: String?
    let daemon: String
    var path: URL { URL(fileURLWithPath: catalog?.configPath ?? "") }
    private var pending: DispatchWorkItem?
    private var lastWritten: Date = .distantPast

    init(daemon: String) {
        self.daemon = daemon
        loadCatalog()
        reload()
        DistributedNotificationCenter.default().addObserver(forName: Notification.Name("app.grids.changed"), object: nil, queue: .main) { [weak self] _ in
            self?.reload()
        }
    }

    func loadCatalog() {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: daemon)
        p.arguments = ["--catalog"]
        let pipe = Pipe()
        p.standardOutput = pipe
        do {
            try p.run()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            p.waitUntilExit()
            catalog = try JSONDecoder().decode(Catalog.self, from: data)
            if let obj = try JSONSerialization.jsonObject(with: data) as? [String: Any] {
                defaults = obj["defaults"] as? [String: Any] ?? [:]
            }
        } catch {
            self.error = "Could not read Grids's catalog: \(error.localizedDescription)"
        }
    }

    func reload() {
        guard let data = try? Data(contentsOf: path), let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            if root.isEmpty { root = defaults }
            return
        }
        root = obj
    }

    // MARK: key-path access ("general.gap", "custom.0.name")

    private func parts(_ kp: String) -> [String] { kp.split(separator: ".").map(String.init) }

    func value(_ kp: String) -> Any? {
        var cur: Any? = root
        for p in parts(kp) {
            if let i = Int(p), let arr = cur as? [Any] { cur = i < arr.count ? arr[i] : nil }
            else { cur = (cur as? [String: Any])?[p] }
        }
        return cur is NSNull ? nil : cur
    }

    private func setIn(_ node: Any?, _ ps: ArraySlice<String>, _ v: Any?) -> Any {
        guard let head = ps.first else { return v ?? NSNull() }
        if let i = Int(head), var arr = node as? [Any] {
            if i < arr.count { arr[i] = setIn(arr[i], ps.dropFirst(), v) }
            return arr
        }
        var dict = node as? [String: Any] ?? [:]
        dict[head] = setIn(dict[head], ps.dropFirst(), v)
        return dict
    }

    func set(_ kp: String, _ v: Any?) {
        root = setIn(root, ArraySlice(parts(kp)), v) as? [String: Any] ?? root
        scheduleSave()
    }

    func bool(_ kp: String) -> Binding<Bool> {
        Binding(get: { (self.value(kp) as? NSNumber)?.boolValue ?? false }, set: { self.set(kp, $0) })
    }
    func double(_ kp: String) -> Binding<Double> {
        Binding(get: { (self.value(kp) as? NSNumber)?.doubleValue ?? 0 }, set: { self.set(kp, $0) })
    }
    func int(_ kp: String) -> Binding<Int> {
        Binding(get: { (self.value(kp) as? NSNumber)?.intValue ?? 0 }, set: { self.set(kp, $0) })
    }
    func string(_ kp: String) -> Binding<String> {
        Binding(get: { self.value(kp) as? String ?? "" }, set: { self.set(kp, $0) })
    }
    /// Optional number shown as text: blank = unset, "1/3" or "0.5" = fraction, "800" = points.
    func optNumber(_ kp: String) -> Binding<String> {
        Binding(
            get: {
                guard let n = self.value(kp) as? NSNumber else { return "" }
                let d = n.doubleValue
                return d == d.rounded() ? String(Int(d)) : String(format: "%.4g", d)
            },
            set: { txt in
                let t = txt.trimmingCharacters(in: .whitespaces)
                if t.isEmpty { self.set(kp, nil); return }
                if let v = parseFraction(t) { self.set(kp, v) }
            })
    }
    func optString(_ kp: String) -> Binding<String> {
        Binding(get: { self.value(kp) as? String ?? "" }, set: { self.set(kp, $0.isEmpty ? nil : $0) })
    }
    /// Renaming a custom entry or layout carries its shortcut to the new name.
    func nameBinding(_ kp: String, prefix: String) -> Binding<String> {
        Binding(get: { self.value(kp) as? String ?? "" }, set: { new in
            let old = self.value(kp) as? String ?? ""
            self.set(kp, new)
            if let sc = self.shortcut(prefix + old) {
                self.setShortcut(prefix + old, nil)
                self.setShortcut(prefix + new, sc)
            }
        })
    }
    func list(_ kp: String) -> [Any] { value(kp) as? [Any] ?? [] }
    func stringList(_ kp: String) -> [String] { list(kp).compactMap { $0 as? String } }

    func scheduleSave() {
        pending?.cancel()
        let item = DispatchWorkItem { [weak self] in self?.save() }
        pending = item
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.25, execute: item)
    }

    func save() {
        do {
            let data = try JSONSerialization.data(withJSONObject: root, options: [.prettyPrinted, .sortedKeys])
            try FileManager.default.createDirectory(at: path.deletingLastPathComponent(), withIntermediateDirectories: true)
            try data.write(to: path, options: .atomic)
            lastWritten = Date()
            DistributedNotificationCenter.default().postNotificationName(Notification.Name("app.grids.reload"), object: nil, userInfo: nil, deliverImmediately: true)
        } catch {
            self.error = "Could not save: \(error.localizedDescription)"
        }
    }

    // MARK: shortcuts

    func shortcut(_ cmd: String) -> (Int, Int)? {
        guard let d = (root["shortcuts"] as? [String: Any])?[cmd] as? [String: Any],
              let k = (d["keyCode"] as? NSNumber)?.intValue, let m = (d["modifierFlags"] as? NSNumber)?.intValue else { return nil }
        return (k, m)
    }

    func setShortcut(_ cmd: String, _ sc: (Int, Int)?) {
        var all = root["shortcuts"] as? [String: Any] ?? [:]
        if let sc { all[cmd] = ["keyCode": sc.0, "modifierFlags": sc.1] } else { all.removeValue(forKey: cmd) }
        root["shortcuts"] = all
        scheduleSave()
    }

    /// Which command already uses this shortcut, for conflict warnings.
    func owner(of sc: (Int, Int), except cmd: String) -> String? {
        guard let all = root["shortcuts"] as? [String: Any] else { return nil }
        for (k, v) in all where k != cmd {
            if let d = v as? [String: Any], (d["keyCode"] as? NSNumber)?.intValue == sc.0, (d["modifierFlags"] as? NSNumber)?.intValue == sc.1 { return k }
        }
        return nil
    }

    // MARK: commands for pickers

    func name(of cmd: String) -> String {
        if cmd.isEmpty { return "None" }
        if cmd.hasPrefix("custom:") { return String(cmd.dropFirst(7)) }
        if cmd.hasPrefix("layout:") { return "Layout: " + cmd.dropFirst(7) }
        if cmd.hasPrefix("compound:") {
            switch cmd {
            case "compound:thirds": return "Thirds, drag to centre for two thirds"
            case "compound:halves": return "Left or right half"
            case "compound:sixths": return "Sixths from corners; thirds"
            case "compound:fourths": return "Fourths columns"
            case "compound:half-corners": return "Half, top/bottom half near corners"
            default: return cmd
            }
        }
        return catalog?.actions.first { $0.id == cmd }?.name ?? cmd
    }

    func unit(of cmd: String) -> [Double]? {
        if cmd.hasPrefix("custom:") {
            let name = String(cmd.dropFirst(7))
            guard let spec = list("custom").compactMap({ $0 as? [String: Any] }).first(where: { $0["name"] as? String == name }),
                  let step = (spec["steps"] as? [[String: Any]])?.first else { return nil }
            if let c = step["command"] as? String { return unit(of: c) }
            return placementUnit(step["place"] as? [String: Any])
        }
        return catalog?.actions.first { $0.id == cmd }?.unit
    }

    var customNames: [String] { list("custom").compactMap { ($0 as? [String: Any])?["name"] as? String } }
    var layoutNames: [String] { list("layouts").compactMap { ($0 as? [String: Any])?["name"] as? String } }
}

func parseFraction(_ s: String) -> Double? {
    let t = s.trimmingCharacters(in: .whitespaces)
    if let slash = t.firstIndex(of: "/") {
        guard let a = Double(t[..<slash].trimmingCharacters(in: .whitespaces)), let b = Double(t[t.index(after: slash)...].trimmingCharacters(in: .whitespaces)), b != 0 else { return nil }
        return a / b
    }
    if t.hasSuffix("%"), let v = Double(t.dropLast()) { return v / 100 }
    return Double(t)
}

/// Unit-square preview of a custom placement on a 16:10 screen (mirrors the daemon's rules).
func placementUnit(_ p: [String: Any]?) -> [Double]? {
    guard let p else { return nil }
    let W = 1600.0, H = 1000.0
    func dim(_ k: String, _ total: Double, _ cur: Double) -> Double {
        guard let v = (p[k] as? NSNumber)?.doubleValue else { return cur }
        return abs(v) <= 1 ? v * total : v
    }
    let w = min(dim("w", W, 600), W), h = min(dim("h", H, 400), H)
    let cx = (W - w) / 2, cy = (H - h) / 2
    var x = cx, y = cy
    switch p["position"] as? String ?? "center" {
    case "top": y = 0
    case "bottom": y = H - h
    case "left": x = 0
    case "right": x = W - w
    case "topLeft": x = 0; y = 0
    case "topRight": x = W - w; y = 0
    case "bottomLeft": x = 0; y = H - h
    case "bottomRight": x = W - w; y = H - h
    case "custom": x = dim("x", W, 500); y = dim("y", H, 300)
    case "centerOffset": x = cx + dim("x", W, 0); y = cy + dim("y", H, 0)
    default: break
    }
    return [x / W, y / H, w / W, h / H]
}

// MARK: key names

func keyName(_ code: Int) -> String {
    let names: [Int: String] = [
        0: "A", 1: "S", 2: "D", 3: "F", 4: "H", 5: "G", 6: "Z", 7: "X", 8: "C", 9: "V", 11: "B", 12: "Q", 13: "W", 14: "E", 15: "R",
        16: "Y", 17: "T", 18: "1", 19: "2", 20: "3", 21: "4", 22: "6", 23: "5", 24: "=", 25: "9", 26: "7", 27: "-", 28: "8", 29: "0",
        30: "]", 31: "O", 32: "U", 33: "[", 34: "I", 35: "P", 36: "↩", 37: "L", 38: "J", 39: "'", 40: "K", 41: ";", 42: "\\", 43: ",",
        44: "/", 45: "N", 46: "M", 47: ".", 48: "⇥", 49: "Space", 50: "`", 51: "⌫", 53: "⎋", 65: "Num.", 67: "Num*", 69: "Num+",
        75: "Num/", 76: "⌤", 78: "Num-", 81: "Num=", 82: "Num0", 83: "Num1", 84: "Num2", 85: "Num3", 86: "Num4", 87: "Num5",
        88: "Num6", 89: "Num7", 91: "Num8", 92: "Num9", 96: "F5", 97: "F6", 98: "F7", 99: "F3", 100: "F8", 101: "F9", 103: "F11",
        105: "F13", 107: "F14", 109: "F10", 111: "F12", 113: "F15", 115: "↖", 116: "⇞", 117: "⌦", 118: "F4", 119: "↘", 120: "F2",
        121: "⇟", 122: "F1", 123: "←", 124: "→", 125: "↓", 126: "↑",
    ]
    return names[code] ?? "#\(code)"
}

func modsString(_ m: Int) -> String {
    var s = ""
    if m & FN != 0 { s += "fn" }
    if m & CTRL != 0 { s += "⌃" }
    if m & OPT != 0 { s += "⌥" }
    if m & SHIFT != 0 { s += "⇧" }
    if m & CMD != 0 { s += "⌘" }
    return s
}
