// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

//! Characters of the "symbol" fonts as text.
//!
//! Symbol, Wingdings, Webdings and ZapfDingbats do not follow Unicode: the code of the letter `a`
//! selects a Greek alpha or a tick. A document stores such a character either as the plain code
//! (`a`, `ü`) with the name of the font next to it, or in the private-use range `U+F020..=U+F0FF`.
//! A reader that has not got the font shows the plain code as a Latin letter and the private-use
//! one as an empty box, which a person takes for a broken encoding. This module turns them into
//! the Unicode characters that look like the glyph, so that the fonts of the device can draw them.
//!
//! The tables are data only; nothing here is taken from the document but one byte per character.

/// A font whose codes are pictures, not letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SymbolFont {
    /// Adobe/Microsoft "Symbol": Greek and mathematics.
    Symbol,
    /// "Wingdings".
    Wingdings,
    /// "Wingdings 2".
    Wingdings2,
    /// "Wingdings 3": arrows.
    Wingdings3,
    /// "Webdings".
    Webdings,
    /// "ZapfDingbats" / "ITC Zapf Dingbats".
    Dingbats,
}

/// What stands for a glyph that has no close Unicode character.
pub(crate) const FALLBACK: char = '•';

/// The symbol font a typeface name denotes, if it is one.
pub(crate) fn symbol_font(face: &str) -> Option<SymbolFont> {
    let name: String = face
        .trim()
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
        .take(40)
        .collect();
    Some(match name.as_str() {
        "symbol" | "symbolmt" | "symbolps" | "standardsymbolsl" | "standardsymbolsps"
        | "opensymbol" | "starsymbol" => SymbolFont::Symbol,
        "wingdings" | "wingdingsregular" => SymbolFont::Wingdings,
        "wingdings2" => SymbolFont::Wingdings2,
        "wingdings3" => SymbolFont::Wingdings3,
        "webdings" => SymbolFont::Webdings,
        "zapfdingbats" | "itczapfdingbats" | "dingbats" | "monotypesorts" => SymbolFont::Dingbats,
        _ => return None,
    })
}

/// The code a character selects in a symbol font: the low byte of a private-use `U+F0xx`, or the
/// character itself when it fits a byte. `None` for anything else (a real Unicode character that
/// the font would not have drawn as a symbol).
pub(crate) fn code_of(c: char) -> Option<u8> {
    let v = u32::from(c);
    match v {
        0xF020..=0xF0FF => u8::try_from(v - 0xF000).ok(),
        0x20..=0xFF => u8::try_from(v).ok(),
        _ => None,
    }
}

/// Whether `c` is a private-use code of a symbol font (`U+F020..=U+F0FF`).
pub(crate) const fn is_symbol_private(c: char) -> bool {
    matches!(c, '\u{F020}'..='\u{F0FF}')
}

// Sixteen codes a row, 0x20..=0xFF.
const WINGDINGS: &str = concat!(
    " ✏✂✁👓🔔📖🕯☎✆✉✉📪📫📬📭",
    "📁📂📄📄📄🗄⌛⌨🖱🖲💻💾💾💾✇✍",
    "✍✌👌👍👎☜☞☝☟✋☺😐☹💣☠⚐",
    "⚑✈☼💧❄✝✞✟✠✡☪☯ॐ☸♈♉",
    "♊♋♌♍♎♏♐♑♒♓&&●❍■□",
    "□❑❒⬧⧫◆❖⬥⌧⍓⌘❀✿❝❞▯",
    "⓪①②③④⑤⑥⑦⑧⑨⑩⓿❶❷❸❹",
    "❺❻❼❽❾❿☙☙❧❧☙❧❦❦·•",
    "▪○⭕◎◉◎❍▪◻✦✦★✶✴✹✵",
    "⌖⌖⟡⌑❓✪✰🕐🕑🕒🕓🕔🕕🕖🕗🕘",
    "🕙🕚🕛↲↳↰↱↵↴⤶⤷❦❦❦❦❦",
    "❦❦❦❦❦⌫⌦◀➢▲▼⇦➲⇧⇩←",
    "→↑↓↖↗↙↘⬅➔⬆⬇⬉⬈⬋⬊⇦",
    "⇨⇧⇩⬄⇳⬁⬀⬃⬂▭▫✗✓☒☑⊞",
);

