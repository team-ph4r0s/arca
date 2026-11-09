//! Preset shapes as paths, in the shape's own coordinates (`0..w`, `0..h`).
//!
//! A small table of the presets slides use all the time (ISO 29500 `presetShapeDefinitions` for
//! the ones listed here, with their default adjust values). Anything else is not drawn from this
//! table: the caller outlines the frame and says the slide is approximate.

use super::Seg;

/// Bézier constant for a quarter of a circle.
const KAPPA: f64 = 0.552_284_749_830_793_4;

/// A preset shape.
pub(super) struct Preset {
    /// The outline.
    pub(super) segs: Vec<Seg>,
    /// A line: it is stroked, never filled.
    pub(super) open: bool,
    /// The preset is only a rough likeness of the named shape.
    pub(super) rough: bool,
}

fn closed(segs: Vec<Seg>) -> Preset {
    Preset { segs, open: false, rough: false }
}

fn rough(segs: Vec<Seg>) -> Preset {
    Preset { segs, open: false, rough: true }
}

/// A rectangle.
pub(super) fn rect(w: f64, h: f64) -> Vec<Seg> {
    poly(&[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)], true)
}

/// A polygon or polyline through `pts`.
pub(super) fn poly(pts: &[(f64, f64)], close: bool) -> Vec<Seg> {
    let mut out = Vec::with_capacity(pts.len() + 1);
    for (i, (x, y)) in pts.iter().enumerate() {
        out.push(if i == 0 { Seg::M(*x, *y) } else { Seg::L(*x, *y) });
    }
    if close && !pts.is_empty() {
        out.push(Seg::Z);
    }
    out
}

/// An ellipse inscribed in `0..w`, `0..h`.
pub(super) fn ellipse(w: f64, h: f64) -> Vec<Seg> {
    let (rx, ry) = (w / 2.0, h / 2.0);
    let (cx, cy) = (rx, ry);
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    vec![
        Seg::M(cx + rx, cy),
        Seg::C(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry),
        Seg::C(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy),
        Seg::C(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry),
        Seg::C(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy),
        Seg::Z,
    ]
}

/// A rectangle with rounded corners of radius `r`.
pub(super) fn round_rect(w: f64, h: f64, r: f64) -> Vec<Seg> {
    let r = r.clamp(0.0, w.min(h) / 2.0);
    if r <= 0.0 {
        return rect(w, h);
    }
    let k = r * KAPPA;
    vec![
        Seg::M(r, 0.0),
        Seg::L(w - r, 0.0),
        Seg::C(w - r + k, 0.0, w, r - k, w, r),
        Seg::L(w, h - r),
        Seg::C(w, h - r + k, w - r + k, h, w - r, h),
        Seg::L(r, h),
        Seg::C(r - k, h, 0.0, h - r + k, 0.0, h - r),
        Seg::L(0.0, r),
        Seg::C(0.0, r - k, r - k, 0.0, r, 0.0),
        Seg::Z,
    ]
}

/// Scales the points so that their bounding box is exactly `0..w`, `0..h`.
fn fit(pts: &[(f64, f64)], w: f64, h: f64) -> Vec<(f64, f64)> {
    let x0 = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let x1 = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let y0 = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let y1 = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let (dx, dy) = ((x1 - x0).max(1e-9), (y1 - y0).max(1e-9));
    pts.iter().map(|(x, y)| ((x - x0) / dx * w, (y - y0) / dy * h)).collect()
}

/// An arrow of length `len` along its axis and thickness `thick` across it, pointing along +x:
/// the points as `(along, across)`.
fn arrow_points(len: f64, thick: f64, shaft: f64, head: f64) -> Vec<(f64, f64)> {
    let y1 = thick * (1.0 - shaft) / 2.0;
    let x1 = (len - head).max(0.0);
    vec![
        (0.0, y1),
        (x1, y1),
        (x1, 0.0),
        (len, thick / 2.0),
        (x1, thick),
        (x1, thick - y1),
        (0.0, thick - y1),
    ]
}

