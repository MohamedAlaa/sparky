// Sparky promo: motion compositor. Renders 1920x1080 @ 60 fps BGRA frames to
// stdout for ffmpeg. Real app recordings (rec/<script>/NNNNN.png, 1440x900)
// are composited with 3D camera moves, kinetic type, keycaps and particles.
//
//   swiftc -O render.swift -o render && ./render rec Sparky.png | ffmpeg -f rawvideo ...
import AppKit
import CoreImage
import CoreImage.CIFilterBuiltins

let args = CommandLine.arguments
let recDir = URL(fileURLWithPath: args[1])
let logoURL = URL(fileURLWithPath: args[2])
let onlyFrame: Int? = args.count > 3 ? Int(args[3]) : nil      // debug: render one frame to PNG
let W: CGFloat = 1920, H: CGFloat = 1080
let FPS = 60.0, DURATION = 36.0
let canvas = CGRect(x: 0, y: 0, width: W, height: H)
let ci = CIContext(options: [.workingColorSpace: CGColorSpace(name: CGColorSpace.sRGB)!, .cacheIntermediates: false])

// MARK: palette
func rgb(_ r: Int, _ g: Int, _ b: Int, _ a: CGFloat = 1) -> NSColor {
    NSColor(srgbRed: CGFloat(r) / 255, green: CGFloat(g) / 255, blue: CGFloat(b) / 255, alpha: a)
}
let violet = rgb(167, 139, 250), deep = rgb(124, 58, 237), ink = rgb(22, 18, 40), soft = rgb(196, 189, 230)

// MARK: easing
func clamp(_ x: Double, _ a: Double = 0, _ b: Double = 1) -> Double { min(max(x, a), b) }
func prog(_ t: Double, _ start: Double, _ dur: Double) -> Double { clamp((t - start) / dur) }
func easeOut(_ x: Double) -> Double { 1 - pow(1 - x, 3) }
func easeInOut(_ x: Double) -> Double { x < 0.5 ? 4 * x * x * x : 1 - pow(-2 * x + 2, 3) / 2 }
func expoOut(_ x: Double) -> Double { x >= 1 ? 1 : 1 - pow(2, -10 * x) }
func easeIn(_ x: Double) -> Double { x * x * x }
/// Damped spring 0→1 with overshoot.
func spring(_ x: Double, freq: Double = 4.2, damp: Double = 5.5) -> Double {
    x <= 0 ? 0 : 1 - exp(-damp * x) * cos(2 * .pi * freq * x)
}
func lerp(_ a: Double, _ b: Double, _ t: Double) -> Double { a + (b - a) * t }

// MARK: CoreImage helpers (CI is bottom-left; these take top-left canvas coords)
extension CIImage {
    func opacity(_ a: Double) -> CIImage {
        if a >= 0.999 { return self }
        let f = CIFilter.colorMatrix(); f.inputImage = self
        f.aVector = CIVector(x: 0, y: 0, z: 0, w: CGFloat(max(a, 0)))
        return f.outputImage!
    }
    func blur(_ r: Double) -> CIImage {
        r < 0.3 ? self : clampedToExtent().applyingGaussianBlur(sigma: r).cropped(to: extent.insetBy(dx: -CGFloat(r) * 3, dy: -CGFloat(r) * 3))
    }
    /// Places this image centered at a top-left canvas point, scaled and rotated.
    func placed(center: CGPoint, scale: Double = 1, rotate: Double = 0) -> CIImage {
        let e = extent
        var t = CGAffineTransform(translationX: -e.midX, y: -e.midY)
        t = t.concatenating(CGAffineTransform(scaleX: CGFloat(scale), y: CGFloat(scale)))
        t = t.concatenating(CGAffineTransform(rotationAngle: CGFloat(rotate)))
        t = t.concatenating(CGAffineTransform(translationX: center.x, y: H - center.y))
        return transformed(by: t)
    }
    func over(_ bg: CIImage) -> CIImage { composited(over: bg) }
    func add(_ bg: CIImage) -> CIImage {
        let f = CIFilter.additionCompositing(); f.inputImage = self; f.backgroundImage = bg; return f.outputImage!
    }
    func motionBlur(_ radius: Double, angle: Double) -> CIImage {
        if radius < 1 { return self }
        let f = CIFilter.motionBlur(); f.inputImage = clampedToExtent(); f.radius = Float(radius); f.angle = Float(angle)
        return f.outputImage!.cropped(to: extent.insetBy(dx: -CGFloat(radius), dy: -CGFloat(radius)))
    }
}
let clear = CIImage(color: .clear).cropped(to: canvas)

/// Renders AppKit drawing into a transparent CIImage of the given size.
func draw(_ size: CGSize, _ body: (CGContext) -> Void) -> CIImage {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(size.width), pixelsHigh: Int(size.height),
                               bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    let gc = NSGraphicsContext(bitmapImageRep: rep)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = gc
    body(gc.cgContext)
    NSGraphicsContext.restoreGraphicsState()
    return CIImage(cgImage: rep.cgImage!)
}

// MARK: text
var textCache: [String: CIImage] = [:]
func font(_ size: CGFloat, _ weight: NSFont.Weight) -> NSFont { NSFont.systemFont(ofSize: size, weight: weight) }
func textImage(_ s: String, _ size: CGFloat, _ weight: NSFont.Weight = .heavy, _ color: NSColor = .white,
               tracking: CGFloat = -1, shadow: Bool = true) -> CIImage {
    let key = "\(s)|\(size)|\(weight.rawValue)|\(color)|\(tracking)|\(shadow)"
    if let hit = textCache[key] { return hit }
    let str = NSAttributedString(string: s, attributes: [.font: font(size, weight), .foregroundColor: color, .kern: tracking])
    let sz = str.size()
    let pad: CGFloat = 40
    let img = draw(CGSize(width: ceil(sz.width + pad * 2), height: ceil(sz.height + pad * 2))) { c in
        if shadow { c.setShadow(offset: CGSize(width: 0, height: -6), blur: 24, color: NSColor.black.withAlphaComponent(0.45).cgColor) }
        str.draw(at: NSPoint(x: pad, y: pad))
    }
    textCache[key] = img
    return img
}
/// Width of a string's visible text (image width minus padding).
func textWidth(_ s: String, _ size: CGFloat, _ weight: NSFont.Weight = .heavy, tracking: CGFloat = -1) -> CGFloat {
    NSAttributedString(string: s, attributes: [.font: font(size, weight), .kern: tracking]).size().width
}

