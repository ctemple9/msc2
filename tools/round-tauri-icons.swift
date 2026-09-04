#!/usr/bin/env swift

import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let arguments = Array(CommandLine.arguments.dropFirst())
let checkOnly = arguments.first == "--check"
let directory = arguments.drop(while: { $0 == "--check" }).first.map { String($0) }
    ?? "clients/desktop-web/src-tauri/icons"

let fileManager = FileManager.default
let directoryURL = URL(fileURLWithPath: directory, isDirectory: true)
let iconURLs = try fileManager.contentsOfDirectory(
    at: directoryURL,
    includingPropertiesForKeys: nil
).filter { $0.pathExtension.lowercased() == "png" && $0.lastPathComponent.hasPrefix("icon") }

guard !iconURLs.isEmpty else {
    throw NSError(domain: "RoundTauriIcons", code: 1, userInfo: [
        NSLocalizedDescriptionKey: "No Tauri PNG icons found in \(directoryURL.path)"
    ])
}

for iconURL in iconURLs.sorted(by: { $0.lastPathComponent < $1.lastPathComponent }) {
    guard let source = CGImageSourceCreateWithURL(iconURL as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
    else {
        throw NSError(domain: "RoundTauriIcons", code: 2, userInfo: [
            NSLocalizedDescriptionKey: "Could not read \(iconURL.path)"
        ])
    }

    let width = image.width
    let height = image.height
    let bytesPerRow = width * 4
    let colorSpace = CGColorSpaceCreateDeviceRGB()
    guard let context = CGContext(
        data: nil,
        width: width,
        height: height,
        bitsPerComponent: 8,
        bytesPerRow: bytesPerRow,
        space: colorSpace,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else {
        throw NSError(domain: "RoundTauriIcons", code: 3, userInfo: [
            NSLocalizedDescriptionKey: "Could not create an RGBA context for \(iconURL.path)"
        ])
    }

    let bounds = CGRect(x: 0, y: 0, width: width, height: height)
    let radius = min(bounds.width, bounds.height) * 0.18
    let roundedRect = CGPath(
        roundedRect: bounds,
        cornerWidth: radius,
        cornerHeight: radius,
        transform: nil
    )

    context.clear(bounds)
    context.saveGState()
    context.addPath(roundedRect)
    context.clip()
    context.interpolationQuality = CGInterpolationQuality.none
    context.draw(image, in: bounds)
    context.restoreGState()

    guard let roundedImage = context.makeImage() else {
        throw NSError(domain: "RoundTauriIcons", code: 4, userInfo: [
            NSLocalizedDescriptionKey: "Could not render \(iconURL.path)"
        ])
    }

    if checkOnly {
        let cornerSamples = [
            roundedImage.pixelData(atX: 0, y: 0),
            roundedImage.pixelData(atX: width - 1, y: 0),
            roundedImage.pixelData(atX: 0, y: height - 1),
            roundedImage.pixelData(atX: width - 1, y: height - 1),
        ]
        guard cornerSamples.allSatisfy({ $0.alpha < 255 }) else {
            throw NSError(domain: "RoundTauriIcons", code: 5, userInfo: [
                NSLocalizedDescriptionKey: "\(iconURL.lastPathComponent) still has opaque square corners"
            ])
        }
        print("checked \(iconURL.lastPathComponent)")
        continue
    }

    let temporaryURL = iconURL.deletingPathExtension().appendingPathExtension("rounded.png")
    guard let destination = CGImageDestinationCreateWithURL(
        temporaryURL as CFURL,
        UTType.png.identifier as CFString,
        1,
        nil
    ) else {
        throw NSError(domain: "RoundTauriIcons", code: 6, userInfo: [
            NSLocalizedDescriptionKey: "Could not create PNG destination for \(iconURL.path)"
        ])
    }
    CGImageDestinationAddImage(destination, roundedImage, nil)
    guard CGImageDestinationFinalize(destination) else {
        throw NSError(domain: "RoundTauriIcons", code: 7, userInfo: [
            NSLocalizedDescriptionKey: "Could not write \(temporaryURL.path)"
        ])
    }
    _ = try fileManager.replaceItemAt(iconURL, withItemAt: temporaryURL)
    print("rounded \(iconURL.lastPathComponent)")
}

private extension CGImage {
    struct Pixel {
        let alpha: UInt8
    }

    func pixelData(atX x: Int, y: Int) -> Pixel {
        guard let data = dataProvider?.data,
              let bytes = CFDataGetBytePtr(data)
        else {
            return Pixel(alpha: 255)
        }
        let offset = y * bytesPerRow + x * 4
        return Pixel(alpha: bytes[offset + 3])
    }
}