const SYMBOL: &str = concat!(
    " !∀#∃%&∋()∗+,−./",
    "0123456789:;<=>?",
    "≅ΑΒΧΔΕΦΓΗΙϑΚΛΜΝΟ",
    "ΠΘΡΣΤΥςΩΞΨΖ[∴]⊥_",
    "‾αβχδεφγηιϕκλμνο",
    "πθρστυϖωξψζ{|}∼ ",
    "                ",
    "                ",
    "€ϒ′≤⁄∞ƒ♣♦♥♠↔←↑→↓",
    "°±″≥×∝∂•÷≠≡≈…⏐⎯↵",
    "ℵℑℜ℘⊗⊕∅∩∪⊃⊇⊄⊂⊆∈∉",
    "∠∇®©™∏√⋅¬∧∨⇔⇐⇑⇒⇓",
    "◊〈®©™∑⎛⎜⎝⎡⎢⎣⎧⎨⎩⎪",
    " 〉∫⌠⎮⌡⎞⎟⎠⎤⎥⎦⎫⎬⎭ ",
);

/// Codes of ZapfDingbats in 0x21..=0x7E that are not `U+2700 + (code - 0x20)`.
const DINGBATS_MOVED: &[(u8, char)] = &[
    (0x25, '☎'),
    (0x2A, '☛'),
    (0x2B, '☞'),
    (0x48, '★'),
    (0x6C, '●'),
    (0x6E, '■'),
    (0x73, '▲'),
    (0x74, '▼'),
    (0x75, '◆'),
    (0x77, '◗'),
];

fn nth(table: &str, code: u8) -> Option<char> {
    let c = table.chars().nth(usize::from(code.checked_sub(0x20)?))?;
    (c != ' ' || code == 0x20).then_some(c)
}

fn from_u32(v: u32) -> Option<char> {
    char::from_u32(v)
}

fn dingbat(code: u8) -> Option<char> {
    let c = u32::from(code);
    match code {
        0x20 => Some(' '),
        0x21..=0x7E => DINGBATS_MOVED
            .iter()
            .find(|(k, _)| *k == code)
            .map(|(_, ch)| *ch)
            .or_else(|| from_u32(0x2700 + c - 0x20)),
        0xA1..=0xA7 => from_u32(0x2761 + c - 0xA1),
        0xA8 => Some('♣'),
        0xA9 => Some('♦'),
        0xAA => Some('♥'),
        0xAB => Some('♠'),
        0xAC..=0xB5 => from_u32(0x2460 + c - 0xAC),
        0xB6..=0xD4 => from_u32(0x2776 + c - 0xB6),
        0xD5 => Some('→'),
        0xD6 => Some('↔'),
        0xD7 => Some('↕'),
        0xD8..=0xEF => from_u32(0x2798 + c - 0xD8),
        0xF1..=0xFE => from_u32(0x27B1 + c - 0xF1),
        _ => None,
    }
}

/// The well-known glyphs of Wingdings 2; the rest are approximated by their neighbourhood.
fn wingdings2(code: u8) -> Option<char> {
    Some(match code {
        0x20 => ' ',
        0x4F | 0x51 => '✗',
        0x50 => '✓',
        0x52 => '☑',
        0x53 | 0x54 => '☒',
        0x55..=0x58 => '⊗',
        0x59 | 0x5A => '⦸',
        0x69 => '⓪',
        0x6A..=0x73 => from_u32(0x2460 + u32::from(code) - 0x6A)?,
        0x74 => '⓿',
        0x75..=0x7E => from_u32(0x2776 + u32::from(code) - 0x75)?,
        0x80 => '☉',
        0x82 | 0x83 => '☽',
        0x95 => '•',
        0x96..=0x98 => '●',
        0x99..=0x9E => '○',
        0x9F => '▪',
        0xA0..=0xA2 => '■',
        0xA3..=0xAD => '□',
        0xAE..=0xB7 => '◆',
        0xB8..=0xC7 => '✦',
        0xC8..=0xDF => '★',
        0xE0..=0xFF => '✶',
        _ => return None,
    })
}