/// Words slam in one by one: scale down from big, blur to sharp, fade up.
struct Word { let text: String; let color: NSColor; let at: Double }
func slamLine(_ words: [Word], center: CGPoint, size: CGFloat, t: Double, weight: NSFont.Weight = .heavy,
              exitAt: Double? = nil, exitStyle: Int = 0) -> CIImage {
    let space = size * 0.28
    let widths = words.map { textWidth($0.text, size, weight) }
    let total = widths.reduce(0, +) + space * CGFloat(words.count - 1)
    var x = center.x - total / 2
    var out = clear
    for (i, w) in words.enumerated() {
        let p = prog(t, w.at, 0.42)
        let cx = x + widths[i] / 2
        x += widths[i] + space
        if p <= 0 { continue }
        let e = expoOut(p)
        var scale = lerp(1.9, 1, e), alpha = clamp(p * 4), blur = 16 * (1 - e)
        var dy = 0.0
        if let exitAt, t > exitAt {
            let q = prog(t, exitAt + Double(i) * 0.03, 0.32)
            if exitStyle == 0 { scale *= lerp(1, 2.4, easeIn(q)); blur += 22 * q; alpha *= 1 - q }
            else { dy = -120 * easeIn(q); alpha *= 1 - q; blur += 10 * q }
        }
        if alpha <= 0.01 { continue }
        let img = textImage(w.text, size, weight, w.color).blur(blur).opacity(alpha)
        out = img.placed(center: CGPoint(x: cx, y: center.y + CGFloat(dy)), scale: scale).over(out)
    }
    return out
}

// MARK: keycaps
var capCache: [String: CIImage] = [:]
func keycap(_ label: String, size: CGFloat, pressed: Double) -> CIImage {
    let key = "\(label)|\(size)|\(Int(pressed * 10))"
    if let hit = capCache[key] { return hit }
    let w = max(size, textWidth(label, size * 0.42, .semibold, tracking: 0) + size * 0.5)
    let depth = size * 0.09 * CGFloat(1 - pressed * 0.7)
    let img = draw(CGSize(width: w + 80, height: size + 80)) { c in
        let body = CGRect(x: 40, y: 40, width: w, height: size - size * 0.09 + depth)
        let face = CGRect(x: 40, y: 40 + depth, width: w, height: size - size * 0.09)
        let r = size * 0.2
        c.setShadow(offset: CGSize(width: 0, height: -size * 0.12 * CGFloat(1 - pressed * 0.6)), blur: size * 0.35,
                    color: NSColor.black.withAlphaComponent(0.55).cgColor)
        c.addPath(CGPath(roundedRect: body, cornerWidth: r, cornerHeight: r, transform: nil))
        c.setFillColor(rgb(170, 166, 196).cgColor); c.fillPath()
        c.setShadow(offset: .zero, blur: 0, color: nil)
        let path = CGPath(roundedRect: face, cornerWidth: r, cornerHeight: r, transform: nil)
        c.saveGState(); c.addPath(path); c.clip()
        let g = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                           colors: [rgb(255, 255, 255).cgColor, rgb(226, 224, 238).cgColor] as CFArray, locations: [0, 1])!
        c.drawLinearGradient(g, start: CGPoint(x: 0, y: face.maxY), end: CGPoint(x: 0, y: face.minY), options: [])
        c.restoreGState()
        c.addPath(path); c.setStrokeColor(NSColor.white.withAlphaComponent(0.9).cgColor); c.setLineWidth(size * 0.012); c.strokePath()
        let str = NSAttributedString(string: label, attributes: [.font: font(size * 0.42, .semibold), .foregroundColor: ink])
        let s = str.size()
        str.draw(at: NSPoint(x: face.midX - s.width / 2, y: face.midY - s.height / 2))
    }
    capCache[key] = img
    return img
}

// MARK: assets
let logo = CIImage(contentsOf: logoURL)!
let ring = draw(CGSize(width: 1000, height: 1000)) { c in
    c.setStrokeColor(NSColor.white.cgColor); c.setLineWidth(10)
    c.strokeEllipse(in: CGRect(x: 20, y: 20, width: 960, height: 960))
}
let dot = draw(CGSize(width: 48, height: 48)) { c in
    let g = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                       colors: [NSColor.white.cgColor, rgb(167, 139, 250, 0).cgColor] as CFArray, locations: [0, 1])!
    c.drawRadialGradient(g, startCenter: CGPoint(x: 24, y: 24), startRadius: 0, endCenter: CGPoint(x: 24, y: 24), endRadius: 24, options: [])
}
let cursor = draw(CGSize(width: 60, height: 80)) { c in
    // macOS-style arrow: tip at (8, 72) in this bitmap (top-left after flip).
    let p = CGMutablePath()
    let pts: [CGPoint] = [(0, 0), (0, 34), (8, 26), (14, 40), (20, 37), (14, 24), (25, 24)].map { CGPoint(x: 8 + $0.0 * 1.6, y: 72 - $0.1 * 1.6) }
    p.addLines(between: pts); p.closeSubpath()
    c.setShadow(offset: CGSize(width: 0, height: -3), blur: 8, color: NSColor.black.withAlphaComponent(0.5).cgColor)
    c.addPath(p); c.setFillColor(NSColor.black.cgColor); c.fillPath()
    c.setShadow(offset: .zero, blur: 0, color: nil)
    c.addPath(p); c.setStrokeColor(NSColor.white.cgColor); c.setLineWidth(3); c.strokePath()
}
func chip(_ label: String, _ color: NSColor) -> CIImage {
    let key = "chip|\(label)"
    if let hit = capCache[key] { return hit }
    let f = font(30, .bold)
    let str = NSAttributedString(string: label, attributes: [.font: f, .foregroundColor: NSColor.white])
    let s = str.size()
    let img = draw(CGSize(width: s.width + 120, height: 120)) { c in
        let r = CGRect(x: 30, y: 30, width: s.width + 60, height: 58)
        c.setShadow(offset: CGSize(width: 0, height: -8), blur: 24, color: color.withAlphaComponent(0.8).cgColor)
        c.addPath(CGPath(roundedRect: r, cornerWidth: 29, cornerHeight: 29, transform: nil))
        c.setFillColor(color.cgColor); c.fillPath()
        c.setShadow(offset: .zero, blur: 0, color: nil)
        str.draw(at: NSPoint(x: r.minX + 30, y: r.midY - s.height / 2))
    }
    capCache[key] = img
    return img
}
let sweepBand = draw(CGSize(width: 240, height: H)) { c in
    let g = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: [NSColor.white.withAlphaComponent(0).cgColor,
        NSColor.white.withAlphaComponent(0.65).cgColor, NSColor.white.withAlphaComponent(0).cgColor] as CFArray, locations: [0, 0.5, 1])!
    c.drawLinearGradient(g, start: .zero, end: CGPoint(x: 240, y: 0), options: [])
}
let titleBar = draw(CGSize(width: 1440, height: 36)) { c in
    c.setFillColor(rgb(236, 236, 242).cgColor); c.fill(CGRect(x: 0, y: 0, width: 1440, height: 36))
    for (i, col) in [rgb(255, 95, 87), rgb(254, 188, 46), rgb(40, 200, 64)].enumerated() {
        c.setFillColor(col.cgColor); c.fillEllipse(in: CGRect(x: 18 + CGFloat(i) * 22, y: 11, width: 14, height: 14))
    }
}
let windowMask = draw(CGSize(width: 1440, height: 936)) { c in
    c.addPath(CGPath(roundedRect: CGRect(x: 0, y: 0, width: 1440, height: 936), cornerWidth: 18, cornerHeight: 18, transform: nil))
    c.setFillColor(NSColor.white.cgColor); c.fillPath()
}

