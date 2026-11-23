#!/usr/bin/env swift
// Captures the running Orbit window into public/screens/ for the tour.
//
//   swift scripts/capture-shot.swift <name> [--light] [--crop X,Y] [--delay S]
//                                         [--owner NAME] [--no-resize] [--out DIR]
//   swift scripts/capture-shot.swift --list
//
// Stage the screen in the app first (theme, page, content), then run this from
// marketing/. Full-window shots match the existing set: the window is resized
// to 1470×922 pt and captured at 2× with its macOS shadow (3164×2068 px), so
// tour.tsx needs no size change. `--light` writes `<name>-light.png`.
//
// Two shots are crops, not windows (no shadow, 2× pixels):
//   sessions-detail  250×575 pt from the window's top-left (the sidebar)
//   agent-detail     600×450 pt at --crop X,Y — screen points read from the
//                    ⌘⇧4 crosshair readout (press Esc after reading them)
//
// Needs Screen Recording for your terminal (or the capture is blank) and
// Accessibility to resize the window (without it, size the window yourself).

import AppKit
import ApplicationServices
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let windowSize = CGSize(width: 1470, height: 922)
let fullShotPixels = (3164, 2068)
let cropPresets: [String: (origin: CGPoint?, size: CGSize)] = [
    "sessions-detail": (CGPoint(x: 0, y: 0), CGSize(width: 250, height: 575)),
    "agent-detail": (nil, CGSize(width: 600, height: 450)),
]

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(("error: " + message + "\n").data(using: .utf8)!)
    exit(1)
}

func warn(_ message: String) {
    FileHandle.standardError.write(("warning: " + message + "\n").data(using: .utf8)!)
}

// MARK: arguments

var name: String?
var light = false
var cropAt: CGPoint?
var delay = 1.0
var owner: String?
var resize = true
var list = false
var outDir = URL(fileURLWithPath: #filePath)
    .deletingLastPathComponent().deletingLastPathComponent()
    .appendingPathComponent("public/screens")

var args = Array(CommandLine.arguments.dropFirst())
while !args.isEmpty {
    let arg = args.removeFirst()
    func value() -> String {
        guard !args.isEmpty else { fail("\(arg) needs a value") }
        return args.removeFirst()
    }
    switch arg {
    case "--light": light = true
    case "--no-resize": resize = false
    case "--list": list = true
    case "--delay":
        guard let seconds = Double(value()), seconds >= 0 else { fail("--delay takes seconds") }
        delay = seconds
    case "--owner": owner = value()
    case "--out": outDir = URL(fileURLWithPath: value())
    case "--crop":
        let parts = value().split(separator: ",").compactMap { Double($0.trimmingCharacters(in: .whitespaces)) }
        guard parts.count == 2 else { fail("--crop takes X,Y in screen points") }
        cropAt = CGPoint(x: parts[0], y: parts[1])
    case "-h", "--help":
        print("usage: swift scripts/capture-shot.swift <name> [--light] [--crop X,Y] [--delay S] [--owner NAME] [--no-resize] [--out DIR] | --list")
        exit(0)
    default:
        if arg.hasPrefix("-") { fail("unknown option \(arg)") }
        guard arg.range(of: "^[a-z0-9-]+$", options: .regularExpression) != nil else {
            fail("name must be lowercase letters, digits and dashes (e.g. new-task)")
        }
        name = arg
    }
}

// MARK: find the window

struct OrbitWindow {
    let id: CGWindowID
    let pid: pid_t
    let owner: String
    let bounds: CGRect
}

func orbitWindows() -> [OrbitWindow] {
    guard let info = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID)
        as? [[String: Any]] else { return [] }
    return info.compactMap { w in
        guard (w[kCGWindowLayer as String] as? Int) == 0,
              let ownerName = w[kCGWindowOwnerName as String] as? String,
              let pid = w[kCGWindowOwnerPID as String] as? pid_t,
              let id = w[kCGWindowNumber as String] as? CGWindowID,
              let dict = w[kCGWindowBounds as String] as? NSDictionary,
              let bounds = CGRect(dictionaryRepresentation: dict),
              bounds.width > 400 else { return nil }
        let matches = owner.map { ownerName == $0 } ?? ownerName.lowercased().contains("orbit")
        return matches ? OrbitWindow(id: id, pid: pid, owner: ownerName, bounds: bounds) : nil
    }
}

if list {
    let found = orbitWindows()
    if found.isEmpty { print("no Orbit window on screen") }
    for w in found {
        print("\(w.owner) (pid \(w.pid)) window \(w.id): \(Int(w.bounds.width))×\(Int(w.bounds.height)) pt at \(Int(w.bounds.minX)),\(Int(w.bounds.minY))")
    }
    exit(0)
}

guard let name else { fail("give a shot name, e.g. new-task (or --list)") }
let preset = cropPresets[name]
if name == "agent-detail" && cropAt == nil {
    fail("agent-detail is a 600×450 pt crop: pass --crop X,Y (screen points from the ⌘⇧4 readout)")
}

// Front to back: the first match is the frontmost Orbit window.
guard let target = orbitWindows().first else {
    fail("no Orbit window on screen (start the app, or pass --owner; --list shows matches)")
}

