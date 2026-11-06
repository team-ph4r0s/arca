// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

import Foundation

/// Run style bitflags defined by `arca_preview::model::style`.
public struct RunStyle: OptionSet, Sendable, Hashable {
    public let rawValue: Int

    public init(rawValue: Int) {
        self.rawValue = rawValue
    }

    public static let bold = RunStyle(rawValue: 1)
    public static let italic = RunStyle(rawValue: 2)
    public static let underline = RunStyle(rawValue: 4)
    public static let strike = RunStyle(rawValue: 8)
    public static let mono = RunStyle(rawValue: 16)
    public static let `super` = RunStyle(rawValue: 32)
    public static let sub = RunStyle(rawValue: 64)
    public static let hidden = RunStyle(rawValue: 128)
    public static let deleted = RunStyle(rawValue: 256)
    public static let inserted = RunStyle(rawValue: 512)
}

/// A slice of styled text within a paragraph.
public struct TextRun: Sendable, Hashable, Identifiable {
    public var id: String { "\(text):\(style.rawValue):\(href ?? "")" }

    public let text: String
    public let style: RunStyle
    /// Optional packed RGB colours emitted by the engine (0xRRGGBB).
    public let foregroundColor: UInt32?
    public let backgroundColor: UInt32?
    /// Font scale in percent (100 is the document's normal size).
    public let fontScale: UInt32?
    /// Display-only link destination (never automatically navigated).
    public let href: String?

    public init(
        text: String,
        style: RunStyle = [],
        href: String? = nil,
        foregroundColor: UInt32? = nil,
        backgroundColor: UInt32? = nil,
        fontScale: UInt32? = nil
    ) {
        self.text = text
        self.style = style
        self.href = href
        self.foregroundColor = foregroundColor
        self.backgroundColor = backgroundColor
        self.fontScale = fontScale
    }
}

/// A paragraph of text with formatting and alignment.
public struct Paragraph: Sendable, Hashable {
    /// Named style (e.g. "Body", "Heading1", "Heading2", "Title").
    public let style: String
    /// Text alignment: 0=start, 1=center, 2=end, 3=justify.
    public let align: Int
    /// Indentation or list level (0-based).
    public let level: Int
    /// List bullet marker or numbering (e.g. "1.", "•").
    public let marker: String
    /// Formatted runs.
    public let runs: [TextRun]

    public init(style: String = "Body", align: Int = 0, level: Int = 0, marker: String = "", runs: [TextRun] = []) {
        self.style = style
        self.align = align
        self.level = level
        self.marker = marker
        self.runs = runs
    }

    /// Concatenated plain text content.
    public var plainText: String {
        runs.map(\.text).joined()
    }
}

/// Table cell holding one or more paragraphs.
public struct TableCell: Sendable, Hashable {
    /// Column span (defaults to 1).
    public let span: Int
    /// Whether this cell is a table header.
    public let isHeader: Bool
    /// Paragraphs inside the cell.
    public let paragraphs: [Paragraph]
    /// Fill colour of the cell as `0xAARRGGBB`; an alpha of 0 means none.
    public let fill: UInt32
    /// Vertical merge: 0 none, 1 the first cell of a merged column, 2 a cell covered by the one above.
    public let verticalMerge: Int

    public init(
        span: Int = 1, isHeader: Bool = false, paragraphs: [Paragraph] = [],
        fill: UInt32 = 0, verticalMerge: Int = 0
    ) {
        self.span = span
        self.isHeader = isHeader
        self.paragraphs = paragraphs
        self.fill = fill
        self.verticalMerge = verticalMerge
    }
}

/// One row of a document table.
public struct TableRow: Sendable, Hashable {
    public let columns: Int
    public let isFirst: Bool
    public let isLast: Bool
    public let cells: [TableCell]

    public init(columns: Int = 1, isFirst: Bool = false, isLast: Bool = false, cells: [TableCell] = []) {
        self.columns = columns
        self.isFirst = isFirst
        self.isLast = isLast
        self.cells = cells
    }
}

/// Header descriptor for a spreadsheet grid.
public struct GridHeader: Sendable, Hashable {
    public let cols: Int
    public let widths: [Int]
    public let totalRows: Int64
    public let isTruncated: Bool