// MARK: recordings
var pointerTracks: [String: [(CGPoint, Bool)]] = [:]
func recFrame(_ script: String, _ f: Int) -> CIImage {
    let dir = recDir.appendingPathComponent(script)
    let count = (try? FileManager.default.contentsOfDirectory(atPath: dir.path).filter { $0.hasSuffix(".png") }.count) ?? 1
    let n = max(0, min(f, count - 1))
    return CIImage(contentsOf: dir.appendingPathComponent(String(format: "%05d.png", n)))!
}
func pointer(_ script: String, _ f: Int) -> (CGPoint, Bool)? {
    if pointerTracks[script] == nil {
        let csv = (try? String(contentsOf: recDir.appendingPathComponent(script).appendingPathComponent("pointer.csv"), encoding: .utf8)) ?? ""
        pointerTracks[script] = csv.split(separator: "\n").dropFirst().compactMap { line in
            let p = line.split(separator: ",").map { Double($0) ?? 0 }
            return p.count == 4 ? (CGPoint(x: p[1], y: p[2]), p[3] > 0) : nil
        }
    }
    let track = pointerTracks[script]!
    return track.isEmpty ? nil : track[max(0, min(f, track.count - 1))]
}
/// App frame inside a Mac window (1440 x 936, title bar on top).
func window(_ app: CIImage) -> CIImage {
    let content = app.transformed(by: CGAffineTransform(translationX: 0, y: 0))
        .over(titleBar.transformed(by: CGAffineTransform(translationX: 0, y: 900)))
    let f = CIFilter.blendWithMask(); f.inputImage = content; f.backgroundImage = CIImage(color: .clear); f.maskImage = windowMask
    return f.outputImage!.cropped(to: CGRect(x: 0, y: 0, width: 1440, height: 936))
}

// MARK: 3D camera
struct Cam {
    var focus = CGPoint(x: 720, y: 468)   // window-local point (top-left coords) placed at `at`
    var at = CGPoint(x: W / 2, y: H / 2 + 20)
    var zoom = 0.95, rx = 0.0, ry = 0.0, rz = 0.0
}
/// Projects a window-local point (top-left coords, 1440 x 936) to the canvas (top-left coords).
func project(_ p: CGPoint, _ c: Cam) -> CGPoint {
    let f = 2200.0
    var x = Double(p.x - c.focus.x), y = Double(p.y - c.focus.y), z = 0.0
    // Rotate about X (tilt back), then Y (turn), then Z.
    let (sx, cx) = (sin(c.rx), cos(c.rx)); (y, z) = (y * cx - z * sx, y * sx + z * cx)
    let (sy, cy) = (sin(c.ry), cos(c.ry)); (x, z) = (x * cy + z * sy, -x * sy + z * cy)
    let (sz, cz) = (sin(c.rz), cos(c.rz)); (x, y) = (x * cz - y * sz, x * sz + y * cz)
    x *= c.zoom; y *= c.zoom; z *= c.zoom
    let k = f / (f + z)
    return CGPoint(x: Double(c.at.x) + x * k, y: Double(c.at.y) + y * k)
}
func ciPoint(_ p: CGPoint) -> CGPoint { CGPoint(x: p.x, y: H - p.y) }
/// Window (with shadow and glow) through the camera; `blur` adds motion blur.
func shoot(_ app: CIImage, _ cam: Cam, alpha: Double = 1, mblur: Double = 0, angle: Double = 0) -> CIImage {
    let win = window(app)
    let corners = [CGPoint(x: 0, y: 0), CGPoint(x: 1440, y: 0), CGPoint(x: 0, y: 936), CGPoint(x: 1440, y: 936)].map { project($0, cam) }
    let pt = CIFilter.perspectiveTransform()
    pt.inputImage = win
    pt.topLeft = ciPoint(corners[0]); pt.topRight = ciPoint(corners[1])
    pt.bottomLeft = ciPoint(corners[2]); pt.bottomRight = ciPoint(corners[3])
    var img = pt.outputImage!
    let ps = CIFilter.perspectiveTransform()
    ps.inputImage = windowMask.applyingFilter("CIColorMatrix", parameters: ["inputRVector": CIVector(x: 0, y: 0, z: 0, w: 0),
        "inputGVector": CIVector(x: 0, y: 0, z: 0, w: 0), "inputBVector": CIVector(x: 0, y: 0, z: 0, w: 0),
        "inputAVector": CIVector(x: 0, y: 0, z: 0, w: 0.62)])
    ps.topLeft = pt.topLeft; ps.topRight = pt.topRight; ps.bottomLeft = pt.bottomLeft; ps.bottomRight = pt.bottomRight
    let shadow = ps.outputImage!.transformed(by: CGAffineTransform(translationX: 0, y: -40 * cam.zoom)).blur(46 * cam.zoom)
    let glow = ps.outputImage!.applyingFilter("CIColorMatrix", parameters: [
        "inputRVector": CIVector(x: 0, y: 0, z: 0, w: 0), "inputGVector": CIVector(x: 0, y: 0, z: 0, w: 0),
        "inputBVector": CIVector(x: 0, y: 0, z: 0, w: 0), "inputAVector": CIVector(x: 0, y: 0, z: 0, w: 0.4),
        "inputBiasVector": CIVector(x: 0.55, y: 0.4, z: 1, w: 0)]).blur(90)
    img = img.over(shadow)
    if mblur > 0 { img = img.motionBlur(mblur, angle: angle) }
    return img.opacity(alpha).over(glow.opacity(alpha * 0.55))
}