/// Wingdings 3 is a font of arrows and triangles.
fn wingdings3(code: u8) -> Option<char> {
    Some(match code {
        0x20 => ' ',
        0x21 | 0x25 | 0x29 | 0x2D | 0x31 | 0x35 | 0x39 | 0x3D | 0x41 | 0x45 | 0x49 | 0x4D => '←',
        0x22 | 0x26 | 0x2A | 0x2E | 0x32 | 0x36 | 0x3A | 0x3E | 0x42 | 0x46 | 0x4A | 0x4E => '→',
        0x23 | 0x27 | 0x2B | 0x2F | 0x33 | 0x37 | 0x3B | 0x3F | 0x43 | 0x47 | 0x4B | 0x4F => '↑',
        0x24 | 0x28 | 0x2C | 0x30 | 0x34 | 0x38 | 0x3C | 0x40 | 0x44 | 0x48 | 0x4C | 0x50 => '↓',
        0x51 | 0x52 => '↺',
        0x53..=0x5F => '↔',
        0x60..=0x6F => '↕',
        0x70 => '▲',
        0x71 => '▼',
        0x72 => '△',
        0x73 => '▽',
        0x74 => '◀',
        0x75 => '▶',
        0x76 => '◁',
        0x77 => '▷',
        0x78 | 0x7A => '◣',
        0x79 | 0x7B => '◢',
        0x7C => '◄',
        0x7D | 0x84 | 0x86 => '►',
        0x7E | 0x80 | 0x82 => '▲',
        0x7F | 0x81 | 0x83 => '▼',
        0x85 => '◄',
        0x87..=0xFF => '➔',
        _ => return None,
    })
}

/// The well-known glyphs of Webdings.
fn webdings(code: u8) -> Option<char> {
    Some(match code {
        0x20 => ' ',
        0x33 => '◀',
        0x34 => '▶',
        0x35 => '▲',
        0x36 => '▼',
        0x37 => '⏪',
        0x38 => '⏩',
        0x39 => '⏮',
        0x3A => '⏭',
        0x3B => '⏸',
        0x3C => '■',
        0x3D | 0x6E => '●',
        0x48 => '🏠',
        0x59 => '♥',
        0x61 => '✔',
        0x63 | 0x67 => '■',
        0x69 => 'ℹ',
        0x72 => '✖',
        0x73 => '❓',
        0x78 => '⛔',
        0x7C => '|',
        0x99 => '✉',
        0x9A => '📧',
        0xC8 => '📞',
        0xC9 => '☎',
        0xFB => '🌍',
        0xFC => '🌎',
        _ => return None,
    })
}

/// The Unicode character that looks like the glyph `code` of `font`, if one is known.
pub(crate) fn glyph(font: SymbolFont, code: u8) -> Option<char> {
    match font {
        SymbolFont::Symbol => nth(SYMBOL, code),
        SymbolFont::Wingdings => nth(WINGDINGS, code),
        SymbolFont::Wingdings2 => wingdings2(code),
        SymbolFont::Wingdings3 => wingdings3(code),
        SymbolFont::Webdings => webdings(code),
        SymbolFont::Dingbats => dingbat(code),
    }
}

/// One character of a run set in `font`: its look-alike, [`FALLBACK`] for a glyph without one,
/// and the character itself when the font would not have drawn it as a symbol.
pub(crate) fn map_char(font: SymbolFont, c: char) -> char {
    // Only the spaces a person types: U+0085 is "white space" too, and a code of these fonts.
    if matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{A0}') {
        return c;
    }
    match code_of(c) {
        Some(code) => glyph(font, code).unwrap_or(FALLBACK),
        None => c,
    }
}

/// Text of a run set in `font` (see [`map_char`]).
pub(crate) fn map_text(font: SymbolFont, text: &str) -> String {
    text.chars().map(|c| map_char(font, c)).collect()
}