    public init(cols: Int, widths: [Int], totalRows: Int64, isTruncated: Bool) {
        self.cols = cols
        self.widths = widths
        self.totalRows = totalRows
        self.isTruncated = isTruncated
    }
}

/// One individual cell inside a spreadsheet grid row.
public struct GridCell: Sendable, Hashable {
    public let column: Int
    /// Cell data kind (0=text, 1=number, 2=date, 3=bool, 4=error, 5=formula, 6=calculated).
    public let kind: Int
    public let text: String
    public let href: String?

    public init(column: Int, kind: Int, text: String, href: String? = nil) {
        self.column = column
        self.kind = kind
        self.text = text
        self.href = href
    }
}

/// One row in a spreadsheet grid.
public struct GridRow: Sendable, Hashable {
    public let index: Int64
    public let isHidden: Bool
    public let cells: [GridCell]

    public init(index: Int64, isHidden: Bool, cells: [GridCell]) {
        self.index = index
        self.isHidden = isHidden
        self.cells = cells
    }
}

/// Safe placeholder for an embedded image (decoded only on demand inside the sandbox).
public struct ImagePlaceholder: Sendable, Hashable {
    public let alt: String
    public let media: String
    public let byteSize: Int64
    public let isExternal: Bool
    /// Engine identifier to fetch RGBA raster via `ARCA_OP_IMAGE`.
    public let imageId: Int

    public init(alt: String, media: String, byteSize: Int64, isExternal: Bool, imageId: Int) {
        self.alt = alt
        self.media = media
        self.byteSize = byteSize
        self.isExternal = isExternal
        self.imageId = imageId
    }
}

/// Polymorphic renderable item returned in paginated chunks.
public enum PreviewItem: Sendable, Hashable {
    case paragraph(Paragraph)
    case tableRow(TableRow)
    case gridHeader(GridHeader)
    case gridRow(GridRow)
    case image(ImagePlaceholder)
    case horizontalRule
}

// MARK: - JSON Decoding for Preview Items

extension PreviewItem {
    public enum DecodingError: Swift.Error, LocalizedError {
        case missingKind
        case unsupportedKind(String)
        case invalidField(kind: String, field: String)

        public var errorDescription: String? {
            switch self {
            case .missingKind: return "Preview item is missing its kind (k)"
            case .unsupportedKind(let kind): return "Unsupported preview item kind: \(kind)"
            case .invalidField(let kind, let field):
                return "Invalid field \(field) in preview item \(kind)"
            }
        }
    }

