// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

import SwiftUI

/// The colour palettes of the app: five light and five dark, the same ten (and the same values)
/// as the Android client's `ui/Theme.java`, so that both apps look like one product.
public enum ArcaTheme: String, CaseIterable, Identifiable, Sendable {
    // Light.
    case alabaster = "Alabaster Studio"
    case cashmere = "Cashmere & Cappuccino"
    case nordicPearl = "Nordic Pearl"
    case sageLinen = "Sage & Linen"
    case roseQuartz = "Rose Quartz Daylight"
    // Dark.
    case obsidian = "Obsidian Vault"
    case sierraBlue = "Sierra Blue & Slate"
    case emeraldSecOps = "Emerald SecOps"
    case desertTitanium = "Desert Titanium"
    case midnightAmethyst = "Midnight Amethyst"

    public var id: String { rawValue }

    /// The colours of one palette as `0xRRGGBB`.
    struct Palette: Sendable {
        let background: UInt32
        let surface: UInt32
        let surfaceSecondary: UInt32
        let textPrimary: UInt32
        let textSecondary: UInt32
        let outline: UInt32
        let accent: UInt32
        let onAccent: UInt32
    }

    var palette: Palette {
        switch self {
        case .alabaster:
            // Ivory paper, ink and antique gold.
            return Palette(
                background: 0xFBFAF6, surface: 0xFFFFFF, surfaceSecondary: 0xF1EEE5,
                textPrimary: 0x17150F, textSecondary: 0x5B564A, outline: 0xDCD5C4,
                accent: 0x7A5A12, onAccent: 0xFFFFFF)
        case .cashmere:
            // Cashmere, espresso and cognac leather.
            return Palette(
                background: 0xF3E9DA, surface: 0xFBF4E8, surfaceSecondary: 0xE8D8C2,
                textPrimary: 0x2B1D12, textSecondary: 0x634C3A, outline: 0xCFB99C,
                accent: 0x6B3A1E, onAccent: 0xFFFFFF)
        case .nordicPearl:
            // Pearl grey and midnight navy.
            return Palette(
                background: 0xEEF1F5, surface: 0xFAFBFD, surfaceSecondary: 0xDEE4EC,
                textPrimary: 0x0E151D, textSecondary: 0x3F4B59, outline: 0xC2CCD8,
                accent: 0x16365E, onAccent: 0xFFFFFF)
        case .sageLinen:
            // Linen and racing green.
            return Palette(
                background: 0xEEEFE6, surface: 0xF9FAF4, surfaceSecondary: 0xDFE3D3,
                textPrimary: 0x181D15, textSecondary: 0x49523F, outline: 0xC6CCB7,
                accent: 0x1F4A33, onAccent: 0xFFFFFF)
        case .roseQuartz:
            // Rose quartz and bordeaux.
            return Palette(
                background: 0xF7EEEE, surface: 0xFFF9F8, surfaceSecondary: 0xEEDDDD,
                textPrimary: 0x271619, textSecondary: 0x664A4E, outline: 0xDEC6C8,
                accent: 0x7A1F3D, onAccent: 0xFFFFFF)
        case .obsidian:
            // True black and champagne gold.
            return Palette(
                background: 0x000000, surface: 0x0B0B0C, surfaceSecondary: 0x17171A,
                textPrimary: 0xECE9E2, textSecondary: 0xA7A39A, outline: 0x2A2823,
                accent: 0xD9B56D, onAccent: 0x2A1D00)
        case .sierraBlue:
            // Midnight slate and ice sapphire.
            return Palette(
                background: 0x0B121C, surface: 0x121C2A, surfaceSecondary: 0x1C2A3D,
                textPrimary: 0xE4EAF2, textSecondary: 0x9FB0C5, outline: 0x2B3B52,
                accent: 0x8FB8E8, onAccent: 0x06203C)
        case .emeraldSecOps:
            // Black emerald.
            return Palette(
                background: 0x04110C, surface: 0x0A1C15, surfaceSecondary: 0x112B21,
                textPrimary: 0xDDF2E7, textSecondary: 0x92B5A3, outline: 0x1E4436,
                accent: 0x4FCF9B, onAccent: 0x00281A)
        case .desertTitanium:
            // Burnished titanium and bronze.
            return Palette(
                background: 0x14110D, surface: 0x1E1A15, surfaceSecondary: 0x2B261F,
                textPrimary: 0xEFE6D8, textSecondary: 0xC0B39F, outline: 0x453C30,
                accent: 0xE0B27A, onAccent: 0x2C1B06)
        case .midnightAmethyst:
            // Deep amethyst and moonlit silver.
            return Palette(
                background: 0x0E0B17, surface: 0x161221, surfaceSecondary: 0x221C33,
                textPrimary: 0xEBE6F5, textSecondary: 0xABA2C2, outline: 0x342C49,
                accent: 0xC3A6F5, onAccent: 0x22103F)
        }
    }