/// The preset `name` for a frame of `w × h`; `adj` are the first two adjust values (in 1/100000).
/// `None` for a name this table does not know.
#[allow(clippy::too_many_lines)] // one arm per preset
pub(super) fn preset(name: &str, w: f64, h: f64, adj: [Option<f64>; 2]) -> Option<Preset> {
    let a = |i: usize, default: f64, max: f64| -> f64 {
        let v = adj.get(i).copied().flatten().unwrap_or(default);
        (if v.is_finite() { v } else { default } / 100_000.0).clamp(0.0, max)
    };
    let ss = w.min(h);
    Some(match name {
        "rect"
        | "flowChartProcess"
        | "flowChartPredefinedProcess"
        | "flowChartInternalStorage"
        | "textBox" => closed(rect(w, h)),
        "roundRect" | "flowChartAlternateProcess" => {
            closed(round_rect(w, h, ss * a(0, 16_667.0, 0.5)))
        }
        "flowChartTerminator" => closed(round_rect(w, h, ss / 2.0)),
        "ellipse" | "flowChartConnector" | "flowChartSummingJunction" | "flowChartOr" => {
            closed(ellipse(w, h))
        }
        "triangle" => closed(poly(&[(0.0, h), (w * a(0, 50_000.0, 1.0), 0.0), (w, h)], true)),
        "rtTriangle" => closed(poly(&[(0.0, 0.0), (0.0, h), (w, h)], true)),
        "diamond" | "flowChartDecision" => {
            closed(poly(&[(w / 2.0, 0.0), (w, h / 2.0), (w / 2.0, h), (0.0, h / 2.0)], true))
        }
        "parallelogram" | "flowChartInputOutput" => {
            let x = ss * a(0, 25_000.0, if ss > 0.0 { w / ss } else { 1.0 });
            closed(poly(&[(0.0, h), (x, 0.0), (w, 0.0), (w - x, h)], true))
        }
        "trapezoid" | "flowChartManualOperation" => {
            let x = ss * a(0, 25_000.0, 0.5 * if ss > 0.0 { w / ss } else { 1.0 });
            closed(poly(&[(0.0, h), (x, 0.0), (w - x, 0.0), (w, h)], true))
        }
        "hexagon" | "flowChartPreparation" => {
            let x = ss * a(0, 25_000.0, 0.5 * if ss > 0.0 { w / ss } else { 1.0 });
            closed(poly(
                &[(0.0, h / 2.0), (x, 0.0), (w - x, 0.0), (w, h / 2.0), (w - x, h), (x, h)],
                true,
            ))
        }
        "octagon" => {
            let x = ss * a(0, 29_289.0, 0.5);
            closed(poly(
                &[
                    (0.0, x),
                    (x, 0.0),
                    (w - x, 0.0),
                    (w, x),
                    (w, h - x),
                    (w - x, h),
                    (x, h),
                    (0.0, h - x),
                ],
                true,
            ))
        }
        "pentagon" => {
            let pts: Vec<(f64, f64)> = (0..5)
                .map(|i| {
                    let t = (f64::from(i) * 72.0 - 90.0).to_radians();
                    (t.cos(), t.sin())
                })
                .collect();
            closed(poly(&fit(&pts, w, h), true))
        }
        "star5" => {
            let pts: Vec<(f64, f64)> = (0..10)
                .map(|i| {
                    let t = (f64::from(i) * 36.0 - 90.0).to_radians();
                    let r = if i % 2 == 0 {
                        1.0
                    } else {
                        (a(0, 19_098.0, 0.5) / 0.190_98 * 0.381_966).clamp(0.05, 0.95)
                    };
                    (r * t.cos(), r * t.sin())
                })
                .collect();
            closed(poly(&fit(&pts, w, h), true))
        }
        "homePlate" | "flowChartOffpageConnector" => {
            let x = w - ss * a(0, 50_000.0, if ss > 0.0 { w / ss } else { 1.0 });
            closed(poly(&[(0.0, 0.0), (x, 0.0), (w, h / 2.0), (x, h), (0.0, h)], true))
        }
        "chevron" => {
            let x = ss * a(0, 50_000.0, if ss > 0.0 { w / ss } else { 1.0 });
            closed(poly(
                &[(0.0, 0.0), (w - x, 0.0), (w, h / 2.0), (w - x, h), (0.0, h), (x, h / 2.0)],
                true,
            ))
        }
        "rightArrow" | "leftArrow" => {
            let head = ss * a(1, 50_000.0, if ss > 0.0 { w / ss } else { 1.0 });
            let pts = arrow_points(w, h, a(0, 50_000.0, 1.0), head);
            let flip = name == "leftArrow";
            let pts: Vec<(f64, f64)> =
                pts.iter().map(|(l, t)| (if flip { w - l } else { *l }, *t)).collect();
            closed(poly(&pts, true))
        }
        "upArrow" | "downArrow" => {
            let head = ss * a(1, 50_000.0, if ss > 0.0 { h / ss } else { 1.0 });
            let pts = arrow_points(h, w, a(0, 50_000.0, 1.0), head);
            let up = name == "upArrow";
            let pts: Vec<(f64, f64)> =
                pts.iter().map(|(l, t)| (*t, if up { h - l } else { *l })).collect();
            closed(poly(&pts, true))
        }
        "plus" | "mathPlus" => {
            let x = ss * a(0, 25_000.0, 0.5);
            closed(poly(
                &[
                    (x, 0.0),
                    (w - x, 0.0),
                    (w - x, x),
                    (w, x),
                    (w, h - x),
                    (w - x, h - x),
                    (w - x, h),
                    (x, h),
                    (x, h - x),
                    (0.0, h - x),
                    (0.0, x),
                    (x, x),
                ],
                true,
            ))
        }
        "line" | "straightConnector1" | "lineInv" => {
            let inv = name == "lineInv";
            Preset {
                segs: vec![
                    Seg::M(0.0, if inv { h } else { 0.0 }),
                    Seg::L(w, if inv { 0.0 } else { h }),
                ],
                open: true,
                rough: false,
            }
        }
        "bentConnector2" => {
            Preset { segs: poly(&[(0.0, 0.0), (w, 0.0), (w, h)], false), open: true, rough: false }
        }
        "bentConnector3" => {
            let x = w * a(0, 50_000.0, 10.0);
            Preset {
                segs: poly(&[(0.0, 0.0), (x, 0.0), (x, h), (w, h)], false),
                open: true,
                rough: false,
            }
        }
        // Other connectors: their path depends on adjust values and routing this table does not
        // model; the straight line between the ends is the closest cheap likeness.
        n if n.starts_with("bentConnector") || n.starts_with("curvedConnector") => {
            Preset { segs: vec![Seg::M(0.0, 0.0), Seg::L(w, h)], open: true, rough: true }
        }
        "round1Rect" => closed(corner_rect(w, h, [0.0, ss * a(0, 16_667.0, 0.5), 0.0, 0.0], true)),
        "round2SameRect" => {
            let (top, bottom) = (ss * a(0, 16_667.0, 0.5), ss * a(1, 0.0, 0.5));
            closed(corner_rect(w, h, [top, top, bottom, bottom], true))
        }
        "round2DiagRect" => {
            let (one, other) = (ss * a(0, 16_667.0, 0.5), ss * a(1, 0.0, 0.5));
            closed(corner_rect(w, h, [one, other, one, other], true))
        }
        "snip1Rect" | "flowChartPunchedCard" => {
            let r = ss * a(0, 16_667.0, 0.5);
            let cut = if name == "snip1Rect" { [0.0, r, 0.0, 0.0] } else { [r, 0.0, 0.0, 0.0] };
            closed(corner_rect(w, h, cut, false))
        }
        "snip2SameRect" => {
            let (top, bottom) = (ss * a(0, 16_667.0, 0.5), ss * a(1, 0.0, 0.5));
            closed(corner_rect(w, h, [top, top, bottom, bottom], false))
        }
        "snip2DiagRect" => {
            let (one, other) = (ss * a(0, 0.0, 0.5), ss * a(1, 16_667.0, 0.5));
            closed(corner_rect(w, h, [one, other, one, other], false))
        }
        "snipRoundRect" => {
            let r = ss * a(0, 16_667.0, 0.5);
            rough(corner_rect(w, h, [r, r, 0.0, 0.0], false))
        }
        "plaque" | "bevel" | "frame" | "foldedCorner" | "cube" => rough(rect(w, h)),
        "leftRightArrow" => {
            let head =
                (ss * a(1, 50_000.0, if ss > 0.0 { w / ss / 2.0 } else { 0.5 })).min(w / 2.0);
            let y1 = h * (1.0 - a(0, 50_000.0, 1.0)) / 2.0;
            closed(poly(
                &[
                    (0.0, h / 2.0),
                    (head, 0.0),
                    (head, y1),
                    (w - head, y1),
                    (w - head, 0.0),
                    (w, h / 2.0),
                    (w - head, h),
                    (w - head, h - y1),
                    (head, h - y1),
                    (head, h),
                ],
                true,
            ))
        }
        "upDownArrow" => {
            let head =
                (ss * a(1, 50_000.0, if ss > 0.0 { h / ss / 2.0 } else { 0.5 })).min(h / 2.0);
            let x1 = w * (1.0 - a(0, 50_000.0, 1.0)) / 2.0;
            closed(poly(
                &[
                    (w / 2.0, 0.0),
                    (w, head),
                    (w - x1, head),
                    (w - x1, h - head),
                    (w, h - head),
                    (w / 2.0, h),
                    (0.0, h - head),
                    (x1, h - head),
                    (x1, head),
                    (0.0, head),
                ],
                true,
            ))
        }
        "notchedRightArrow" | "stripedRightArrow" | "bentArrow" | "uturnArrow"
        | "curvedRightArrow" | "curvedLeftArrow" | "circularArrow" | "swooshArrow" => {
            let head = ss * 0.5_f64.min(if ss > 0.0 { w / ss } else { 1.0 });
            rough(poly(&arrow_points(w, h, 0.5, head), true))
        }
        "rightArrowCallout"
        | "leftArrowCallout"
        | "upArrowCallout"
        | "downArrowCallout"
        | "leftRightArrowCallout"
        | "quadArrowCallout"
        | "quadArrow"
        | "leftRightUpArrow"
        | "leftUpArrow"
        | "bentUpArrow" => rough(rect(w, h)),
        "star4" | "star6" | "star7" | "star8" | "star10" | "star12" | "star16" | "star24"
        | "star32" | "irregularSeal1" | "irregularSeal2" => {
            let points: u32 = name.trim_start_matches("star").parse().unwrap_or(12).clamp(4, 32);
            let inner = match points {
                4 => 0.25,
                6 => 0.577,
                7 => 0.69,
                8 | 10 => 0.75,
                _ => 0.8,
            };
            let pts: Vec<(f64, f64)> = (0..points * 2)
                .map(|i| {
                    let t = (f64::from(i) * 180.0 / f64::from(points) - 90.0).to_radians();
                    let r = if i % 2 == 0 { 1.0 } else { inner };
                    (r * t.cos(), r * t.sin())
                })
                .collect();
            let p = poly(&fit(&pts, w, h), true);
            if name.starts_with("star") { closed(p) } else { rough(p) }
        }
        "heptagon" | "decagon" | "dodecagon" => {
            let sides: u32 = match name {
                "heptagon" => 7,
                "decagon" => 10,
                _ => 12,
            };
            let pts: Vec<(f64, f64)> = (0..sides)
                .map(|i| {
                    let t = (f64::from(i) * 360.0 / f64::from(sides) - 90.0).to_radians();
                    (t.cos(), t.sin())
                })
                .collect();
            closed(poly(&fit(&pts, w, h), true))
        }
        "wedgeRectCallout"
        | "borderCallout1"
        | "borderCallout2"
        | "borderCallout3"
        | "accentCallout1"
        | "accentCallout2"
        | "accentCallout3"
        | "callout1"
        | "callout2"
        | "callout3"
        | "accentBorderCallout1"
        | "accentBorderCallout2"
        | "accentBorderCallout3" => rough(rect(w, h)),
        "wedgeRoundRectCallout" => rough(round_rect(w, h, ss * 0.166_67)),
        "wedgeEllipseCallout"
        | "cloudCallout"
        | "cloud"
        | "donut"
        | "noSmoking"
        | "sun"
        | "smileyFace"
        | "pie"
        | "chord"
        | "teardrop"
        | "blockArc"
        | "moon"
        | "heart"
        | "flowChartDelay"
        | "flowChartMagneticDrum"
        | "flowChartMagneticTape"
        | "flowChartDisplay"
        | "flowChartOnlineStorage"
        | "actionButtonBlank"
        | "pieWedge" => rough(ellipse(w, h)),
        "flowChartExtract" => closed(poly(&[(0.0, h), (w / 2.0, 0.0), (w, h)], true)),
        "flowChartMerge" => closed(poly(&[(0.0, 0.0), (w, 0.0), (w / 2.0, h)], true)),
        "flowChartSort" => {
            closed(poly(&[(w / 2.0, 0.0), (w, h / 2.0), (w / 2.0, h), (0.0, h / 2.0)], true))
        }
        "flowChartCollate" => closed(poly(&[(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)], true)),
        "flowChartManualInput" => closed(poly(&[(0.0, h / 5.0), (w, 0.0), (w, h), (0.0, h)], true)),
        "mathMinus" => closed(poly(
            &[
                (w * 0.13, h * 0.38),
                (w * 0.87, h * 0.38),
                (w * 0.87, h * 0.62),
                (w * 0.13, h * 0.62),
            ],
            true,
        )),
        // Brackets and braces are lines: only their frame's sides are hinted at.
        "leftBracket" | "leftBrace" => Preset {
            segs: poly(&[(w, 0.0), (0.0, 0.0), (0.0, h), (w, h)], false),
            open: true,
            rough: true,
        },
        "rightBracket" | "rightBrace" => Preset {
            segs: poly(&[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)], false),
            open: true,
            rough: true,
        },
        "bracketPair" | "bracePair" => Preset {
            segs: vec![
                Seg::M(w * 0.08, 0.0),
                Seg::L(0.0, 0.0),
                Seg::L(0.0, h),
                Seg::L(w * 0.08, h),
                Seg::M(w * 0.92, 0.0),
                Seg::L(w, 0.0),
                Seg::L(w, h),
                Seg::L(w * 0.92, h),
            ],
            open: true,
            rough: true,
        },
        "arc" => Preset {
            segs: vec![
                Seg::M(w / 2.0, 0.0),
                Seg::C(w / 2.0 + w / 2.0 * KAPPA, 0.0, w, h / 2.0 - h / 2.0 * KAPPA, w, h / 2.0),
            ],
            open: true,
            rough: true,
        },
        // A cylinder, a document with a wavy edge and the like: the frame is the likeness.
        "can"
        | "flowChartDocument"
        | "flowChartMultidocument"
        | "flowChartMagneticDisk"
        | "flowChartPunchedTape"
        | "wave"
        | "doubleWave"
        | "ribbon"
        | "ribbon2"
        | "ellipseRibbon"
        | "ellipseRibbon2"
        | "horizontalScroll"
        | "verticalScroll"
        | "gear6"
        | "gear9"
        | "funnel"
        | "lightningBolt"
        | "halfFrame"
        | "corner"
        | "diagStripe"
        | "mathMultiply"
        | "mathDivide"
        | "mathEqual"
        | "mathNotEqual"
        | "chartPlus"
        | "chartStar"
        | "chartX"
        | "cornerTabs"
        | "plaqueTabs"
        | "squareTabs" => rough(rect(w, h)),
        _ => return None,
    })
}

