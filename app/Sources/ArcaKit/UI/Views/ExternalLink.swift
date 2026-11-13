// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

import Foundation

/// What may be done with a link from a document. The address comes from the document and is not
/// trusted: a tap never opens it directly. The reader sees the full address first and may copy
/// it; only `http` and `https` addresses with a host can be opened, in the browser, never `tel:`,
/// `sms:`, `ms-*:`, `file:` or an app's own scheme (the same rule as the Android app).
enum ExternalLink {
    /// The longest address that is turned into a tappable link (longer ones stay plain text).
    static let maxLength = 4096

    /// The address as a URL the viewer can attach to the text, or `nil` when it cannot be one.
    static func url(from href: String?) -> URL? {
        guard let href, !href.isEmpty, href.count <= maxLength else { return nil }
        let trimmed = href.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let url = URL(string: trimmed), url.scheme != nil else { return nil }
        return url
    }

    /// Whether the address may be opened in the browser (after the reader confirmed it).
    static func isOpenable(_ url: URL) -> Bool {
        guard let scheme = url.scheme?.lowercased(), scheme == "http" || scheme == "https" else {
            return false
        }
        guard let host = url.host, !host.isEmpty else { return false }
        return true
    }
}
