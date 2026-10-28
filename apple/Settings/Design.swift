import SwiftUI

// Dusk-over-glass look: one dark animated scene, frosted cards, quiet type.

enum Palette {
    static let ink = Color(red: 0.05, green: 0.06, blue: 0.11)
    static let dusk1 = Color(red: 0.16, green: 0.12, blue: 0.32)
    static let dusk2 = Color(red: 0.05, green: 0.23, blue: 0.33)
    static let ember = Color(red: 0.98, green: 0.55, blue: 0.40)
    static let accent = Color(red: 0.45, green: 0.62, blue: 1.0)
    static let mint = Color(red: 0.45, green: 0.88, blue: 0.70)
    static let line = Color.white.opacity(0.12)
    static let dim = Color.white.opacity(0.55)
}

struct DuskBackground: View {
    @Environment(\.controlActiveState) private var active

    var body: some View {
        TimelineView(.animation(minimumInterval: 1.0 / 20.0, paused: active == .inactive)) { ctx in
            let t = ctx.date.timeIntervalSinceReferenceDate
            GeometryReader { geo in
                ZStack {
                    LinearGradient(colors: [Palette.ink, Palette.dusk1.opacity(0.85), Palette.dusk2], startPoint: .topLeading, endPoint: .bottomTrailing)
                    blob(Palette.ember.opacity(0.32), size: geo.size.width * 0.55, x: 0.78 + 0.05 * sin(t / 9), y: 0.18 + 0.04 * cos(t / 11), in: geo.size)
                    blob(Palette.accent.opacity(0.30), size: geo.size.width * 0.65, x: 0.15 + 0.05 * cos(t / 13), y: 0.85 + 0.03 * sin(t / 7), in: geo.size)
                    blob(Palette.mint.opacity(0.16), size: geo.size.width * 0.45, x: 0.55 + 0.06 * sin(t / 15), y: 0.55 + 0.05 * cos(t / 10), in: geo.size)
                    // Horizon line: light on water.
                    LinearGradient(colors: [.clear, Palette.ember.opacity(0.10), .clear], startPoint: .top, endPoint: .bottom)
                        .frame(height: 140)
                        .position(x: geo.size.width / 2, y: geo.size.height * 0.62)
                        .blur(radius: 30)
                }
            }
        }
        .ignoresSafeArea()
    }

    private func blob(_ c: Color, size: CGFloat, x: Double, y: Double, in s: CGSize) -> some View {
        Circle().fill(c).frame(width: size, height: size).blur(radius: size * 0.28)
            .position(x: s.width * x, y: s.height * y)
    }
}

struct Glass<Content: View>: View {
    var padding: CGFloat = 16
    @ViewBuilder var content: Content
    var body: some View {
        VStack(alignment: .leading, spacing: 12) { content }
            .padding(padding)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(.ultraThinMaterial.opacity(0.85), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 18, style: .continuous).strokeBorder(Palette.line))
    }
}

struct GlassButtonStyle: ButtonStyle {
    var prominent = false
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 12.5, weight: .medium))
            .padding(.horizontal, 12).padding(.vertical, 6)
            .background(
                RoundedRectangle(cornerRadius: 9, style: .continuous)
                    .fill(prominent ? AnyShapeStyle(Palette.accent.opacity(configuration.isPressed ? 0.55 : 0.75)) : AnyShapeStyle(Color.white.opacity(configuration.isPressed ? 0.18 : 0.09)))
            )
            .overlay(RoundedRectangle(cornerRadius: 9, style: .continuous).strokeBorder(Color.white.opacity(0.14)))
            .foregroundStyle(.white)
    }
}

struct SectionTitle: View {
    let text: String
    var body: some View {
        Text(text.uppercased()).font(.system(size: 10.5, weight: .semibold)).tracking(1.1).foregroundStyle(Palette.dim)
    }
}

struct PaneHeader: View {
    let title: String
    let subtitle: String
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title).font(.system(size: 26, weight: .semibold, design: .rounded))
            Text(subtitle).font(.system(size: 13)).foregroundStyle(Palette.dim)
        }
        .padding(.bottom, 6)
    }
}

/// A labelled row: text on the left, a control on the right.
struct Row<Control: View>: View {
    let title: String
    var detail: String? = nil
    @ViewBuilder var control: Control
    var body: some View {
        HStack(alignment: .center, spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.system(size: 13))
                if let d = detail { Text(d).font(.system(size: 11)).foregroundStyle(Palette.dim).fixedSize(horizontal: false, vertical: true) }
            }
            Spacer(minLength: 8)
            control
        }
    }
}

struct ToggleRow: View {
    let title: String
    var detail: String? = nil
    @Binding var on: Bool
    var body: some View {
        Row(title: title, detail: detail) { Toggle("", isOn: $on).toggleStyle(.switch).labelsHidden().controlSize(.small) }
    }
}

struct NavItem: View {
    let title: String
    let symbol: String
    let tint: Color
    let selected: Bool
    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: symbol)
                .font(.system(size: 12, weight: .semibold))
                .frame(width: 24, height: 24)
                .background(RoundedRectangle(cornerRadius: 7, style: .continuous).fill(tint.opacity(selected ? 0.9 : 0.55)))
                .foregroundStyle(.white)
            Text(title).font(.system(size: 13, weight: selected ? .semibold : .regular))
            Spacer()
        }
        .padding(.horizontal, 10).padding(.vertical, 6)
        .background(RoundedRectangle(cornerRadius: 10, style: .continuous).fill(Color.white.opacity(selected ? 0.12 : 0)))
        .contentShape(Rectangle())
    }
}

/// Miniature screen with an action's region filled.
struct ActionIcon: View {
    let unit: [Double]?
    var size: CGFloat = 22
    var tint: Color = Palette.accent
    var body: some View {
        Canvas { ctx, s in
            let screen = CGRect(x: 1, y: 1, width: s.width - 2, height: s.height - 2)
            ctx.stroke(Path(roundedRect: screen, cornerRadius: 2.5), with: .color(.white.opacity(0.6)), lineWidth: 1.2)
            if let u = unit, u.count == 4 {
                let inner = screen.insetBy(dx: 2, dy: 2)
                let r = CGRect(x: inner.minX + inner.width * u[0], y: inner.minY + inner.height * u[1], width: max(inner.width * u[2], 1.5), height: max(inner.height * u[3], 1.5))
                ctx.fill(Path(roundedRect: r, cornerRadius: 1.2), with: .color(tint))
            }
        }
        .frame(width: size * 1.45, height: size)
    }
}

struct Chip: View {
    let text: String
    let on: Bool
    var body: some View {
        Text(text).font(.system(size: 12, weight: .semibold, design: .rounded))
            .frame(minWidth: 26).padding(.horizontal, 6).padding(.vertical, 4)
            .background(RoundedRectangle(cornerRadius: 7, style: .continuous).fill(on ? Palette.accent.opacity(0.8) : Color.white.opacity(0.07)))
            .overlay(RoundedRectangle(cornerRadius: 7, style: .continuous).strokeBorder(Color.white.opacity(on ? 0.3 : 0.1)))
    }
}