/// A rectangle whose corners (top left, top right, bottom right, bottom left) are each cut by
/// the given amount: rounded when `round`, snipped straight otherwise. Zero leaves the corner.
fn corner_rect(w: f64, h: f64, cut: [f64; 4], round: bool) -> Vec<Seg> {
    let lim = w.min(h) / 2.0;
    let [tl, tr, br, bl] = cut.map(|c| if c.is_finite() { c.clamp(0.0, lim) } else { 0.0 });
    let corner = |out: &mut Vec<Seg>, from: (f64, f64), at: (f64, f64), to: (f64, f64)| {
        if round {
            let k = 1.0 - KAPPA;
            out.push(Seg::C(
                from.0 + (at.0 - from.0) * (1.0 - k),
                from.1 + (at.1 - from.1) * (1.0 - k),
                to.0 + (at.0 - to.0) * (1.0 - k),
                to.1 + (at.1 - to.1) * (1.0 - k),
                to.0,
                to.1,
            ));
        } else {
            out.push(Seg::L(to.0, to.1));
        }
    };
    let mut out = vec![Seg::M(tl, 0.0), Seg::L(w - tr, 0.0)];
    corner(&mut out, (w - tr, 0.0), (w, 0.0), (w, tr));
    out.push(Seg::L(w, h - br));
    corner(&mut out, (w, h - br), (w, h), (w - br, h));
    out.push(Seg::L(bl, h));
    corner(&mut out, (bl, h), (0.0, h), (0.0, h - bl));
    out.push(Seg::L(0.0, tl));
    corner(&mut out, (0.0, tl), (0.0, 0.0), (tl, 0.0));
    out.push(Seg::Z);
    out
}

