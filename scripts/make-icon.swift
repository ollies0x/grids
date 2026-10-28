// Renders the app icon: a dusk-glass squircle with a window divided by gridss.
import AppKit

func render(_ px: Int) -> Data {
    let s = CGFloat(px)
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: px, pixelsHigh: px, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    // Below 64 px the gradient, the glow and hairline gridss all land inside one pixel and the
    // mark turns to noise (it read as "garbage" in the small icons System Settings draws).
    let small = s < 64
    let inset = small ? (s * 0.09).rounded() : s * 0.09
    let body = NSRect(x: inset, y: inset, width: s - 2 * inset, height: s - 2 * inset)
    let shape = NSBezierPath(roundedRect: body, xRadius: body.width * 0.225, yRadius: body.width * 0.225)
    let bar = small ? max(1.0, (s * 0.06).rounded()) : s * 0.022
    // Whole-pixel geometry at small sizes: a fractional edge is a grey fringe at 16 px.
    let wx = (s * 0.25).rounded()
    NSGraphicsContext.current?.saveGraphicsState()
    shape.addClip()
    if small {
        NSColor(red: 0.10, green: 0.11, blue: 0.23, alpha: 1).setFill()
        body.fill()
    } else {
        NSGradient(colors: [
            NSColor(red: 0.05, green: 0.06, blue: 0.13, alpha: 1),
            NSColor(red: 0.20, green: 0.14, blue: 0.38, alpha: 1),
            NSColor(red: 0.06, green: 0.27, blue: 0.38, alpha: 1),
        ])!.draw(in: body, angle: -60)
    }
    // Ember glow low on the horizon.
    if !small {
        let glow = NSGradient(colors: [NSColor(red: 0.98, green: 0.55, blue: 0.40, alpha: 0.55), NSColor(red: 0.98, green: 0.55, blue: 0.40, alpha: 0)])!
        glow.draw(fromCenter: NSPoint(x: body.midX, y: body.minY + body.height * 0.22), radius: 0, toCenter: NSPoint(x: body.midX, y: body.minY + body.height * 0.22), radius: body.width * 0.62, options: [])
    }
    // The window: glass pane with two gridss; the left pane lit.
    let w = small
        ? NSRect(x: wx, y: wx, width: s - 2 * wx, height: s - 2 * wx)
        : body.insetBy(dx: body.width * 0.17, dy: body.height * 0.22)
    let frame = NSBezierPath(roundedRect: w, xRadius: s * 0.035, yRadius: s * 0.035)
    NSColor(white: 1, alpha: 0.10).setFill()
    frame.fill()
    let paneW = (w.width - 2 * bar) / 3
    let lit = NSRect(x: w.minX, y: w.minY, width: paneW, height: w.height)
    NSGraphicsContext.current?.saveGraphicsState()
    frame.addClip()
    if small {
        NSColor(red: 1.0, green: 0.58, blue: 0.42, alpha: 1).setFill()
        lit.fill()
    } else {
        NSGradient(colors: [NSColor(red: 1.0, green: 0.68, blue: 0.48, alpha: 0.95), NSColor(red: 0.98, green: 0.45, blue: 0.40, alpha: 0.9)])!.draw(in: lit, angle: 90)
    }
    NSGraphicsContext.current?.restoreGraphicsState()
    NSColor(white: 1, alpha: 0.85).setFill()
    for i in 1...2 {
        NSRect(x: w.minX + CGFloat(i) * paneW + CGFloat(i - 1) * bar, y: w.minY, width: bar, height: w.height).fill()
    }
    NSColor(white: 1, alpha: 0.85).setStroke()
    frame.lineWidth = bar
    frame.stroke()
    NSGraphicsContext.current?.restoreGraphicsState()
    if !small {
        NSColor(white: 1, alpha: 0.16).setStroke()
        shape.lineWidth = s * 0.006
        shape.stroke()
    }
    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])!
}

let out = CommandLine.arguments[1]
try? FileManager.default.createDirectory(atPath: out, withIntermediateDirectories: true)
for (name, px) in [("16x16", 16), ("16x16@2x", 32), ("32x32", 32), ("32x32@2x", 64), ("128x128", 128), ("128x128@2x", 256), ("256x256", 256), ("256x256@2x", 512), ("512x512", 512), ("512x512@2x", 1024)] {
    try! render(px).write(to: URL(fileURLWithPath: "\(out)/icon_\(name).png"))
}
