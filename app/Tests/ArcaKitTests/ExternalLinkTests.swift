// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

import XCTest
@testable import ArcaKit

final class ExternalLinkTests: XCTestCase {
    func testOnlyWebAddressesWithAHostCanBeOpened() throws {
        for href in ["https://example.org/a?b=c", "http://example.org", "HTTPS://Example.org/"] {
            let url = try XCTUnwrap(ExternalLink.url(from: href), href)
            XCTAssertTrue(ExternalLink.isOpenable(url), href)
        }
        for href in [
            "tel:+10000000000", "sms:+10000000000", "mailto:someone@example.org",
            "ms-msdt:/id PCWDiagnostic", "ms-word:ofe|u|https://example.org/x.docx",
            "file:///etc/passwd", "javascript:alert(1)", "arca-custom://open", "https://", "http:///path",
        ] {
            if let url = ExternalLink.url(from: href) {
                XCTAssertFalse(ExternalLink.isOpenable(url), href)
            }
        }
    }

    func testNoLinkIsMadeFromNothingOrFromAnOverlongAddress() {
        XCTAssertNil(ExternalLink.url(from: nil))
        XCTAssertNil(ExternalLink.url(from: ""))
        XCTAssertNil(ExternalLink.url(from: "no scheme at all"))
        XCTAssertNil(ExternalLink.url(from: "https://example.org/" + String(repeating: "a", count: ExternalLink.maxLength)))
    }
}
