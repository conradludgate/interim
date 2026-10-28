use core::fmt;

use super::{DateError, DateErrorWithInput};

const TAB_WIDTH: usize = 4;

// Keep terminal layout deliberately small: one ASCII line, with tabs expanded.
// Unicode/control characters or multi-line spans use the escaped compact report.
pub(super) fn format(
    report: &DateErrorWithInput<'_>,
    f: &mut fmt::Formatter<'_>,
) -> Option<fmt::Result> {
    let (expected, span) = match report.error {
        DateError::ExpectedToken(expected, span) => (*expected, span.clone()),
        DateError::EndOfText(expected) => (*expected, report.input.len()..report.input.len()),
        _ => return None,
    };
    let found = report.input.get(span.clone())?;
    let prefix = report.input.get(..span.start)?;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let line_end = report
        .input
        .get(span.start..)?
        .find('\n')
        .map_or(report.input.len(), |index| span.start + index);
    let line = report.input.get(line_start..line_end)?;
    let line = line.strip_suffix('\r').unwrap_or(line);
    if span.end > line_start + line.len()
        || !line
            .bytes()
            .all(|byte| byte == b'\t' || byte == b' ' || byte.is_ascii_graphic())
    {
        return None;
    }

    let number = report.input[..line_start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let gutter = number.ilog10() as usize + 1;
    let column = columns(&line[..span.start - line_start]);
    let end_column = columns(&line[..span.end - line_start]);
    let width = (end_column - column).max(1);

    Some((|| {
        match report.error {
            DateError::EndOfText(_) => writeln!(f, "unexpected end of input")?,
            _ => writeln!(f, "unexpected input {found:?}")?,
        }
        writeln!(f, "{:>gutter$} |", "")?;
        write!(f, "{number:>gutter$} | ")?;
        let mut displayed = 0;
        for byte in line.bytes() {
            if byte == b'\t' {
                let spaces = TAB_WIDTH - displayed % TAB_WIDTH;
                write!(f, "{:spaces$}", "")?;
                displayed += spaces;
            } else {
                write!(f, "{}", char::from(byte))?;
                displayed += 1;
            }
        }
        write!(f, "\n{:>gutter$} | {:column$}", "", "")?;
        for _ in 0..width {
            f.write_str("^")?;
        }
        write!(f, " expected {expected}")
    })())
}

fn columns(text: &str) -> usize {
    text.bytes().fold(0, |column, byte| {
        column
            + if byte == b'\t' {
                TAB_WIDTH - column % TAB_WIDTH
            } else {
                1
            }
    })
}