/// Whether an unknown preset is better hinted at by an outline than by a filled frame: lines,
/// brackets and connectors enclose nothing.
pub(super) fn is_line_like(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["connector", "bracket", "brace", "line", "arc"].iter().any(|k| n.contains(k))
}

/// The rectangle text is laid out in, `[x, y, w, h]` in the shape's own coordinates, for a frame
/// of `w × h`: inside the body of the shape, as the office suites place it (ISO 29500 gives a
/// text rectangle for every preset; these are the ones that differ visibly from the frame).
pub(super) fn text_rect(name: &str, w: f64, h: f64) -> [f64; 4] {
    let inset = |l: f64, t: f64, r: f64, b: f64| {
        let (x0, y0) = (w * l, h * t);
        [x0, y0, (w * (1.0 - l - r)).max(0.0), (h * (1.0 - t - b)).max(0.0)]
    };
    // The square inscribed in an ellipse leaves (1 - cos 45°) / 2 on each side.
    const ROUND: f64 = 0.146_45;
    match name {
        "ellipse"
        | "flowChartConnector"
        | "flowChartSummingJunction"
        | "flowChartOr"
        | "donut"
        | "sun"
        | "smileyFace"
        | "wedgeEllipseCallout"
        | "cloud"
        | "cloudCallout"
        | "noSmoking"
        | "pie"
        | "chord"
        | "teardrop" => inset(ROUND, ROUND, ROUND, ROUND),
        "diamond" | "flowChartDecision" | "flowChartSort" => inset(0.25, 0.25, 0.25, 0.25),
        "triangle" | "flowChartExtract" => inset(0.25, 0.5, 0.25, 0.0),
        "flowChartMerge" => inset(0.25, 0.0, 0.25, 0.5),
        "rtTriangle" => inset(0.083, 0.583, 0.417, 0.083),
        "rightArrow" | "notchedRightArrow" | "stripedRightArrow" => inset(0.0, 0.25, 0.12, 0.25),
        "leftArrow" => inset(0.12, 0.25, 0.0, 0.25),
        "leftRightArrow" => inset(0.12, 0.25, 0.12, 0.25),
        "upArrow" => inset(0.25, 0.12, 0.25, 0.0),
        "downArrow" => inset(0.25, 0.0, 0.25, 0.12),
        "upDownArrow" => inset(0.25, 0.12, 0.25, 0.12),
        "chevron" => {
            let k = if w > 0.0 { (h.min(w) * 0.5 / w).min(0.4) } else { 0.0 };
            inset(k, 0.0, k, 0.0)
        }
        "homePlate" | "flowChartOffpageConnector" => {
            let k = if w > 0.0 { (h.min(w) * 0.25 / w).min(0.4) } else { 0.0 };
            inset(0.0, 0.0, k, 0.0)
        }
        "hexagon"
        | "flowChartPreparation"
        | "octagon"
        | "heptagon"
        | "decagon"
        | "dodecagon"
        | "pentagon" => inset(0.12, 0.12, 0.12, 0.12),
        "parallelogram" | "flowChartInputOutput" | "trapezoid" | "flowChartManualOperation" => {
            let k = if w > 0.0 { (h.min(w) * 0.25 / w).min(0.3) } else { 0.0 };
            inset(k, 0.0, k, 0.0)
        }
        "plus" | "mathPlus" => inset(0.25, 0.25, 0.25, 0.25),
        "star5" => inset(0.25, 0.38, 0.25, 0.2),
        "star4" | "star6" | "star7" | "star8" | "star10" | "star12" | "star16" | "star24"
        | "star32" | "irregularSeal1" | "irregularSeal2" => inset(0.22, 0.22, 0.22, 0.22),
        "heart" => inset(0.17, 0.2, 0.17, 0.33),
        "can" | "flowChartMagneticDisk" => inset(0.0, 0.25, 0.0, 0.08),
        "cube" => inset(0.0, 0.25, 0.25, 0.0),
        "flowChartDocument" | "flowChartMultidocument" | "wave" => inset(0.0, 0.0, 0.0, 0.2),
        "flowChartTerminator" => inset(0.08, 0.0, 0.08, 0.0),
        "flowChartManualInput" => inset(0.0, 0.2, 0.0, 0.0),
        _ => [0.0, 0.0, w.max(0.0), h.max(0.0)],
    }
}

