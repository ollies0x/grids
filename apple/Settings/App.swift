import AppKit
import SwiftUI

enum Pane: String, CaseIterable, Identifiable {
    case welcome, shortcuts, cursor, quick, snap, custom, layouts, stash, pin, menu, general, advanced
    var id: String { rawValue }
    var title: String {
        switch self {
        case .welcome: return "Welcome"
        case .shortcuts: return "Keyboard"
        case .cursor: return "Cursor"
        case .quick: return "Quick Throw"
        case .snap: return "Snap Areas"
        case .custom: return "Custom"
        case .layouts: return "Layouts"
        case .stash: return "Stash"
        case .pin: return "Pin Mode"
        case .menu: return "Menu & Icon"
        case .general: return "General"
        case .advanced: return "Advanced"
        }
    }
    var symbol: String {
        switch self {
        case .welcome: return "sparkles"
        case .shortcuts: return "keyboard"
        case .cursor: return "scope"
        case .quick: return "hare"
        case .snap: return "rectangle.inset.topleft.filled"
        case .custom: return "slider.horizontal.below.rectangle"
        case .layouts: return "square.grid.3x1.below.line.grid.1x2"
        case .stash: return "sidebar.squares.left"
        case .pin: return "pin"
        case .menu: return "menubar.rectangle"
        case .general: return "gearshape"
        case .advanced: return "wrench.and.screwdriver"
        }
    }
    var tint: Color {
        switch self {
        case .welcome: return Palette.ember
        case .shortcuts, .quick: return Palette.accent
        case .cursor, .snap: return Color(red: 0.55, green: 0.45, blue: 1.0)
        case .custom, .layouts: return Palette.mint
        case .stash, .pin: return Color(red: 0.95, green: 0.6, blue: 0.3)
        default: return Color.gray
        }
    }
}

final class Nav: ObservableObject {
    @Published var pane: Pane = .shortcuts
    @Published var focusLayout: String?

    func go(_ spec: String) {
        let parts = spec.split(separator: "/", maxSplits: 1).map(String.init)
        if let p = Pane(rawValue: parts.first ?? "") { pane = p }
        if parts.count > 1 { focusLayout = parts[1] }
    }
}

struct RootView: View {
    @EnvironmentObject var store: Store
    @EnvironmentObject var nav: Nav

    var body: some View {
        ZStack {
            DuskBackground()
            HStack(spacing: 0) {
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 9) {
                        if let icon = BrandIcon.image {
                            Image(nsImage: icon).resizable().interpolation(.high).frame(width: 22, height: 22)
                        } else {
                            ActionIcon(unit: [0, 0, 1.0 / 3, 1], size: 18, tint: Palette.ember)
                        }
                        Text("Grids").font(.system(size: 17, weight: .semibold, design: .rounded))
                    }
                    .padding(.horizontal, 10).padding(.top, 34).padding(.bottom, 14)
                    ForEach(Pane.allCases) { p in
                        NavItem(title: p.title, symbol: p.symbol, tint: p.tint, selected: nav.pane == p)
                            .onTapGesture { withAnimation(.easeOut(duration: 0.15)) { nav.pane = p } }
                    }
                    Spacer()
                }
                .frame(width: 200)
                .padding(.horizontal, 10)
                .background(Color.black.opacity(0.18))
                ScrollView {
                    VStack(alignment: .leading) {
                        if let e = store.error {
                            Glass { Text(e).foregroundStyle(Palette.ember) }
                        }
                        pane
                    }
                    .padding(.horizontal, 28).padding(.top, 34).padding(.bottom, 28)
                    .frame(maxWidth: 860, alignment: .leading)
                }
                .id(nav.pane)
            }
        }
        .foregroundStyle(.white)
        .tint(Palette.accent)
        .preferredColorScheme(.dark)
    }

    @ViewBuilder private var pane: some View {
        switch nav.pane {
        case .welcome: WelcomePane()
        case .shortcuts: ShortcutsPane()
        case .cursor: CursorPane()
        case .quick: QuickThrowPane()
        case .snap: SnapPane()
        case .custom: CustomPane()
        case .layouts: LayoutsPane(focus: $nav.focusLayout)
        case .stash: StashPane()
        case .pin: PinPane()
        case .menu: MenuPane()
        case .general: GeneralPane()
        case .advanced: AdvancedPane()
        }
    }
}

/// Grids's generated app icon: the helper's own Resources/AppIcon.icns, else the host app's.
enum BrandIcon {
    static let image: NSImage? = {
        if let own = Bundle.main.url(forResource: "AppIcon", withExtension: "icns"),
           let icon = NSImage(contentsOf: own) { return icon }
        let host = Bundle.main.bundleURL.deletingLastPathComponent().deletingLastPathComponent()
        return NSImage(contentsOf: host.appendingPathComponent("Resources/AppIcon.icns"))
    }()
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationWillFinishLaunching(_ notification: Notification) {
        if let icon = BrandIcon.image { NSApp.applicationIconImage = icon }
    }
}

/// Raises this window above other apps. Only ever our own window: a foreign window's level
/// cannot be changed from here (the write succeeds and everyone else still reads 0).
final class LevelView: NSView {
    var floating = false { didSet { apply() } }
    override func viewDidMoveToWindow() { super.viewDidMoveToWindow(); apply() }
    private func apply() { window?.level = floating ? .floating : .normal }
}

struct KeepOnTop: NSViewRepresentable {
    var on: Bool
    func makeNSView(context: Context) -> LevelView { let v = LevelView(); v.floating = on; return v }
    func updateNSView(_ v: LevelView, context: Context) { v.floating = on }
}

func argument(_ name: String) -> String? {
    let a = CommandLine.arguments
    guard let i = a.firstIndex(of: name), i + 1 < a.count else { return nil }
    return a[i + 1]
}

func daemonPath() -> String {
    if let d = argument("--daemon"), FileManager.default.isExecutableFile(atPath: d) { return d }
    // Contents/Helpers/Grids Settings.app -> Contents/MacOS/Grids
    let inBundle = Bundle.main.bundleURL.deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("MacOS/Grids").path
    if FileManager.default.isExecutableFile(atPath: inBundle) { return inBundle }
    return "/Applications/Grids.app/Contents/MacOS/Grids"
}

@main
struct SettingsApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var delegate
    @StateObject private var store = Store(daemon: daemonPath())
    @StateObject private var nav = Nav()
    @AppStorage("settingsAlwaysOnTop") private var onTop = false

    var body: some Scene {
        Window("Grids Settings", id: "settings") {
            RootView()
                .environmentObject(store)
                .environmentObject(nav)
                // Fixed, not minimum: the dusk background is drawn from blobs sized against the
                // window, so letting it stretch turns the backdrop into a smeared mess.
                .frame(width: 1040, height: 760)
                .background(KeepOnTop(on: onTop))
                .onAppear {
                    if let p = argument("--pane") { nav.go(p) }
                    DistributedNotificationCenter.default().addObserver(forName: Notification.Name("app.grids.settings.pane"), object: nil, queue: .main) { n in
                        if let p = n.object as? String { nav.go(p) }
                        NSApp.activate(ignoringOtherApps: true)
                    }
                }
        }
        .windowStyle(.hiddenTitleBar)
        .windowResizability(.contentSize)
    }
}
