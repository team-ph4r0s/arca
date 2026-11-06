// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

import SwiftUI
import Combine

/// High-level navigation state of the application.
public enum AppScreen: Sendable, Equatable {
    case idle
    case scanning(fileName: String)
    case report(report: Report, diagnostics: RunDiagnostics)
    case viewer(report: Report, meta: PreviewMeta, diagnostics: RunDiagnostics)
    case about
}

/// Central state manager driving the entire iOS user interface.
@MainActor
public final class AppState: ObservableObject {
    @Published public var currentScreen: AppScreen = .idle
    @Published public var theme: ArcaTheme = .obsidian
    @Published public var isPasswordPromptVisible: Bool = false
    @Published public var passwordInput: String = ""
    @Published public var isSanitizeInProgress: Bool = false
    @Published public var sanitizedDocument: SanitizedDocument?
    @Published public var inspectedMacro: MacroModule?
    @Published public var inspectedImage: DecodedImage?
    @Published public var activeError: ArcaError?
    /// What the sandbox spent on the most recent run (scan, preview or page render): the figures
    /// the viewer's metrics bar shows. Always set on the main actor, after the engine answered.
    @Published public private(set) var lastRun: SandboxRun?

    public let engine: ArcaEngine
    public let patronStore: PatronStore

    /// One sandbox run and its resource use.
    public struct SandboxRun: Sendable, Equatable {
        public enum Kind: String, Sendable { case scan, preview, render }
        public let kind: Kind
        public let diagnostics: RunDiagnostics
    }

    private var activeDocument: DocumentSession?
    private var previewSession: PreviewSession?
    private var pictureCache: [Int: CGImage] = [:]
    private var activeDocumentName: String = ""
    private var pendingDocumentData: Data?

    public init(engine: ArcaEngine = .shared, patronStore: PatronStore = PatronStore()) {
        self.engine = engine
        self.patronStore = patronStore
    }

    // MARK: - Document Intake

    /// Opens and scans a document from a file URL.
    public func openDocument(from url: URL) {
        let name = url.lastPathComponent
        currentScreen = .scanning(fileName: name)
        activeDocumentName = name

        Task {
            do {
                let session = try await engine.openDocument(at: url)
                self.activeDocument = session
                try await startScan(session: session)
            } catch let err as ArcaError {
                handleError(err)
            } catch {
                handleError(.fault)
            }
        }
    }

    /// Opens and scans a document from in-memory Data.
    public func openDocument(data: Data, name: String) {
        currentScreen = .scanning(fileName: name)
        activeDocumentName = name
        pendingDocumentData = data

        Task {
            do {
                let session = try await engine.openDocument(data: data)
                self.activeDocument = session
                try await startScan(session: session)
            } catch let err as ArcaError {
                handleError(err)
            } catch {
                handleError(.fault)
            }
        }
    }

    // MARK: - Scanning & Unlocking

    private func startScan(session: DocumentSession, password: String? = nil) async throws {
        do {
            let result = try await engine.scan(document: session, password: password)
            ArcaHaptics.shieldVerdict(result.report.verdict)

            // Update patron passport statistics
            await patronStore.recordDocumentOpened(
                trackers: Int(result.report.trackers),
                active: Int(result.report.active)
            )

            lastRun = SandboxRun(kind: .scan, diagnostics: result.diagnostics)
            currentScreen = .report(report: result.report, diagnostics: result.diagnostics)
        } catch ArcaError.encrypted, ArcaError.wrongPassword {
            isPasswordPromptVisible = true
        }
    }

    /// Submits a password for an encrypted document.
    public func submitPassword() {
        guard let session = activeDocument, !passwordInput.isEmpty else { return }
        let pw = passwordInput
        passwordInput = ""
        isPasswordPromptVisible = false

        currentScreen = .scanning(fileName: activeDocumentName)
        Task {
            do {
                try await startScan(session: session, password: pw)
            } catch let err as ArcaError {
                handleError(err)
            } catch {
                handleError(.fault)
            }
        }
    }

    // MARK: - Viewer Navigation