// MARK: activate + resize

// Inactive windows draw grey traffic lights and a smaller shadow, so the app
// has to be frontmost. A command-line process cannot take focus through
// NSRunningApplication.activate() on macOS 14+ (cooperative activation), so
// Accessibility does it; without that permission, the user clicks the window.
let trusted = AXIsProcessTrusted()
let axApp = AXUIElementCreateApplication(target.pid)
if trusted {
    AXUIElementSetAttributeValue(axApp, kAXFrontmostAttribute as CFString, kCFBooleanTrue)
} else {
    NSRunningApplication(processIdentifier: target.pid)?.activate()
}

if resize && preset == nil {
    if trusted {
        var value: CFTypeRef?
        if AXUIElementCopyAttributeValue(axApp, kAXFocusedWindowAttribute as CFString, &value) == .success,
           let window = value {
            var size = windowSize
            if let axSize = AXValueCreate(.cgSize, &size) {
                AXUIElementSetAttributeValue(window as! AXUIElement, kAXSizeAttribute as CFString, axSize)
            }
        } else {
            warn("could not reach the focused window to resize it")
        }
    } else {
        warn("no Accessibility permission, so the window was not resized (needs \(Int(windowSize.width))×\(Int(windowSize.height)) pt)")
    }
}

// Wait for the window to come to the front (CGWindowList is front to back).
func targetIsFrontmost() -> Bool {
    guard let info = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID)
        as? [[String: Any]] else { return false }
    let front = info.first { ($0[kCGWindowLayer as String] as? Int) == 0 }
    return (front?[kCGWindowNumber as String] as? CGWindowID) == target.id
}
let deadline = Date().addingTimeInterval(3)
while !targetIsFrontmost() && Date() < deadline { Thread.sleep(forTimeInterval: 0.1) }
if !targetIsFrontmost() {
    warn("Orbit is not the frontmost window, so it will look inactive (grey traffic lights, smaller shadow); grant Accessibility or use --delay and click it")
}

Thread.sleep(forTimeInterval: delay)

// Re-read bounds after the resize settled.
let bounds = orbitWindows().first { $0.id == target.id }?.bounds ?? target.bounds

// MARK: capture

func capture(shadow: Bool) -> URL {
    let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("orbit-shot-\(UUID().uuidString).png")
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    process.arguments = ["-l\(target.id)", "-x", "-t", "png"] + (shadow ? [] : ["-o"]) + [tmp.path]
    do { try process.run() } catch { fail("screencapture: \(error)") }
    process.waitUntilExit()
    guard process.terminationStatus == 0, FileManager.default.fileExists(atPath: tmp.path) else {
        fail("screencapture failed (does your terminal have Screen Recording permission?)")
    }
    return tmp
}

func loadImage(_ url: URL) -> CGImage {
    guard let src = CGImageSourceCreateWithURL(url as CFURL, nil),
          let img = CGImageSourceCreateImageAtIndex(src, 0, nil) else { fail("could not read \(url.path)") }
    return img
}

func write(_ image: CGImage, to url: URL) {
    guard let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil) else {
        fail("could not write \(url.path)")
    }
    CGImageDestinationAddImage(dest, image, nil)
    guard CGImageDestinationFinalize(dest) else { fail("could not write \(url.path)") }
}

try? FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)
let out = outDir.appendingPathComponent(name + (light ? "-light" : "") + ".png")

if let preset {
    let tmp = capture(shadow: false)
    defer { try? FileManager.default.removeItem(at: tmp) }
    let full = loadImage(tmp)
    let scale = CGFloat(full.width) / bounds.width
    let origin = cropAt.map { CGPoint(x: $0.x - bounds.minX, y: $0.y - bounds.minY) } ?? preset.origin!
    let rect = CGRect(x: origin.x * scale, y: origin.y * scale,
                      width: preset.size.width * scale, height: preset.size.height * scale).integral
    guard CGRect(x: 0, y: 0, width: full.width, height: full.height).contains(rect),
          let cropped = full.cropping(to: rect) else {
        fail("crop \(Int(origin.x)),\(Int(origin.y)) \(Int(preset.size.width))×\(Int(preset.size.height)) pt falls outside the \(Int(bounds.width))×\(Int(bounds.height)) pt window")
    }
    write(cropped, to: out)
    print("wrote \(out.path) (\(cropped.width)×\(cropped.height))")
} else {
    let tmp = capture(shadow: true)
    defer { try? FileManager.default.removeItem(at: tmp) }
    let image = loadImage(tmp)
    write(image, to: out)
    print("wrote \(out.path) (\(image.width)×\(image.height))")
    if (image.width, image.height) != fullShotPixels {
        if bounds.size == windowSize {
            warn("expected \(fullShotPixels.0)×\(fullShotPixels.1) like the other tour shots; the window size is right, so the shadow differs — the window was probably inactive when captured, retake it")
        } else {
            warn("expected \(fullShotPixels.0)×\(fullShotPixels.1) like the other tour shots; the window was \(Int(bounds.width))×\(Int(bounds.height)) pt — resize it to \(Int(windowSize.width))×\(Int(windowSize.height)) pt and retake")
        }
    }
}
