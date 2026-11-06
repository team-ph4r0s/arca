// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

import XCTest

@testable import ArcaKit

final class PageDrawingTests: XCTestCase {
    private let sample = """
        {"w":595.0,"h":842.0,"index":0,"pages":3,"truncated":false,"approx":true,"p":[
          ["S"],["C",10,10,500,700],
          ["P",4294967295,0,1.5,0,[0,10,10,1,100,10,2,100,50,150,50,150,10,3]],
          ["T",72,100,12,0.0,300,5,4278190080,"Hello, Arca"],
          ["I",100,0,0,50,20,200,"photo","12345678901234"],
          ["R"]
        ]}
        """

    func testParsesEveryKindOfPrimitive() throws {
        let page = try PageDrawing.parse(sample)
        XCTAssertEqual(page.width, 595)
        XCTAssertEqual(page.height, 842)
        XCTAssertEqual(page.pages, 3)
        XCTAssertTrue(page.isApproximate)
        XCTAssertFalse(page.isTruncated)
        XCTAssertEqual(page.primitives.count, 6)

        guard case .path(let fill, let stroke, let width, let evenOdd, let ops) = page.primitives[2] else {
            return XCTFail("third primitive is a path")
        }
        XCTAssertEqual(fill, 0xFFFF_FFFF)
        XCTAssertEqual(stroke, 0)
        XCTAssertEqual(width, 1.5)
        XCTAssertFalse(evenOdd)
        XCTAssertEqual(ops.count, 14)

        guard case .text(let x, let y, let size, _, let advance, let style, let color, let text) = page.primitives[3]
        else { return XCTFail("fourth primitive is text") }
        XCTAssertEqual([x, y, size, advance], [72, 100, 12, 300])
        XCTAssertEqual(style, PageDrawing.bold | PageDrawing.mono)
        XCTAssertEqual(color, 0xFF00_0000)
        XCTAssertEqual(text, "Hello, Arca")
    }

    func testImageIdsAreDecimalStringsAndListedOnce() throws {
        let page = try PageDrawing.parse(sample)
        XCTAssertEqual(page.imageIds, [12_345_678_901_234])
        guard case .image(let matrix, let label, let id) = page.primitives[4] else {
            return XCTFail("fifth primitive is a picture")
        }
        XCTAssertEqual(matrix, [100, 0, 0, 50, 20, 200])
        XCTAssertEqual(label, "photo")
        XCTAssertEqual(id, 12_345_678_901_234)
    }

    func testRejectsAnUnknownPrimitive() {
        XCTAssertThrowsError(try PageDrawing.parse(#"{"w":1,"h":1,"p":[["Z"]]}"#)) { error in
            XCTAssertEqual(error as? PageDrawing.ParseError, .badPrimitive(0))
        }
        XCTAssertThrowsError(try PageDrawing.parse("[]")) { error in
            XCTAssertEqual(error as? PageDrawing.ParseError, .notAnObject)
        }
    }

    func testDiagnosticsKeepRealFiguresWhenBuiltDirectly() {
        let d = RunDiagnostics(fuel: 1_234, fuelLimit: 9_999, peakMemory: 4 << 20, wallMs: 17)
        XCTAssertEqual([d.fuel, d.fuelLimit, d.peakMemory, d.wallMs], [1_234, 9_999, 4 << 20, 17])
    }
}