/// Text of a run whose font is an ordinary one, or unknown: private-use symbol codes cannot be
/// drawn by any font of the device, so they become `guess`'s glyphs (Symbol by default, where
/// `U+F0B7` is the bullet every list of Word uses).
pub(crate) fn map_private(text: &str, guess: Option<SymbolFont>) -> std::borrow::Cow<'_, str> {
    if !text.chars().any(is_symbol_private) {
        return std::borrow::Cow::Borrowed(text);
    }
    let font = guess.unwrap_or(SymbolFont::Symbol);
    std::borrow::Cow::Owned(
        text.chars().map(|c| if is_symbol_private(c) { map_char(font, c) } else { c }).collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_cover_every_code_exactly() {
        assert_eq!(WINGDINGS.chars().count(), 224);
        assert_eq!(SYMBOL.chars().count(), 224);
    }

    #[test]
    fn typeface_names_are_recognised_loosely() {
        assert_eq!(symbol_font("Wingdings"), Some(SymbolFont::Wingdings));
        assert_eq!(symbol_font(" wingdings 2 "), Some(SymbolFont::Wingdings2));
        assert_eq!(symbol_font("Wingdings-3"), Some(SymbolFont::Wingdings3));
        assert_eq!(symbol_font("SYMBOL"), Some(SymbolFont::Symbol));
        assert_eq!(symbol_font("ZapfDingbats"), Some(SymbolFont::Dingbats));
        assert_eq!(symbol_font("Arial"), None);
        assert_eq!(symbol_font("Segoe UI Symbol"), None);
        assert_eq!(symbol_font(&"w".repeat(100_000)), None);
    }

    #[test]
    fn the_bullets_powerpoint_writes_become_real_characters() {
        // The bullets of the stock templates: "§", "Ø", "ü", "q", "v" in Wingdings.
        assert_eq!(map_char(SymbolFont::Wingdings, '§'), '▪');
        assert_eq!(map_char(SymbolFont::Wingdings, 'Ø'), '➢');
        assert_eq!(map_char(SymbolFont::Wingdings, 'ü'), '✓');
        assert_eq!(map_char(SymbolFont::Wingdings, 'q'), '❑');
        assert_eq!(map_char(SymbolFont::Wingdings, 'v'), '❖');
        assert_eq!(map_char(SymbolFont::Wingdings, 'l'), '●');
        assert_eq!(map_char(SymbolFont::Wingdings, 'n'), '■');
        // The arrow autocorrect makes of "-->", stored in the private-use range.
        assert_eq!(map_char(SymbolFont::Wingdings, '\u{F0E0}'), '→');
        assert_eq!(map_char(SymbolFont::Wingdings, 'à'), '→');
        assert_eq!(map_char(SymbolFont::Wingdings, '\u{F0FC}'), '✓');
        assert_eq!(map_char(SymbolFont::Wingdings3, 'u'), '▶');
        assert_eq!(map_char(SymbolFont::Wingdings2, 'P'), '✓');
        assert_eq!(map_char(SymbolFont::Webdings, 'a'), '✔');
    }

    #[test]
    fn symbol_is_greek_and_mathematics() {
        assert_eq!(map_text(SymbolFont::Symbol, "abgDW"), "αβγΔΩ");
        assert_eq!(map_char(SymbolFont::Symbol, '\u{F0B7}'), '•');
        assert_eq!(map_char(SymbolFont::Symbol, '\u{F0B1}'), '±');
        assert_eq!(map_char(SymbolFont::Symbol, '\u{F0A3}'), '≤');
        assert_eq!(map_char(SymbolFont::Symbol, '\u{F0E5}'), '∑');
        assert_eq!(map_char(SymbolFont::Symbol, '5'), '5');
        // A code the font leaves empty is a neutral bullet, never the Latin letter.
        assert_eq!(map_char(SymbolFont::Symbol, '\u{F0F0}'), FALLBACK);
    }

    #[test]
    fn dingbats_follow_the_unicode_block() {
        assert_eq!(map_char(SymbolFont::Dingbats, '4'), '✔');
        assert_eq!(map_char(SymbolFont::Dingbats, 'l'), '●');
        assert_eq!(map_char(SymbolFont::Dingbats, 'n'), '■');
        assert_eq!(map_char(SymbolFont::Dingbats, 'H'), '★');
        assert_eq!(map_char(SymbolFont::Dingbats, '\u{AC}'), '①');
        assert_eq!(map_char(SymbolFont::Dingbats, '\u{D5}'), '→');
    }

    #[test]
    fn real_text_and_spaces_pass_through() {
        assert_eq!(map_text(SymbolFont::Wingdings, "Привет 日本"), "Привет 日本");
        assert_eq!(map_text(SymbolFont::Wingdings, " \t"), " \t");
        assert_eq!(map_private("обычный текст", None), "обычный текст");
        assert!(matches!(map_private("abc", None), std::borrow::Cow::Borrowed(_)));
        assert_eq!(map_private("a\u{F0B7}b", None), "a•b");
        assert_eq!(map_private("\u{F0E0}", Some(SymbolFont::Wingdings)), "→");
    }

    #[test]
    fn every_code_of_every_font_maps_to_a_printable_character() {
        for font in [
            SymbolFont::Symbol,
            SymbolFont::Wingdings,
            SymbolFont::Wingdings2,
            SymbolFont::Wingdings3,
            SymbolFont::Webdings,
            SymbolFont::Dingbats,
        ] {
            for code in 0x20u8..=0xFF {
                let c = map_char(font, char::from(code));
                assert!(!c.is_control(), "{font:?} {code:#x}");
                assert!(!is_symbol_private(c), "{font:?} {code:#x}");
                let private = char::from_u32(0xF000 + u32::from(code)).unwrap_or(' ');
                assert!(!is_symbol_private(map_char(font, private)), "{font:?} {code:#x}");
            }
        }
    }
}
