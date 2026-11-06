// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

import Foundation

/// The vector drawing of one page, as produced by the sandboxed renderer (`ARCA_OP_RENDER`).
///
/// The wire format is `{"w","h","index","pages","truncated","approx","p":[...]}` where each
/// primitive is an array that starts with a letter (see `render::page_json` in arca-engine):
/// `["P", fill, stroke, width, evenOdd, [ops]]`, `["T", x, y, size, angle, width, style, color,
/// "text"]`, `["I", a, b, c, d, e, f, "label", "id"]`, `["S"]`, `["R"]`, `["C", x, y, w, h]`.
/// Colours are `0xAARRGGBB`; an alpha of 0 means "none".
public struct PageDrawing: Sendable, Equatable {
    public enum Primitive: Sendable, Equatable {
        /// `ops`: `0 x y` move, `1 x y` line, `2 x1 y1 x2 y2 x y` curve, `3` close.
        case path(fill: UInt32, stroke: UInt32, width: Double, evenOdd: Bool, ops: [Double])
        case text(
            x: Double, y: Double, size: Double, angle: Double, advance: Double,
            style: UInt32, color: UInt32, text: String)
        /// `matrix` is `[a, b, c, d, e, f]`: it maps the unit square onto the page.
        case image(matrix: [Double], label: String, id: Int)
        case save
        case restore
        case clip(x: Double, y: Double, width: Double, height: Double)
    }

    public static let bold: UInt32 = 1
    public static let italic: UInt32 = 2
    public static let mono: UInt32 = 4
    public static let serif: UInt32 = 8

    public let width: Double
    public let height: Double
    public let index: Int
    public let pages: Int
    public let isTruncated: Bool
    public let isApproximate: Bool
    public let primitives: [Primitive]

    public init(
        width: Double, height: Double, index: Int = 0, pages: Int = 1,
        isTruncated: Bool = false, isApproximate: Bool = false, primitives: [Primitive]
    ) {
        self.width = width
        self.height = height
        self.index = index
        self.pages = pages
        self.isTruncated = isTruncated
        self.isApproximate = isApproximate
        self.primitives = primitives
    }

    /// The ids of every picture on the page (`Op::Image` takes them), in drawing order.
    public var imageIds: [Int] {
        var seen = Set<Int>()
        var ids: [Int] = []
        for case .image(_, _, let id) in primitives where id >= 0 && seen.insert(id).inserted {
            ids.append(id)
        }
        return ids
    }

    public enum ParseError: Error, Equatable {
        case notAnObject
        case badPrimitive(Int)
    }

    /// Parses the JSON text returned by `ArcaEngine.renderPage`.
    public static func parse(_ json: String) throws -> PageDrawing {
        try parse(data: Data(json.utf8))
    }

    public static func parse(data: Data) throws -> PageDrawing {
        guard let root = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw ParseError.notAnObject
        }
        let rawPrims = root["p"] as? [Any] ?? []
        var prims: [Primitive] = []
        prims.reserveCapacity(rawPrims.count)
        for (n, raw) in rawPrims.enumerated() {
            guard let a = raw as? [Any], let tag = a.first as? String else {
                throw ParseError.badPrimitive(n)
            }
            func num(_ i: Int) -> Double { (a.count > i ? (a[i] as? NSNumber)?.doubleValue : nil) ?? 0 }
            func uint(_ i: Int) -> UInt32 {
                UInt32(truncatingIfNeeded: (a.count > i ? (a[i] as? NSNumber)?.uint64Value : nil) ?? 0)
            }
            switch tag {
            case "P":
                let ops = (a.count > 5 ? a[5] as? [Any] : nil)?.map { ($0 as? NSNumber)?.doubleValue ?? 0 } ?? []
                prims.append(
                    .path(fill: uint(1), stroke: uint(2), width: num(3), evenOdd: num(4) != 0, ops: ops))
            case "T":
                let text = (a.count > 8 ? a[8] as? String : nil) ?? ""
                prims.append(
                    .text(
                        x: num(1), y: num(2), size: num(3), angle: num(4), advance: num(5),
                        style: uint(6), color: uint(7), text: text))
            case "I":
                let matrix = (1...6).map { num($0) }
                let label = (a.count > 7 ? a[7] as? String : nil) ?? ""
                // The id is a decimal string so that a reader of floats never rounds it.
                let id = Int((a.count > 8 ? a[8] as? String : nil) ?? "") ?? -1
                prims.append(.image(matrix: matrix, label: label, id: id))
            case "S": prims.append(.save)
            case "R": prims.append(.restore)
            case "C": prims.append(.clip(x: num(1), y: num(2), width: num(3), height: num(4)))
            default: throw ParseError.badPrimitive(n)
            }
        }
        return PageDrawing(
            width: (root["w"] as? NSNumber)?.doubleValue ?? 0,
            height: (root["h"] as? NSNumber)?.doubleValue ?? 0,
            index: (root["index"] as? NSNumber)?.intValue ?? 0,
            pages: (root["pages"] as? NSNumber)?.intValue ?? 1,
            isTruncated: (root["truncated"] as? NSNumber)?.boolValue ?? false,
            isApproximate: (root["approx"] as? NSNumber)?.boolValue ?? false,
            primitives: prims)
    }
}
