// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

import XCTest

@testable import ArcaKit

final class MacroExplainTests: XCTestCase {
    private func macroJSON(explain: String?) -> Data {
        let extra = explain.map { ",\"explain\":\($0)" } ?? ""
        return Data(
            """
            {"lang":"Vba","loc":"word/vbaProject.bin","name":"Module1","source":"Sub A()\\nEnd Sub",
             "truncated":false,"autoExec":["AutoOpen"],"suspicious":["Shell"],"iocs":[],
             "intent":"Malicious","procs":["A"],"behaviors":[]\(extra)}
            """.utf8)
    }

    func testDecodesTheStaticExplanation() throws {
        let explain = """
            {"verdict":"Hostile",
             "title":{"ru":"Макрос перенаправления на сайт","en":"Redirecting macro"},
             "summary":{"ru":"Способен перенаправить вас","en":"Can redirect you"},
             "steps":[{"ru":"Запускается сам","en":"Starts by itself"},{"ru":"Вывод","en":"Conclusion"}],
             "iocs":[{"kind":"Url","value":"http://evil.example.test/pay"}]}
            """
        let macro = try JSONDecoder().decode(MacroModule.self, from: macroJSON(explain: explain))
        let e = try XCTUnwrap(macro.explain)
        XCTAssertEqual(e.verdict, "Hostile")
        XCTAssertEqual(e.title.en, "Redirecting macro")
        XCTAssertEqual(e.steps.count, 2)
        XCTAssertEqual(e.iocs.first?.value, "http://evil.example.test/pay")
        XCTAssertFalse(e.summary.localized.isEmpty)
    }

    func testAReportWithoutAnExplanationStillDecodes() throws {
        let macro = try JSONDecoder().decode(MacroModule.self, from: macroJSON(explain: nil))
        XCTAssertNil(macro.explain)
        XCTAssertEqual(macro.intent, "Malicious")
    }
}