// MARK: background
let grainBase = CIFilter.randomGenerator().outputImage!
func background(_ t: Double, energy: Double = 1) -> CIImage {
    var bg = CIImage(color: CIColor(red: 0.027, green: 0.02, blue: 0.06)).cropped(to: canvas)
    let blobs: [(Double, Double, Double, CIColor)] = [
        (0.3, 0.35, 0.0, CIColor(red: 0.42, green: 0.24, blue: 0.95, alpha: 0.55)),
        (0.75, 0.6, 2.1, CIColor(red: 0.16, green: 0.33, blue: 0.95, alpha: 0.35)),
        (0.5, 0.9, 4.2, CIColor(red: 0.85, green: 0.25, blue: 0.75, alpha: 0.22)),
    ]
    for (bx, by, phase, color) in blobs {
        let g = CIFilter.radialGradient()
        g.center = CGPoint(x: W * (bx + 0.08 * sin(t * 0.5 + phase)), y: H * (1 - by + 0.08 * cos(t * 0.4 + phase)))
        g.radius0 = 0; g.radius1 = Float(820 * (0.9 + 0.1 * energy))
        g.color0 = color; g.color1 = CIColor(red: 0, green: 0, blue: 0, alpha: 0)
        bg = g.outputImage!.cropped(to: canvas).over(bg)
    }
    return bg
}
func grain(_ t: Double) -> CIImage {
    let shift = CGAffineTransform(translationX: CGFloat(Int(t * 997) % 400), y: CGFloat(Int(t * 619) % 400))
    return grainBase.transformed(by: shift).cropped(to: canvas)
        .applyingFilter("CIColorMatrix", parameters: ["inputRVector": CIVector(x: 1, y: 0, z: 0, w: 0),
            "inputGVector": CIVector(x: 1, y: 0, z: 0, w: 0), "inputBVector": CIVector(x: 1, y: 0, z: 0, w: 0),
            "inputAVector": CIVector(x: 0, y: 0, z: 0, w: 0.035)])
}
func flash(_ t: Double, _ at: Double, _ dur: Double = 0.35, _ peak: Double = 0.9) -> CIImage? {
    let p = (t - at) / dur
    guard p >= 0, p < 1 else { return nil }
    return CIImage(color: CIColor(red: 1, green: 1, blue: 1, alpha: CGFloat(peak * pow(1 - p, 2)))).cropped(to: canvas)
}
/// Dark gradient behind captions so they stay readable over the app.
/// edge: 0 top, 1 bottom, 2 right. `reach` is the fraction of the canvas covered.
func scrim(_ edge: Int, reach: Double, alpha: Double) -> CIImage {
    // A gradient between two fully clear colors produces no image at all.
    if alpha <= 0.001 { return clear }
    let g = CIFilter.linearGradient()
    let dark = CIColor(red: 0.03, green: 0.02, blue: 0.07, alpha: CGFloat(alpha))
    let none = CIColor(red: 0.03, green: 0.02, blue: 0.07, alpha: 0)
    switch edge {
    case 0: g.point0 = CGPoint(x: 0, y: H * (1 - reach * 0.55)); g.point1 = CGPoint(x: 0, y: H * (1 - reach))
    case 1: g.point0 = CGPoint(x: 0, y: H * reach * 0.55); g.point1 = CGPoint(x: 0, y: H * reach)
    default: g.point0 = CGPoint(x: W * (1 - reach * 0.6), y: 0); g.point1 = CGPoint(x: W * (1 - reach), y: 0)
    }
    g.color0 = dark; g.color1 = none
    return (g.outputImage ?? clear).cropped(to: canvas)
}
/// Decaying camera shake after an impact.
func shake(_ t: Double, _ at: Double, _ amp: Double = 18) -> CGPoint {
    let d = t - at
    guard d >= 0, d < 0.5 else { return .zero }
    let k = amp * exp(-9 * d)
    return CGPoint(x: k * sin(d * 90), y: k * cos(d * 77))
}

// MARK: particles (deterministic)
struct Rng { var s: UInt64; mutating func next() -> Double { s = s &* 6364136223846793005 &+ 1442695040888963407; return Double(s >> 33) / Double(1 << 31) } }
func burst(_ t: Double, at start: Double, center: CGPoint, count: Int, speed: Double, seed: UInt64, color: Bool = true) -> CIImage {
    let d = t - start
    guard d >= 0, d < 2.2 else { return clear }
    var r = Rng(s: seed), out = clear
    for _ in 0..<count {
        let a = r.next() * 2 * .pi, v = speed * (0.35 + r.next()), size = 0.3 + r.next() * 0.9
        let dist = v * (1 - exp(-2.6 * d)) / 2.6
        let p = CGPoint(x: Double(center.x) + cos(a) * dist, y: Double(center.y) + sin(a) * dist + 40 * d * d)
        let alpha = clamp(1 - d / (1.2 + r.next()))
        if alpha <= 0 { continue }
        var img = dot.placed(center: p, scale: size)
        if color && r.next() > 0.5 {
            img = img.applyingFilter("CIColorMatrix", parameters: ["inputRVector": CIVector(x: 0.7, y: 0, z: 0, w: 0),
                "inputGVector": CIVector(x: 0, y: 0.6, z: 0, w: 0), "inputBVector": CIVector(x: 0, y: 0, z: 1, w: 0)])
        }
        out = img.opacity(alpha).add(out)
    }
    return out
}