/// Appends the arc of an ellipse with radii `wr`, `hr` that starts at the current point `from`
/// at the visual angle `st` and sweeps `sw` (both in degrees, positive clockwise on the screen).
/// Returns the end point.
pub(super) fn arc_to(
    out: &mut Vec<Seg>,
    from: (f64, f64),
    wr: f64,
    hr: f64,
    st: f64,
    sw: f64,
) -> (f64, f64) {
    let sw = if sw.is_finite() { sw.clamp(-360.0, 360.0) } else { 0.0 };
    if !(wr.is_finite() && hr.is_finite()) || wr.abs() < 1e-9 || hr.abs() < 1e-9 || sw == 0.0 {
        return from;
    }
    let (wr, hr) = (wr.abs(), hr.abs());
    // The parametric angle of the point at visual angle `theta`.
    let param = |theta_deg: f64| {
        let t = theta_deg.to_radians();
        (wr * t.sin()).atan2(hr * t.cos())
    };
    let t0 = param(st);
    let center = (from.0 - wr * t0.cos(), from.1 - hr * t0.sin());
    let pieces = (sw.abs() / 90.0).ceil().clamp(1.0, 4.0);
    let step = sw / pieces;
    let mut ta = t0;
    let mut end = from;
    for i in 0..4_u8 {
        if f64::from(i) >= pieces {
            break;
        }
        let b = st + step * f64::from(i + 1);
        let a_vis = st + step * f64::from(i);
        // The parametric span of this piece, unwrapped into the direction of the sweep.
        let mut dt = param(b) - param(a_vis);
        if step > 0.0 {
            while dt <= 0.0 {
                dt += std::f64::consts::TAU;
            }
            while dt > std::f64::consts::PI {
                dt -= std::f64::consts::TAU;
            }
        } else {
            while dt >= 0.0 {
                dt -= std::f64::consts::TAU;
            }
            while dt < -std::f64::consts::PI {
                dt += std::f64::consts::TAU;
            }
        }
        let k = 4.0 / 3.0 * (dt / 4.0).tan();
        let tb = ta + dt;
        let p = |t: f64| (center.0 + wr * t.cos(), center.1 + hr * t.sin());
        let d = |t: f64| (-wr * t.sin(), hr * t.cos());
        let (pa, pb) = (p(ta), p(tb));
        let (da, db) = (d(ta), d(tb));
        out.push(Seg::C(
            pa.0 + k * da.0,
            pa.1 + k * da.1,
            pb.0 - k * db.0,
            pb.1 - k * db.1,
            pb.0,
            pb.1,
        ));
        end = pb;
        ta = tb;
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(segs: &[Seg]) -> (f64, f64, f64, f64) {
        let mut b = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut add = |x: f64, y: f64| {
            b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        };
        for s in segs {
            match *s {
                Seg::M(x, y) | Seg::L(x, y) => add(x, y),
                Seg::C(a, b2, c, d, e, f) => {
                    add(a, b2);
                    add(c, d);
                    add(e, f);
                }
                Seg::Z => {}
            }
        }
        b
    }

    #[test]
    fn every_preset_stays_inside_its_frame() {
        for name in [
            "rect",
            "roundRect",
            "ellipse",
            "triangle",
            "rtTriangle",
            "diamond",
            "parallelogram",
            "trapezoid",
            "hexagon",
            "octagon",
            "pentagon",
            "star5",
            "homePlate",
            "chevron",
            "rightArrow",
            "leftArrow",
            "upArrow",
            "downArrow",
            "plus",
            "line",
            "bentConnector3",
            "curvedConnector3",
            "flowChartTerminator",
        ] {
            let p = preset(name, 200.0, 100.0, [None, None]).unwrap_or_else(|| panic!("{name}"));
            let (x0, y0, x1, y1) = bounds(&p.segs);
            assert!(
                x0 >= -1e-6 && y0 >= -1e-6 && x1 <= 200.0 + 1e-6 && y1 <= 100.0 + 1e-6,
                "{name}: {x0} {y0} {x1} {y1}"
            );
        }
        assert!(preset("noSuchPreset", 10.0, 10.0, [None, None]).is_none());
    }

    #[test]
    fn extreme_adjust_values_stay_finite_and_inside() {
        for adj in [f64::NAN, f64::INFINITY, -5.0e9, 5.0e9] {
            for name in ["roundRect", "triangle", "parallelogram", "rightArrow", "star5", "plus"] {
                let p = preset(name, 80.0, 40.0, [Some(adj), Some(adj)]).expect("known");
                let (x0, y0, x1, y1) = bounds(&p.segs);
                assert!(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite());
                assert!(x0 >= -1e-6 && y0 >= -1e-6 && x1 <= 80.0 + 1e-6 && y1 <= 40.0 + 1e-6);
            }
        }
    }

    #[test]
    fn a_quarter_arc_ends_where_the_circle_does() {
        // Start at the rightmost point of a circle of radius 10 around (0, 0); sweep a quarter
        // clockwise on the screen (y down): the end is the bottom point.
        let mut out = Vec::new();
        let end = arc_to(&mut out, (10.0, 0.0), 10.0, 10.0, 0.0, 90.0);
        assert!((end.0).abs() < 1e-9 && (end.1 - 10.0).abs() < 1e-9, "{end:?}");
        assert_eq!(out.len(), 1);
        // A full circle comes back to the start.
        let mut out = Vec::new();
        let end = arc_to(&mut out, (10.0, 0.0), 10.0, 10.0, 0.0, 360.0);
        assert!((end.0 - 10.0).abs() < 1e-9 && end.1.abs() < 1e-9, "{end:?}");
    }
}