    static func parse(from jsonObject: [String: Any]) throws -> PreviewItem {
        guard let kind = jsonObject["k"] as? String else { throw DecodingError.missingKind }
        switch kind {
        case "p":
            return .paragraph(try parseParagraph(jsonObject, kind: kind))
        case "tr":
            let n = max(1, try intValue(jsonObject["n"], kind: kind, field: "n") ?? 1)
            let first = try boolValue(jsonObject["first"], kind: kind, field: "first") ?? false
            let last = try boolValue(jsonObject["last"], kind: kind, field: "last") ?? false
            var cells: [TableCell] = []
            guard let cellsArray = jsonObject["cells"] as? [Any] else {
                if jsonObject["cells"] != nil { throw DecodingError.invalidField(kind: kind, field: "cells") }
                return .tableRow(TableRow(columns: n, isFirst: first, isLast: last, cells: []))
            }
            for rawCell in cellsArray {
                guard let c = rawCell as? [String: Any] else {
                    throw DecodingError.invalidField(kind: kind, field: "cells")
                }
                let span = max(1, try intValue(c["c"], kind: "tr.cell", field: "c") ?? 1)
                let h = try boolValue(c["h"], kind: "tr.cell", field: "h") ?? false
                var paras: [Paragraph] = []
                if let rawParagraphs = c["p"] {
                    guard let paragraphArray = rawParagraphs as? [Any] else {
                        throw DecodingError.invalidField(kind: "tr.cell", field: "p")
                    }
                    for rawParagraph in paragraphArray {
                        guard let paragraph = rawParagraph as? [String: Any] else {
                            throw DecodingError.invalidField(kind: "tr.cell", field: "p")
                        }
                        paras.append(try parseParagraph(paragraph, kind: "tr.cell"))
                    }
                }
                // Fill and vertical merge are written only when present (an older engine omits them).
                let fill = UInt32(truncatingIfNeeded: (c["f"] as? NSNumber)?.uint64Value ?? 0)
                let vmerge = (c["v"] as? NSNumber)?.intValue ?? 0
                cells.append(
                    TableCell(span: span, isHeader: h, paragraphs: paras, fill: fill, verticalMerge: vmerge))
            }
            return .tableRow(TableRow(columns: n, isFirst: first, isLast: last, cells: cells))
        case "gh":
            let cols = try intValue(jsonObject["cols"], kind: kind, field: "cols") ?? 0
            let widths = try intArray(jsonObject["w"], kind: kind, field: "w")
            let rows = try int64Value(jsonObject["rows"], kind: kind, field: "rows") ?? 0
            let trunc = try boolValue(jsonObject["trunc"], kind: kind, field: "trunc") ?? false
            return .gridHeader(GridHeader(cols: cols, widths: widths, totalRows: rows, isTruncated: trunc))
        case "gr":
            let idx = try int64Value(jsonObject["i"], kind: kind, field: "i") ?? 0
            let hidden = try boolValue(jsonObject["h"], kind: kind, field: "h") ?? false
            var cells: [GridCell] = []
            if let rawCells = jsonObject["c"] {
                guard let cArr = rawCells as? [Any] else {
                    throw DecodingError.invalidField(kind: kind, field: "c")
                }
                for raw in cArr {
                    guard let rawCell = raw as? [Any], rawCell.count >= 3,
                          let col = rawCell[0] as? NSNumber,
                          let cKind = rawCell[1] as? NSNumber,
                          let txt = rawCell[2] as? String else {
                        throw DecodingError.invalidField(kind: kind, field: "c")
                    }
                    let href = rawCell.count > 3 ? (rawCell[3] as? String) : nil
                    cells.append(GridCell(column: col.intValue, kind: cKind.intValue, text: txt, href: href))
                }
            }
            return .gridRow(GridRow(index: idx, isHidden: hidden, cells: cells))
        case "i":
            let alt = try stringValue(jsonObject["alt"], kind: kind, field: "alt") ?? ""
            let media = try stringValue(jsonObject["media"], kind: kind, field: "media") ?? ""
            let bytes = try int64Value(jsonObject["bytes"], kind: kind, field: "bytes") ?? 0
            let ext = try boolValue(jsonObject["ext"], kind: kind, field: "ext") ?? false
            let imgId = try intValue(jsonObject["id"], kind: kind, field: "id") ?? -1
            return .image(ImagePlaceholder(alt: alt, media: media, byteSize: bytes, isExternal: ext, imageId: imgId))
        case "r":
            return .horizontalRule
        default:
            throw DecodingError.unsupportedKind(kind)
        }
    }

    private static func parseParagraph(_ m: [String: Any], kind: String) throws -> Paragraph {
        let style = try stringValue(m["s"], kind: kind, field: "s") ?? "Body"
        let align = try intValue(m["a"], kind: kind, field: "a") ?? 0
        let level = try intValue(m["l"], kind: kind, field: "l") ?? 0
        let marker = try stringValue(m["m"], kind: kind, field: "m") ?? ""
        var runs: [TextRun] = []
        guard let rawRuns = m["r"] as? [Any] else {
            throw DecodingError.invalidField(kind: kind, field: "r")
        }
        for raw in rawRuns {
            guard let values = raw as? [Any], let text = values.first as? String else {
                throw DecodingError.invalidField(kind: kind, field: "r")
            }
            let styleBits = values.count > 1 ? try requiredInt(values[1], kind: kind, field: "r.style") : 0
            let href = values.count > 2 ? (values[2] as? String) : nil
            let foreground = values.count > 3 ? try requiredUInt(values[3], kind: kind, field: "r.fg") : nil
            let background = values.count > 4 ? try requiredUInt(values[4], kind: kind, field: "r.bg") : nil
            let scale = values.count > 5 ? try requiredUInt(values[5], kind: kind, field: "r.scale") : nil
            runs.append(TextRun(text: text, style: RunStyle(rawValue: styleBits), href: href,
                                foregroundColor: foreground, backgroundColor: background, fontScale: scale))
        }
        return Paragraph(style: style, align: align, level: level, marker: marker, runs: runs)
    }

