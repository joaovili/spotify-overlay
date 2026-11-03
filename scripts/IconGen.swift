// Gera os PNGs do icone. Uso: swift Tools/IconGen.swift <diretorio-iconset>
// Desenha com CGContext (sem NSImage/NSGraphicsContext) para funcionar em runner sem
// sessao grafica: NSImage.lockFocus depende de WindowServer.
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let outputDirectory = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "build/Icon.iconset"
try? FileManager.default.createDirectory(atPath: outputDirectory, withIntermediateDirectories: true)

func color(_ r: CGFloat, _ g: CGFloat, _ b: CGFloat) -> CGColor {
    CGColor(srgbRed: r, green: g, blue: b, alpha: 1)
}

/// Fundo escuro arredondado, triangulo de play e barra de progresso verde: le a 16 px e
/// nao se confunde com o circulo verde do Spotify.
func drawIcon(size: Int) -> CGImage? {
    let dimension = CGFloat(size)
    let space = CGColorSpaceCreateDeviceRGB()
    guard let context = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8,
                                  bytesPerRow: 0, space: space,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else {
        return nil
    }
    context.setAllowsAntialiasing(true)
    context.interpolationQuality = .high

    let inset = dimension * 0.06
    let rect = CGRect(x: inset, y: inset, width: dimension - inset * 2, height: dimension - inset * 2)
    let radius = dimension * 0.22

    context.addPath(CGPath(roundedRect: rect, cornerWidth: radius, cornerHeight: radius, transform: nil))
    context.setFillColor(color(0.094, 0.094, 0.094))
    context.fillPath()

    context.addPath(CGPath(roundedRect: rect.insetBy(dx: dimension * 0.006, dy: dimension * 0.006),
                           cornerWidth: radius, cornerHeight: radius, transform: nil))
    context.setStrokeColor(color(0.16, 0.16, 0.16))
    context.setLineWidth(max(1, dimension * 0.012))
    context.strokePath()

    let triangleHeight = dimension * 0.34
    let triangleWidth = dimension * 0.30
    let centerY = dimension * 0.55
    let triangle = CGMutablePath()
    triangle.move(to: CGPoint(x: dimension * 0.5 - triangleWidth * 0.42, y: centerY - triangleHeight / 2))
    triangle.addLine(to: CGPoint(x: dimension * 0.5 - triangleWidth * 0.42, y: centerY + triangleHeight / 2))
    triangle.addLine(to: CGPoint(x: dimension * 0.5 + triangleWidth * 0.58, y: centerY))
    triangle.closeSubpath()
    context.addPath(triangle)
    context.setFillColor(color(1, 1, 1))
    context.fillPath()

    let trackHeight = max(2, dimension * 0.055)
    let track = CGRect(x: dimension * 0.22, y: dimension * 0.26,
                       width: dimension * 0.56, height: trackHeight)
    context.addPath(CGPath(roundedRect: track, cornerWidth: trackHeight / 2,
                           cornerHeight: trackHeight / 2, transform: nil))
    context.setFillColor(color(0.25, 0.25, 0.25))
    context.fillPath()

    let filled = CGRect(x: track.minX, y: track.minY, width: track.width * 0.62, height: track.height)
    context.addPath(CGPath(roundedRect: filled, cornerWidth: trackHeight / 2,
                           cornerHeight: trackHeight / 2, transform: nil))
    context.setFillColor(color(0.113, 0.725, 0.329))
    context.fillPath()

    return context.makeImage()
}

let variants: [(String, Int)] = [
    ("icon_16x16.png", 16), ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32), ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128), ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256), ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512), ("icon_512x512@2x.png", 1024),
]

for (name, size) in variants {
    guard let image = drawIcon(size: size) else {
        FileHandle.standardError.write(Data("falha ao desenhar \(name)\n".utf8))
        exit(1)
    }
    let url = URL(fileURLWithPath: outputDirectory).appendingPathComponent(name)
    guard let destination = CGImageDestinationCreateWithURL(url as CFURL,
                                                           UTType.png.identifier as CFString, 1, nil) else {
        FileHandle.standardError.write(Data("falha ao criar \(name)\n".utf8))
        exit(1)
    }
    CGImageDestinationAddImage(destination, image, nil)
    guard CGImageDestinationFinalize(destination) else {
        FileHandle.standardError.write(Data("falha ao gravar \(name)\n".utf8))
        exit(1)
    }
}
print("icones gerados em \(outputDirectory)")
