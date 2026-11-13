// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

//! Entry points for the coverage-guided fuzz targets in `fuzz/` (feature `fuzzing`): the parts of
//! the readers that hostile documents reach but that the public API only reaches through a whole
//! file — the spreadsheet formula engine, number formats and the plain-text readers. Never part
//! of the app or the guest (the feature is off everywhere else).

use std::collections::BTreeMap;

use crate::formula::{Book, Cell, Sheet, Val};
use crate::model::FileFormat;
use crate::{PreviewPolicy, numfmt, plain, sink::Sink};

/// Calculates a small workbook whose cell `A1` holds `formula` (without `=`), next to a few cells
/// of every kind it may refer to, on a second sheet as well. Must end, and never panic.
pub fn formula(formula: &str) {
    let mut cells = BTreeMap::new();
    cells.insert((1, 0), Cell { value: Val::Empty, formula: Some(formula.to_owned()) });
    cells.insert((1, 1), Cell { value: Val::Num(2.5), formula: None });
    cells.insert((2, 0), Cell { value: Val::Str("x".to_owned()), formula: None });
    cells.insert((2, 1), Cell { value: Val::Bool(true), formula: None });
    cells.insert((3, 0), Cell { value: Val::Empty, formula: Some("A1+B1".to_owned()) });
    let mut other = BTreeMap::new();
    other.insert((1, 0), Cell { value: Val::Empty, formula: Some(formula.to_owned()) });
    let mut book = Book::new(false);
    book.add(Sheet::new("Sheet1", cells, true));
    book.add(Sheet::new("Data", other, true));
    let _ = book.calculate();
    let _ = crate::formula::shifted(formula, 3, -2);
}

/// Formats `value` and `text` with the number format `code`, in both date systems.
pub fn number_format(code: &str, value: f64, text: &str) {
    let f = numfmt::parse(code);
    let _ = f.number(value, false);
    let _ = f.number(value, true);
    let _ = f.text(text);
    let _ = numfmt::general(value);
    let _ = numfmt::serial_to_datetime(value, false);
}

/// Reads `data` as each kind of plain text (text, CSV, TSV, Markdown).
pub fn plain_text(data: &[u8]) {
    let Some(decoded) = plain::decode(data) else { return };
    let policy = PreviewPolicy::default();
    for format in [FileFormat::PlainText, FileFormat::Csv, FileFormat::Tsv, FileFormat::Markdown] {
        let mut sink = Sink::new(&policy);
        plain::extract(&mut sink, &decoded.text, format);
        let _ = sink.finish(format);
    }
}