    private static func stringValue(_ value: Any?, kind: String, field: String) throws -> String? {
        guard let value else { return nil }
        guard let string = value as? String else { throw DecodingError.invalidField(kind: kind, field: field) }
        return string
    }

    private static func boolValue(_ value: Any?, kind: String, field: String) throws -> Bool? {
        guard let value else { return nil }
        guard let bool = value as? Bool else { throw DecodingError.invalidField(kind: kind, field: field) }
        return bool
    }

    private static func intValue(_ value: Any?, kind: String, field: String) throws -> Int? {
        guard let value else { return nil }
        guard let number = value as? NSNumber else { throw DecodingError.invalidField(kind: kind, field: field) }
        return number.intValue
    }

    private static func int64Value(_ value: Any?, kind: String, field: String) throws -> Int64? {
        guard let value else { return nil }
        guard let number = value as? NSNumber else { throw DecodingError.invalidField(kind: kind, field: field) }
        return number.int64Value
    }

    private static func requiredInt(_ value: Any, kind: String, field: String) throws -> Int {
        guard let result = try intValue(value, kind: kind, field: field) else {
            throw DecodingError.invalidField(kind: kind, field: field)
        }
        return result
    }

    private static func requiredUInt(_ value: Any, kind: String, field: String) throws -> UInt32 {
        let number = try requiredInt(value, kind: kind, field: field)
        guard number >= 0 else { throw DecodingError.invalidField(kind: kind, field: field) }
        return UInt32(clamping: number)
    }

    private static func intArray(_ value: Any?, kind: String, field: String) throws -> [Int] {
        guard let value else { return [] }
        guard let values = value as? [Any] else { throw DecodingError.invalidField(kind: kind, field: field) }
        return try values.map { try requiredInt($0, kind: kind, field: field) }
    }
}

// MARK: - Preview Metadata

/// Description of one section (sheet, slide, page, or body).
public struct SectionInfo: Codable, Sendable, Equatable, Identifiable {
    public var id: String { "\(kind):\(name)" }

    /// Section kind (e.g. "Body", "Sheet", "Slide", "Page").
    public let kind: String
    /// User-assigned name (e.g. sheet tab title "Q3 Forecast").
    public let name: String
    /// Whether the section was marked hidden in the document.
    public let isHidden: Bool
    /// Total item count in this section.
    public let totalItems: Int64

    enum CodingKeys: String, CodingKey {
        case kind
        case name
        case isHidden = "hidden"
        case totalItems = "items"
    }
}

/// Notice explaining content that is deliberately simplified or omitted.
public struct Notice: Codable, Sendable, Equatable, Identifiable {
    public var id: String { "\(kind):\(detail)" }

    public let kind: String
    public let count: Int64
    public let detail: String
}

/// Top-level metadata for preview navigation.
public struct PreviewMeta: Decodable, Sendable, Equatable {
    public let format: String
    public let family: String
    public let sections: [SectionInfo]
    public let notices: [Notice]
    public let totalChars: Int64
    public let totalItems: Int64
    public let isTruncated: Bool

    private enum CodingKeys: String, CodingKey {
        case format, family, sections, notices, stats
    }

    private struct Stats: Codable {
        let chars: Int64
        let items: Int64
        let truncated: Bool
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        format = try c.decodeIfPresent(String.self, forKey: .format) ?? "Unknown"
        family = try c.decodeIfPresent(String.self, forKey: .family) ?? "None"
        sections = try c.decodeIfPresent([SectionInfo].self, forKey: .sections) ?? []
        notices = try c.decodeIfPresent([Notice].self, forKey: .notices) ?? []

        let st = try c.decodeIfPresent(Stats.self, forKey: .stats)
        totalChars = st?.chars ?? 0
        totalItems = st?.items ?? 0
        isTruncated = st?.truncated ?? false
    }
}