    /// Loads preview items for the document currently owned by the app state.
    public func previewItems(section: UInt32, from: UInt64, count: UInt32) async throws -> [PreviewItem] {
        // Slices come from the one preview run made when the viewer was opened.
        if let preview = previewSession {
            return try preview.items(section: section, from: from, count: count)
        }
        guard let session = activeDocument else { throw ArcaError.badHandle }
        return try await engine.previewItems(document: session, section: section, from: from, count: count)
    }

    /// Renders one page (0-based) of the active document into a vector drawing.
    ///
    /// The metrics of the render run are published on the main actor once the engine answered.
    public func renderPage(_ pageNumber: Int) async throws -> PageDrawing {
        guard let session = activeDocument else { throw ArcaError.badHandle }
        let result = try await engine.renderPageWithDiagnostics(
            document: session, pageNumber: UInt32(max(0, pageNumber)))
        lastRun = SandboxRun(kind: .render, diagnostics: result.diagnostics)
        return try PageDrawing.parse(result.json)
    }

    /// A picture of the active document decoded in the sandbox, or `nil` if it cannot be shown.
    public func picture(id: Int) async -> CGImage? {
        if let cached = pictureCache[id] { return cached }
        guard let session = activeDocument,
            let decoded = try? await engine.decodeImage(document: session, imageId: id),
            let image = Self.makeCGImage(from: decoded)
        else { return nil }
        pictureCache[id] = image
        objectWillChange.send()
        return image
    }

    /// The pictures decoded so far (the page canvas redraws when this grows).
    public var decodedPictures: [Int: CGImage] { pictureCache }

    nonisolated static func makeCGImage(from img: DecodedImage) -> CGImage? {
        let width = img.facts.width
        let height = img.facts.height
        guard width > 0, height > 0, img.rgbaPixels.count == width * height * 4,
            let provider = CGDataProvider(data: img.rgbaPixels as CFData)
        else { return nil }
        return CGImage(
            width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32,
            bytesPerRow: width * 4, space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue),
            provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent)
    }

    /// Transitions to the safe read-only preview viewer.
    public func proceedToViewer(report: Report, diagnostics: RunDiagnostics) {
        guard let session = activeDocument else { return }
        Task {
            do {
                let preview = try await engine.openPreview(document: session)
                previewSession = preview
                lastRun = SandboxRun(kind: .preview, diagnostics: preview.diagnostics)
                let meta = preview.meta
                currentScreen = .viewer(report: report, meta: meta, diagnostics: diagnostics)
                ArcaHaptics.selection()
            } catch let err as ArcaError {
                handleError(err)
            } catch {
                handleError(.fault)
            }
        }
    }

    // MARK: - One-Tap Sanitize

    /// Triggers one-tap sterilization of the currently active document.
    public func sanitizeCurrentDocument() {
        guard let session = activeDocument else { return }
        isSanitizeInProgress = true

        Task {
            do {
                let cleanDoc = try await engine.sanitize(document: session)
                self.sanitizedDocument = cleanDoc
                self.isSanitizeInProgress = false
                ArcaHaptics.sanitizeSuccess()
            } catch let err as ArcaError {
                self.isSanitizeInProgress = false
                handleError(err)
            } catch {
                self.isSanitizeInProgress = false
                handleError(.fault)
            }
        }
    }

    // MARK: - Image Inspection

    /// Fetches and opens an embedded sandboxed image raster in full screen.
    public func inspectImage(imageId: Int) {
        guard let session = activeDocument else { return }
        Task {
            do {
                let decoded = try await engine.decodeImage(document: session, imageId: imageId)
                self.inspectedImage = decoded
                ArcaHaptics.selection()
            } catch let err as ArcaError {
                handleError(err)
            } catch {
                handleError(.fault)
            }
        }
    }

    // MARK: - Close / Reset

    /// Closes the current document, wipes session memory, and returns to the home screen.
    public func closeDocument() {
        previewSession = nil
        pictureCache = [:]
        lastRun = nil
        activeDocument?.close()
        activeDocument = nil
        activeDocumentName = ""
        pendingDocumentData = nil
        sanitizedDocument = nil
        inspectedImage = nil
        inspectedMacro = nil
        currentScreen = .idle
        ArcaHaptics.selection()
    }

    private func handleError(_ error: ArcaError) {
        self.activeError = error
        if case .scanning = currentScreen {
            currentScreen = .idle
        }
    }
}