/// Keystroke HUD: keycaps pop up at the bottom when a key is pressed.
func hud(_ t: Double, _ presses: [(Double, String)]) -> CIImage {
    var out = clear
    for (at, label) in presses {
        let d = t - at
        guard d >= -0.02, d < 0.75 else { continue }
        let s = spring(d / 0.6, freq: 2.2, damp: 6)
        let alpha = clamp(d * 12) * clamp((0.75 - d) * 6)
        let pressed = d < 0.12 ? 1.0 : 0.0
        out = keycap(label, size: 96, pressed: pressed).opacity(alpha)
            .placed(center: CGPoint(x: W / 2, y: H - 120 + 40 * (1 - s)), scale: 0.6 + 0.4 * s).over(out)
    }
    return out
}

// MARK: scenes
func frame(_ t: Double) -> CIImage {
    var energy = 1.0
    if t > 4 && t < 31 { energy = 1 + 0.15 * pow(1 - (t - 4).truncatingRemainder(dividingBy: 0.5) / 0.5, 3) } // pulses on the beat
    var img = background(t, energy: energy)
    var shakeAt: [Double] = [4.0, 10.5, 31.0]
    var offset = CGPoint.zero
    for s in shakeAt { let o = shake(t, s, s == 10.5 ? 26 : 16); offset.x += o.x; offset.y += o.y }
    shakeAt.removeAll()
    var layer = clear

    // A. Hook: 0–4 s
    if t < 4.1 {
        let push = easeIn(prog(t, 1.85, 0.4))
        let line1 = slamLine([Word(text: "I", color: .white, at: 0.15), Word(text: "got", color: .white, at: 0.3),
                              Word(text: "fed up", color: violet, at: 0.48)],
                             center: CGPoint(x: W / 2, y: 470), size: 120, t: t)
        let line2 = slamLine([Word(text: "with", color: .white, at: 0.95), Word(text: "email", color: .white, at: 1.1),
                              Word(text: "clients.", color: .white, at: 1.25)],
                             center: CGPoint(x: W / 2, y: 620), size: 120, t: t)
        if push < 1 {
            var hook = line1.over(line2)
            hook = hook.blur(24 * push).opacity(1 - push)
                .transformed(by: CGAffineTransform(translationX: -W / 2, y: -H / 2).concatenating(
                    CGAffineTransform(scaleX: 1 + 1.6 * push, y: 1 + 1.6 * push)).concatenating(CGAffineTransform(translationX: W / 2, y: H / 2)))
            layer = hook.over(layer)
        }
        if t > 2.1 {
            let out = easeIn(prog(t, 3.65, 0.35))
            var built = slamLine([Word(text: "So", color: .white, at: 2.2), Word(text: "I", color: .white, at: 2.32),
                                  Word(text: "built", color: .white, at: 2.44), Word(text: "my own.", color: violet, at: 2.62)],
                                 center: CGPoint(x: W / 2, y: 500), size: 150, t: t)
            // Typewriter: "over a weekend."
            let typed = "over a weekend."
            let n = Int(clamp((t - 3.0) / 0.045, 0, Double(typed.count)))
            if n > 0 {
                let s = String(typed.prefix(n)) + (Int(t * 8) % 2 == 0 && n < typed.count ? "▍" : "")
                let img = textImage(s, 58, .medium, soft, tracking: 2, shadow: false)
                built = img.transformed(by: CGAffineTransform(translationX: W / 2 - textWidth(typed, 58, .medium, tracking: 2) / 2 - 40,
                                                              y: H - 640 - img.extent.height / 2)).over(built)
            }
            built = built.blur(30 * out).opacity(1 - out)
                .transformed(by: CGAffineTransform(translationX: -W / 2, y: -H / 2).concatenating(
                    CGAffineTransform(scaleX: 1 + 3 * out, y: 1 + 3 * out)).concatenating(CGAffineTransform(translationX: W / 2, y: H / 2)))
            layer = built.over(layer)
        }
    }

    // B. Logo drop: 4–6.6 s
    if t >= 4.0 && t < 7.0 {
        let d = t - 4.0
        let out = easeInOut(prog(t, 6.15, 0.55))
        let center = CGPoint(x: W / 2, y: lerp(370, 300, out))
        var group = clear
        for (k, delay) in [0.0, 0.12, 0.3].enumerated() {
            let q = prog(d, delay, 0.9)
            if q > 0 && q < 1 {
                group = ring.opacity((1 - q) * (k == 2 ? 0.35 : 0.7)).placed(center: center, scale: lerp(0.1, 1.6 + Double(k) * 0.4, expoOut(q))).add(group)
            }
        }
        group = burst(t, at: 4.0, center: center, count: 70, speed: 1300, seed: 7).add(group)
        let ls = spring(d / 0.9) * lerp(1, 0.55, out)
        let rot = -0.35 * (1 - spring(d / 0.9, freq: 2.5))
        group = logo.placed(center: center, scale: 0.3 * ls, rotate: rot).over(group)
        // "Sparky" letters rise one by one.
        let word = "Sparky"
        let size: CGFloat = 170 * CGFloat(lerp(1, 0.6, out))
        var x = W / 2 - textWidth(word, size) / 2
        let y = lerp(690, 600, out)
        for (i, ch) in word.enumerated() {
            let s = String(ch)
            let wch = textWidth(s, size)
            let q = prog(d, 0.35 + Double(i) * 0.06, 0.5)
            if q > 0 {
                let e = expoOut(q)
                group = textImage(s, size).blur(12 * (1 - e)).opacity(clamp(q * 3))
                    .placed(center: CGPoint(x: x + wch / 2, y: y + 90 * (1 - e))).over(group)
            }
            x += wch - 1
        }
        let tq = prog(d, 1.0, 0.6)
        if tq > 0 {
            group = textImage("A calmer kind of email.", 52 * CGFloat(lerp(1, 0.7, out)), .medium, soft, tracking: 0)
                .opacity(easeOut(tq) * (1 - out)).placed(center: CGPoint(x: W / 2, y: lerp(810, 700, out) + 30 * (1 - easeOut(tq)))).over(group)
        }
        let fade = 1 - prog(t, 6.55, 0.35)
        layer = group.opacity(fade).over(layer)
    }

    // C. App reveal + navigation: 6.5–10 s (nav recording from frame 0)
    if t >= 6.4 && t < 10.15 {
        let f = Int((t - 6.5) * FPS)
        let rise = spring(prog(t, 6.4, 1.4), freq: 1.4, damp: 4.2)
        let out = easeIn(prog(t, 9.85, 0.3))
        var cam = Cam()
        cam.zoom = lerp(0.62, 0.86, rise) + 0.05 * prog(t, 7.5, 2.5) + 1.2 * out
        cam.rx = lerp(1.0, 0.16, rise) - 0.1 * easeInOut(prog(t, 7.6, 2.2))
        cam.ry = lerp(0, -0.14, rise) + 0.24 * easeInOut(prog(t, 7.6, 2.3))
        cam.at = CGPoint(x: W / 2, y: H / 2 + 60 + 900 * (1 - rise))
        layer = shoot(recFrame("nav", f), cam, alpha: 1 - out, mblur: 60 * out, angle: .pi / 2).over(layer)
        layer = scrim(0, reach: 0.22, alpha: 0.7 * clamp(prog(t, 6.8, 0.3)) * (1 - out)).over(layer)
        let cap = slamLine([Word(text: "Built", color: .white, at: 7.0), Word(text: "for", color: .white, at: 7.25),
                            Word(text: "speed.", color: violet, at: 7.5)],
                           center: CGPoint(x: W / 2, y: 120), size: 96, t: t, exitAt: 9.75)
        layer = cap.over(layer)
        layer = hud(t, [(6.5 + 24 / FPS, "J"), (6.5 + 46 / FPS, "J"), (6.5 + 68 / FPS, "J"), (6.5 + 96 / FPS, "K"),
                        (6.5 + 130 / FPS, "E"), (6.5 + 175 / FPS, "E")]).over(layer)
    }

    // D. ⌘K keycaps (10–11) then the palette (11–14)
    if t >= 9.95 && t < 11.15 {
        let through = easeIn(prog(t, 10.85, 0.25))
        var caps = clear
        for (i, (label, at)) in [("⌘", 10.0), ("K", 10.2)].enumerated() {
            let q = prog(t, at, 0.45)
            if q <= 0 { continue }
            let e = expoOut(q)
            let pressed = t > 10.5 && t < 10.68 ? 1.0 : 0.0
            let x = W / 2 + (i == 0 ? -170 : 170)
            caps = keycap(label, size: 300, pressed: pressed).blur(18 * (1 - e)).opacity(clamp(q * 3))
                .placed(center: CGPoint(x: x, y: H / 2 + 20 + (pressed > 0 ? 18 : 0)), scale: lerp(2.6, 1, e), rotate: (i == 0 ? -0.08 : 0.08) * (1 - e)).over(caps)
        }
        caps = burst(t, at: 10.5, center: CGPoint(x: W / 2, y: H / 2 + 60), count: 40, speed: 1100, seed: 11).add(caps)
        caps = caps.transformed(by: CGAffineTransform(translationX: -W / 2, y: -H / 2).concatenating(
            CGAffineTransform(scaleX: 1 + 5 * through, y: 1 + 5 * through)).concatenating(CGAffineTransform(translationX: W / 2, y: H / 2)))
            .opacity(1 - through)
        layer = caps.over(layer)
    }
    if t >= 10.95 && t < 14.15 {
        let f = 30 + Int((t - 11.0) * FPS)
        let enter = expoOut(prog(t, 10.95, 0.6))
        let out = easeIn(prog(t, 13.85, 0.25))
        var cam = Cam()
        cam.focus = CGPoint(x: 720, y: 330)
        cam.zoom = lerp(2.6, 1.42, enter) + 0.1 * prog(t, 11.5, 2.5)
        cam.ry = lerp(0.35, 0.1, enter) - 0.2 * easeInOut(prog(t, 11.4, 2.6))
        cam.rx = 0.08
        cam.at = CGPoint(x: W / 2 - 2200 * out, y: H / 2 + 70)
        layer = shoot(recFrame("palette", f), cam, alpha: clamp(enter * 3), mblur: 140 * out, angle: 0).over(layer)
        layer = scrim(0, reach: 0.3, alpha: 0.95 * clamp(prog(t, 11.2, 0.3)) * (1 - out)).over(layer)
        let cap = slamLine([Word(text: "One", color: .white, at: 11.4), Word(text: "keystroke.", color: .white, at: 11.6),
                            Word(text: "Everything.", color: violet, at: 12.1)],
                           center: CGPoint(x: W / 2, y: 110), size: 84, t: t, exitAt: 13.75, exitStyle: 1)
        layer = cap.over(layer)
    }

    // E. Search: 14–18 s
    if t >= 13.9 && t < 18.15 {
        let f = 10 + Int((t - 14.0) * FPS)
        let enter = expoOut(prog(t, 13.92, 0.45))
        let dolly = easeInOut(prog(t, 14.9, 2.2))
        let out = easeIn(prog(t, 17.8, 0.3))
        var cam = Cam()
        cam.focus = CGPoint(x: lerp(720, 720, dolly), y: lerp(468, 280, dolly))
        cam.zoom = lerp(0.9, 1.55, dolly) + 2.2 * out
        cam.ry = -0.12 + 0.18 * dolly
        cam.rx = 0.12 - 0.06 * dolly
        cam.at = CGPoint(x: W / 2 + 2200 * (1 - enter), y: H / 2 + 80)
        layer = shoot(recFrame("search", f), cam, alpha: 1 - out, mblur: 140 * (1 - enter) + 50 * out, angle: 0).over(layer)
        layer = scrim(0, reach: 0.3, alpha: 0.95 * clamp(prog(t, 14.3, 0.3)) * (1 - out)).over(layer)
        let cap = slamLine([Word(text: "Search", color: .white, at: 14.5), Word(text: "every", color: .white, at: 14.65),
                            Word(text: "folder.", color: .white, at: 14.8)],
                           center: CGPoint(x: W / 2, y: 110), size: 84, t: t, exitAt: 15.9, exitStyle: 1)
        layer = cap.over(layer)
        let inst = slamLine([Word(text: "Instantly.", color: violet, at: 16.0)], center: CGPoint(x: W / 2, y: 110), size: 110, t: t, exitAt: 17.6)
        layer = inst.over(layer)
        layer = hud(t, [(14.0 + 10 / FPS, "/")]).over(layer)
    }

    // F. Swipe close-up: 18–23 s
    if t >= 17.95 && t < 23.2 {
        let f = Int((t - 18.0) * FPS)
        let enter = expoOut(prog(t, 17.95, 0.55))
        let out = easeInOut(prog(t, 22.75, 0.45))
        var cam = Cam()
        cam.focus = CGPoint(x: 430, y: 374)
        cam.zoom = lerp(3.2, 1.95, enter) - 0.9 * out
        cam.ry = 0.18 * (1 - out)
        cam.rx = 0.06
        cam.rz = -0.02
        cam.at = CGPoint(x: W / 2 - 180 + 180 * out, y: H / 2 + 40)
        layer = shoot(recFrame("swipe", f), cam, alpha: clamp(enter * 2.5) * (1 - out)).over(layer)
        if let (p, down) = pointer("swipe", f), p.x >= 0, out < 0.5 {
            let c = project(CGPoint(x: p.x, y: p.y + 36), cam)
            layer = cursor.placed(center: CGPoint(x: c.x + 14, y: c.y + 22), scale: down ? 0.85 : 1).over(layer)
            for click in [100, 160, 244] {
                let q = Double(f - click) / 24
                if q >= 0 && q < 1 {
                    layer = ring.opacity(0.8 * (1 - q)).placed(center: c, scale: 0.02 + 0.09 * expoOut(q)).add(layer)
                }
            }
        }
        layer = scrim(2, reach: 0.52, alpha: 0.96 * clamp(prog(t, 18.2, 0.3)) * (1 - out)).over(layer)
        // Words land on the actions.
        let words: [(String, NSColor, Double, Double)] = [("Swipe.", .white, 18.5, 19.6), ("Gone.", violet, 19.8, 20.55),
                                                          ("Drag.", .white, 20.7, 21.9), ("Done.", violet, 22.1, 22.9)]
        for (w, color, at, exit) in words {
            layer = slamLine([Word(text: w, color: color, at: at)], center: CGPoint(x: 1560, y: 540), size: 150, t: t, exitAt: exit).over(layer)
        }
    }

    // G. AI labels: 23–27 s
    if t >= 22.9 && t < 27.1 {
        let enter = easeOut(prog(t, 22.9, 0.5))
        var cam = Cam()
        cam.focus = CGPoint(x: 720, y: 468)
        cam.zoom = lerp(0.68, 0.8, enter) + 0.04 * prog(t, 23.5, 3.5)
        cam.ry = 0.34 - 0.08 * easeInOut(prog(t, 23.2, 3.8))
        cam.rx = 0.06
        cam.at = CGPoint(x: 540, y: H / 2 + 20)
        let app = recFrame("nav", 0)
        layer = shoot(app, cam, alpha: clamp(prog(t, 22.9, 0.25) * 1.5) * (1 - prog(t, 26.95, 0.1))).over(layer)
        let chips: [(String, NSColor, Int)] = [("Work", rgb(124, 58, 237), 0), ("Work", rgb(124, 58, 237), 1),
                                               ("Newsletters", rgb(13, 148, 136), 2), ("Personal", rgb(219, 39, 119), 3)]
        for (i, (label, color, row)) in chips.enumerated() {
            let at = 23.5 + Double(i) * 0.5
            let q = prog(t, at, 0.5)
            if q <= 0 { continue }
            let s = spring(q, freq: 1.8, damp: 5)
            let target = project(CGPoint(x: 560, y: 36 + 202 + 92 * CGFloat(row) + 66), cam)
            let from = CGPoint(x: W + 200, y: target.y - 160)
            let p = CGPoint(x: lerp(Double(from.x), Double(target.x), s), y: lerp(Double(from.y), Double(target.y), s))
            layer = chip(label, color).placed(center: p, scale: 0.75 * cam.zoom / 1.3, rotate: 0.25 * (1 - s)).over(layer)
            layer = burst(t, at: at + 0.18, center: target, count: 14, speed: 420, seed: UInt64(30 + i)).add(layer)
        }
        layer = scrim(2, reach: 0.5, alpha: 0.9).opacity(clamp(prog(t, 22.9, 0.3)) * (1 - prog(t, 26.95, 0.1))).over(layer)
        let cap = slamLine([Word(text: "AI", color: violet, at: 23.3), Word(text: "sorts", color: .white, at: 23.45),
                            Word(text: "your", color: .white, at: 23.6), Word(text: "inbox.", color: .white, at: 23.75)],
                           center: CGPoint(x: 1480, y: 430), size: 84, t: t, exitAt: 26.85)
        let cap2 = slamLine([Word(text: "On-device.", color: .white, at: 25.0), Word(text: "Private.", color: violet, at: 25.5)],
                            center: CGPoint(x: 1480, y: 550), size: 72, t: t, exitAt: 26.85)
        let small = textImage("Your mail never leaves your Mac.", 40, .medium, soft, tracking: 0)
            .opacity(easeOut(prog(t, 26.0, 0.4)) * (1 - prog(t, 26.85, 0.15)))
            .placed(center: CGPoint(x: 1480, y: 650 + 20 * (1 - easeOut(prog(t, 26.0, 0.4)))))
        layer = small.over(cap2.over(cap.over(layer)))
    }

    // H. Montage: 27–31 s, a cut on every beat.
    if t >= 27.0 && t < 31.0 {
        let beat = Int((t - 27.0) / 0.5)
        let local = (t - 27.0) - Double(beat) * 0.5
        let cuts: [(String, Int, CGPoint, Double, Double, Double)] = [
            ("accent", 5, CGPoint(x: 720, y: 468), 0.85, 0.35, 0.1),
            ("accent", 35, CGPoint(x: 300, y: 330), 1.6, -0.3, 0.05),
            ("shortcuts", 170, CGPoint(x: 720, y: 360), 1.35, 0.2, 0.12),
            ("accent", 65, CGPoint(x: 1040, y: 400), 1.5, -0.25, -0.05),
            ("reply", 150, CGPoint(x: 1040, y: 690), 1.55, 0.22, 0.1),
            ("accent", 95, CGPoint(x: 300, y: 600), 1.7, 0.3, -0.08),
            ("accent", 125, CGPoint(x: 720, y: 468), 1.0, -0.35, 0.12),
            ("accent", 155, CGPoint(x: 720, y: 468), 0.82, 0.0, 0.2),
        ]
        let (script, f0, focus, zoom, ry, rx) = cuts[min(beat, cuts.count - 1)]
        var cam = Cam()
        cam.focus = focus
        cam.zoom = zoom * (1.08 - 0.08 * expoOut(local / 0.5))
        cam.ry = ry * (1 - 0.4 * local / 0.5)
        cam.rx = rx
        layer = shoot(recFrame(script, f0 + Int(local * FPS)), cam).over(layer)
        if let fl = flash(t, 27.0 + Double(beat) * 0.5, 0.12, 0.5) { layer = fl.over(layer) }
        layer = scrim(1, reach: 0.36, alpha: 0.95).over(layer)
        let words: [(String, NSColor, Double)] = [("Your colors.", .white, 27.0), ("Your keys.", .white, 28.0),
                                                  ("Your words.", .white, 29.0), ("Your way.", violet, 30.0)]
        for (w, color, at) in words {
            layer = slamLine([Word(text: w, color: color, at: at)], center: CGPoint(x: W / 2, y: H - 130), size: 104, t: t, exitAt: at + 0.85).over(layer)
        }
    }

    // I. Outro: 31–36 s
    if t >= 31.0 {
        let d = t - 31.0
        let center = CGPoint(x: W / 2, y: 330)
        var group = burst(t, at: 31.0, center: center, count: 90, speed: 1500, seed: 99)
        for (k, delay) in [0.0, 0.14].enumerated() {
            let q = prog(d, delay, 1.0)
            if q > 0 && q < 1 { group = ring.opacity((1 - q) * 0.6).placed(center: center, scale: lerp(0.1, 1.8 + Double(k) * 0.5, expoOut(q))).add(group) }
        }
        let ls = spring(d / 0.8)
        group = logo.placed(center: center, scale: 0.28 * ls).over(group)
        group = slamLine([Word(text: "Sparky", color: .white, at: 31.25)], center: CGPoint(x: W / 2, y: 560), size: 150, t: t).over(group)
        let tq = easeOut(prog(d, 0.9, 0.5))
        group = textImage("Only the features you want. Built in a weekend.", 46, .medium, soft, tracking: 0)
            .opacity(tq).placed(center: CGPoint(x: W / 2, y: 680 + 24 * (1 - tq))).over(group)
        // Pill with a light sweep.
        let pq = spring(prog(d, 1.5, 0.8), freq: 2, damp: 6)
        if pq > 0 {
            let pill = chip("Open source — coming soon", deep)
            var p = pill.placed(center: CGPoint(x: W / 2, y: 810), scale: 1.25 * pq)
            let sweepX = lerp(-400, Double(W) + 400, prog(d, 2.4, 1.0))
            // A narrow bright band (clear → white → clear) crossing the pill.
            let band = sweepBand.transformed(by: CGAffineTransform(translationX: CGFloat(sweepX) - 120, y: 0))
            let mask = CIFilter.blendWithAlphaMask()
            mask.inputImage = band; mask.backgroundImage = clear; mask.maskImage = p
            p = mask.outputImage!.add(p)
            group = p.over(group)
        }
        let fade = 1 - prog(t, 35.2, 0.8)
        layer = group.opacity(fade).over(layer)
    }

    // Global impacts.
    for at in [4.0, 10.5, 31.0] { if let fl = flash(t, at, at == 10.5 ? 0.25 : 0.45, at == 10.5 ? 0.55 : 0.95) { layer = fl.over(layer) } }
    if let fl = flash(t, 11.0, 0.2, 0.6) { layer = fl.over(layer) }
    layer = layer.transformed(by: CGAffineTransform(translationX: offset.x, y: offset.y))
    img = layer.over(img)
    img = img.applyingFilter("CIVignette", parameters: [kCIInputIntensityKey: 0.65, kCIInputRadiusKey: 1.6]).cropped(to: canvas)
    img = grain(t).over(img)
    let fadeIn = prog(t, 0, 0.3)
    if fadeIn < 1 { img = img.opacity(fadeIn).over(CIImage(color: .black).cropped(to: canvas)) }
    if t > 35.2 { img = img.opacity(1 - prog(t, 35.2, 0.8)).over(CIImage(color: .black).cropped(to: canvas)) }
    return img.cropped(to: canvas)
}

// MARK: output
let rowBytes = Int(W) * 4
var buffer = [UInt8](repeating: 0, count: rowBytes * Int(H))
let sRGB = CGColorSpace(name: CGColorSpace.sRGB)!
if let only = onlyFrame {
    let t = Double(only) / FPS
    let cg = ci.createCGImage(frame(t), from: canvas)!
    let rep = NSBitmapImageRep(cgImage: cg)
    try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "preview-\(only).png"))
    exit(0)
}
let total = Int(DURATION * FPS)
let out = FileHandle.standardOutput
for n in 0..<total {
    autoreleasepool {
        ci.render(frame(Double(n) / FPS), toBitmap: &buffer, rowBytes: rowBytes, bounds: canvas, format: .BGRA8, colorSpace: sRGB)
        out.write(Data(buffer))
        if n % 120 == 0 { FileHandle.standardError.write("frame \(n)/\(total)\n".data(using: .utf8)!) }
    }
}
