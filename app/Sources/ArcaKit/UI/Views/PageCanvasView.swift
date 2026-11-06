// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

import SwiftUI

/// "Pages" mode of the viewer: the vector drawing of one page, produced by the sandboxed
/// renderer (`ARCA_OP_RENDER`), painted on a SwiftUI `Canvas`. Text, paths, clips and pictures
/// are drawn from primitives; nothing from the document is ever executed.
struct PageCanvasView: View {
    @EnvironmentObject private var state: AppState

    @State private var pageIndex = 0
    @State private var drawing: PageDrawing?
    @State private var isLoading = false
    @State private var failure: String?
    /// Decode and show the pictures of the page without a tap.
    @AppStorage("arca.autoPictures") private var autoPictures = true

    /// At most this many pictures are decoded for one page without a tap.
    private let autoPictureLimit = 24

    var body: some View {
        VStack(spacing: 0) {
            pager
            ScrollView([.vertical]) {
                pageBody
                    .padding(12)
            }
        }
        .task(id: pageIndex) { await load() }
    }

    // MARK: - Pager

    private var pageCount: Int { max(1, drawing?.pages ?? 1) }

    private var pager: some View {
        HStack(spacing: 14) {
            Button {
                if pageIndex > 0 { pageIndex -= 1 }
            } label: {
                ArcaGlyph(.arrowLeft, size: 16, color: pageIndex > 0 ? state.theme.accent : state.theme.textSecondary)
            }
            .disabled(pageIndex == 0)

            Text("Страница \(pageIndex + 1) из \(pageCount)")
                .font(.system(size: 13, weight: .semibold))
                .foregroundColor(state.theme.textPrimary)

            Button {
                if pageIndex + 1 < pageCount { pageIndex += 1 }
            } label: {
                ArcaGlyph(.chevronRight, size: 16, color: pageIndex + 1 < pageCount ? state.theme.accent : state.theme.textSecondary)
            }
            .disabled(pageIndex + 1 >= pageCount)

            Spacer()

            Toggle(isOn: $autoPictures) {
                Text("Картинки")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundColor(state.theme.textSecondary)
            }
            .toggleStyle(.switch)
            .fixedSize()
            .onChange(of: autoPictures) { _ in Task { await loadPictures() } }
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 8)
        .background(state.theme.surface)
    }

    // MARK: - Page

    @ViewBuilder
    private var pageBody: some View {
        if let drawing, drawing.width > 0, drawing.height > 0 {
            let pictures = state.decodedPictures
            Canvas { context, size in
                Self.paint(drawing, pictures: pictures, in: &context, size: size)
            }
            .aspectRatio(CGFloat(drawing.width / drawing.height), contentMode: .fit)
            .frame(maxWidth: .infinity)
            .background(Color.white)
            .clipShape(RoundedRectangle(cornerRadius: 4))
            .shadow(color: .black.opacity(0.25), radius: 6, y: 2)
            if drawing.isTruncated || drawing.isApproximate {
                Text(drawing.isTruncated ? "Страница показана не полностью." : "Вёрстка приблизительная.")
                    .font(.system(size: 11))
                    .foregroundColor(state.theme.textSecondary)
                    .padding(.top, 6)
            }
        } else if let failure {
            Text(failure)
                .font(.system(size: 13))
                .foregroundColor(state.theme.textSecondary)
                .frame(maxWidth: .infinity, minHeight: 200)
        } else {
            ProgressView().frame(maxWidth: .infinity, minHeight: 200)
        }
    }

    // MARK: - Loading

    private func load() async {
        guard !isLoading else { return }
        isLoading = true
        defer { isLoading = false }
        do {
            let page = try await state.renderPage(pageIndex)
            drawing = page
            failure = nil
            await loadPictures()
        } catch is CancellationError {
            return
        } catch {
            drawing = nil
            failure = "Эту страницу не удалось отрисовать."
        }
    }

    private func loadPictures() async {
        guard autoPictures, let ids = drawing?.imageIds else { return }
        for id in ids.prefix(autoPictureLimit) {
            if Task.isCancelled { return }
            _ = await state.picture(id: id)
        }
    }

    // MARK: - Painting