    /// Whether the palette is a dark one (the system controls follow it).
    public var isDark: Bool {
        switch self {
        case .alabaster, .cashmere, .nordicPearl, .sageLinen, .roseQuartz: return false
        case .obsidian, .sierraBlue, .emeraldSecOps, .desertTitanium, .midnightAmethyst: return true
        }
    }

    /// The colour scheme the system controls (alerts, pickers, the status bar) are shown in.
    public var colorScheme: ColorScheme { isDark ? .dark : .light }

    static func color(_ rgb: UInt32) -> Color {
        Color(
            .sRGB,
            red: Double((rgb >> 16) & 0xff) / 255,
            green: Double((rgb >> 8) & 0xff) / 255,
            blue: Double(rgb & 0xff) / 255,
            opacity: 1)
    }

    /// Background color of the main interface canvas.
    public var background: Color { Self.color(palette.background) }
    /// Card, cell and sheet surface color.
    public var surface: Color { Self.color(palette.surface) }
    /// Secondary elevated surface (e.g. tag chips, button backgrounds).
    public var surfaceSecondary: Color { Self.color(palette.surfaceSecondary) }
    /// Primary high-contrast text color.
    public var textPrimary: Color { Self.color(palette.textPrimary) }
    /// Secondary muted text color for captions and metadata.
    public var textSecondary: Color { Self.color(palette.textSecondary) }
    /// Hairlines and the outlines of cards.
    public var outline: Color { Self.color(palette.outline) }
    /// Primary accent tint.
    public var accent: Color { Self.color(palette.accent) }
    /// Text and glyphs drawn on the accent colour (a filled button).
    public var onAccent: Color { Self.color(palette.onAccent) }

    /// The display face of the app: a serif, for the headline of a screen.
    public static func display(_ size: CGFloat) -> Font {
        .system(size: size, weight: .bold, design: .serif)
    }

    /// WCAG contrast ratio of two `0xRRGGBB` colours (for the tests of the palettes).
    static func contrast(_ a: UInt32, _ b: UInt32) -> Double {
        func channel(_ v: UInt32) -> Double {
            let c = Double(v & 0xff) / 255
            return c <= 0.03928 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4)
        }
        func luminance(_ c: UInt32) -> Double {
            0.2126 * channel(c >> 16) + 0.7152 * channel(c >> 8) + 0.0722 * channel(c)
        }
        let (la, lb) = (luminance(a), luminance(b))
        return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)
    }

    /// Clean verdict color (Shield green).
    public static let cleanGreen = Color(red: 0.13, green: 0.75, blue: 0.42)
    /// Warning verdict color (Trackers amber).
    public static let trackerAmber = Color(red: 0.95, green: 0.65, blue: 0.15)
    /// Active payload neutralized color (Active red).
    public static let activeRed = Color(red: 0.92, green: 0.28, blue: 0.25)
    /// Rejected threat color (Crimson).
    public static let rejectedCrimson = Color(red: 0.78, green: 0.10, blue: 0.15)
    /// Unverified status color (Muted slate).
    public static let unverifiedSlate = Color(red: 0.55, green: 0.60, blue: 0.65)
}

/// Dynamic spring animation parameters mandated by the Arca specification:
/// `mass: 1.0, stiffness: 480, damping: 34` (critically damped, interruption-friendly).
public extension Animation {
    static var arcaSpring: Animation {
        .interpolatingSpring(mass: 1.0, stiffness: 480, damping: 34)
    }

    static var arcaBouncy: Animation {
        .interpolatingSpring(mass: 0.9, stiffness: 350, damping: 25)
    }
}
