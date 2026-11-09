// Render Sparky's existing purple S mark into the macOS app icon family.
import AppKit
let output = CommandLine.arguments[1]
let sizes = [(16, "16x16"), (32, "16x16@2x"), (32, "32x32"), (64, "32x32@2x"), (128, "128x128"), (256, "128x128@2x"), (256, "256x256"), (512, "256x256@2x"), (512, "512x512"), (1024, "512x512@2x")]
try FileManager.default.createDirectory(atPath: output, withIntermediateDirectories: true)
for (size, name) in sizes {
    let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    let context = NSGraphicsContext(bitmapImageRep: bitmap)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context
    let scale = CGFloat(size) / 1024
    context.cgContext.scaleBy(x: scale, y: scale)
    NSColor(calibratedRed: 113/255, green: 91/255, blue: 211/255, alpha: 1).setFill()
    NSBezierPath(roundedRect: NSRect(x: 64, y: 64, width: 896, height: 896), xRadius: 190, yRadius: 190).fill()
    let text = "S" as NSString
    let attributes: [NSAttributedString.Key: Any] = [.font: NSFont.systemFont(ofSize: 740, weight: .regular), .foregroundColor: NSColor.white]
    let bounds = text.size(withAttributes: attributes)
    text.draw(at: NSPoint(x: (1024-bounds.width)/2, y: (1024-bounds.height)/2 + 12), withAttributes: attributes)
    NSGraphicsContext.restoreGraphicsState()
    try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "\(output)/icon_\(name).png"))
}
// ICNS uses a big-endian container of PNG representations.
func bigEndian(_ value: Int) -> Data {
    var number = UInt32(value).bigEndian
    return withUnsafeBytes(of: &number) { Data($0) }
}
var chunks = Data()
for (type, name) in [("icp4", "16x16"), ("icp5", "32x32"), ("icp6", "32x32@2x"), ("ic07", "128x128"), ("ic08", "256x256"), ("ic09", "512x512"), ("ic10", "512x512@2x"), ("ic11", "16x16@2x"), ("ic12", "32x32@2x"), ("ic13", "128x128@2x"), ("ic14", "256x256@2x")] {
    let png = try Data(contentsOf: URL(fileURLWithPath: "\(output)/icon_\(name).png"))
    chunks.append(Data(type.utf8))
    chunks.append(bigEndian(png.count + 8))
    chunks.append(png)
}
var icon = Data("icns".utf8)
icon.append(bigEndian(chunks.count + 8))
icon.append(chunks)
let assets = URL(fileURLWithPath: CommandLine.arguments[0]).deletingLastPathComponent()
try icon.write(to: assets.appendingPathComponent("Sparky.icns"))
try Data(contentsOf: URL(fileURLWithPath: "\(output)/icon_512x512@2x.png")).write(to: assets.appendingPathComponent("Sparky.png"))