    private static func color(_ argb: UInt32) -> Color {
        Color(
            .sRGB,
            red: Double((argb >> 16) & 0xff) / 255,
            green: Double((argb >> 8) & 0xff) / 255,
            blue: Double(argb & 0xff) / 255,
            opacity: Double((argb >> 24) & 0xff) / 255)
    }

    private static func path(from ops: [Double]) -> Path {
        var path = Path()
        var i = 0
        while i < ops.count {
            let code = Int(ops[i])
            if code == 0, i + 2 < ops.count {
                path.move(to: CGPoint(x: ops[i + 1], y: ops[i + 2]))
                i += 3
            } else if code == 1, i + 2 < ops.count {
                path.addLine(to: CGPoint(x: ops[i + 1], y: ops[i + 2]))
                i += 3
            } else if code == 2, i + 6 < ops.count {
                path.addCurve(
                    to: CGPoint(x: ops[i + 5], y: ops[i + 6]),
                    control1: CGPoint(x: ops[i + 1], y: ops[i + 2]),
                    control2: CGPoint(x: ops[i + 3], y: ops[i + 4]))
                i += 7
            } else if code == 3 {
                path.closeSubpath()
                i += 1
            } else {
                break
            }
        }
        return path
    }

    static func paint(
        _ drawing: PageDrawing, pictures: [Int: CGImage], in context: inout GraphicsContext, size: CGSize
    ) {
        let scale = size.width / CGFloat(max(drawing.width, 1))
        var c = context
        c.scaleBy(x: scale, y: scale)
        var saved: [GraphicsContext] = []

        for primitive in drawing.primitives {
            switch primitive {
            case .save:
                saved.append(c)
            case .restore:
                if let previous = saved.popLast() { c = previous }
            case .clip(let x, let y, let w, let h):
                c.clip(to: Path(CGRect(x: x, y: y, width: w, height: h)))
            case .path(let fill, let stroke, let width, let evenOdd, let ops):
                let p = path(from: ops)
                if (fill >> 24) != 0 {
                    c.fill(p, with: .color(color(fill)), style: FillStyle(eoFill: evenOdd))
                }
                if (stroke >> 24) != 0 {
                    // A hairline (width 0) is one device pixel wide.
                    c.stroke(p, with: .color(color(stroke)), lineWidth: width > 0 ? CGFloat(width) : 1 / scale)
                }
            case .text(let x, let y, let fontSize, let angle, let advance, let style, let rgba, let string):
                guard !string.isEmpty, fontSize > 0 else { continue }
                var t = c
                t.translateBy(x: x, y: y)
                t.rotate(by: .radians(angle))
                var font = Font.system(
                    size: CGFloat(fontSize),
                    weight: (style & PageDrawing.bold) != 0 ? .bold : .regular,
                    design: (style & PageDrawing.mono) != 0 ? .monospaced
                        : (style & PageDrawing.serif) != 0 ? .serif : .default)
                if (style & PageDrawing.italic) != 0 { font = font.italic() }
                let resolved = t.resolve(Text(string).font(font).foregroundColor(color(rgba)))
                let natural = resolved.measure(in: CGSize(width: 10_000, height: 10_000)).width
                // The document says how wide the run is: fit the substitute font to it.
                if advance > 0, natural > 0 {
                    t.scaleBy(x: min(2, max(0.5, CGFloat(advance) / natural)), y: 1)
                }
                // (x, y) is the baseline; the descent of the box sits a little below it.
                t.draw(resolved, at: CGPoint(x: 0, y: CGFloat(fontSize) * 0.2), anchor: .bottomLeading)
            case .image(let m, _, let id):
                guard m.count == 6 else { continue }
                var ic = c
                ic.concatenate(CGAffineTransform(a: m[0], b: m[1], c: m[2], d: m[3], tx: m[4], ty: m[5]))
                let unit = CGRect(x: 0, y: 0, width: 1, height: 1)
                if let picture = pictures[id] {
                    ic.draw(Image(decorative: picture, scale: 1), in: unit)
                } else {
                    // Grey box until the sandbox has decoded the picture.
                    ic.fill(Path(unit), with: .color(.gray.opacity(0.25)))
                }
            }
        }
    }
}
