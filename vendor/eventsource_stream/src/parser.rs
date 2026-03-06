use nom::branch::alt;
use nom::bytes::streaming::{take_while, take_while1, take_while_m_n};
use nom::combinator::opt;
use nom::sequence::{preceded, terminated};
use nom::IResult;
use nom::Parser;
use nom::{error::ErrorKind, Err, Needed};

/// ; ABNF definition from HTML spec
///
/// stream        = [ bom ] *event
/// event         = *( comment / field ) end-of-line
/// comment       = colon *any-char end-of-line
/// field         = 1*name-char [ colon [ space ] *any-char ] end-of-line
/// end-of-line   = ( cr lf / cr / lf )
///
/// ; characters
/// lf            = %x000A ; U+000A LINE FEED (LF)
/// cr            = %x000D ; U+000D CARRIAGE RETURN (CR)
/// space         = %x0020 ; U+0020 SPACE
/// colon         = %x003A ; U+003A COLON (:)
/// bom           = %xFEFF ; U+FEFF BYTE ORDER MARK
/// name-char     = %x0000-0009 / %x000B-000C / %x000E-0039 / %x003B-10FFFF
///                 ; a scalar value other than U+000A LINE FEED (LF), U+000D CARRIAGE RETURN (CR), or U+003A COLON (:)
/// any-char      = %x0000-0009 / %x000B-000C / %x000E-10FFFF
///                 ; a scalar value other than U+000A LINE FEED (LF) or U+000D CARRIAGE RETURN (CR)

#[derive(Debug)]
pub enum RawEventLine<'a> {
    Comment,
    Field(&'a str, Option<&'a str>),
    Empty,
}

#[inline]
pub fn is_lf(c: char) -> bool {
    c == '\u{000A}'
}

#[inline]
pub fn is_space(c: char) -> bool {
    c == '\u{0020}'
}

#[inline]
pub fn is_colon(c: char) -> bool {
    c == '\u{003A}'
}

#[inline]
pub fn is_bom(c: char) -> bool {
    c == '\u{feff}'
}

#[inline]
pub fn is_name_char(c: char) -> bool {
    matches!(c, '\u{0000}'..='\u{0009}'
        | '\u{000B}'..='\u{000C}'
        | '\u{000E}'..='\u{0039}'
        | '\u{003B}'..='\u{10FFFF}')
}

#[inline]
pub fn is_any_char(c: char) -> bool {
    matches!(c, '\u{0000}'..='\u{0009}'
        | '\u{000B}'..='\u{000C}'
        | '\u{000E}'..='\u{10FFFF}')
}

#[inline]
fn end_of_line(input: &str) -> IResult<&str, &str> {
    if input.is_empty() {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    if let Some(rem) = input.strip_prefix("\u{000D}\u{000A}") {
        return Ok((rem, &input[..2]));
    }

    if let Some(rem) = input.strip_prefix('\u{000D}') {
        return Ok((rem, &input[..1]));
    }

    if let Some(rem) = input.strip_prefix('\u{000A}') {
        return Ok((rem, &input[..1]));
    }

    Err(Err::Error(nom::error::Error::new(input, ErrorKind::CrLf)))
}

#[inline]
fn comment(input: &str) -> IResult<&str, RawEventLine<'_>> {
    preceded(
        take_while_m_n(1, 1, is_colon),
        terminated(take_while(is_any_char), end_of_line),
    )
    .parse(input)
    .map(|(input, _)| (input, RawEventLine::Comment))
}

#[inline]
fn field(input: &str) -> IResult<&str, RawEventLine<'_>> {
    terminated(
        (
            take_while1(is_name_char),
            opt(preceded(
                take_while_m_n(1, 1, is_colon),
                preceded(opt(take_while_m_n(1, 1, is_space)), take_while(is_any_char)),
            )),
        ),
        end_of_line,
    )
    .parse(input)
    .map(|(input, (field, data))| (input, RawEventLine::Field(field, data)))
}

#[inline]
fn empty(input: &str) -> IResult<&str, RawEventLine<'_>> {
    end_of_line(input).map(|(i, _)| (i, RawEventLine::Empty))
}

pub fn line(input: &str) -> IResult<&str, RawEventLine<'_>> {
    alt((comment, field, empty)).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_line_leaves_trailing_empty_line_in_buffer() {
        let (rem, parsed) = line("data: Hello, world!\n\n").expect("field line should parse");

        match parsed {
            RawEventLine::Field(name, Some(value)) => {
                assert_eq!(name, "data");
                assert_eq!(value, "Hello, world!");
            }
            _ => panic!("expected field line"),
        }

        assert_eq!(rem, "\n");
    }

    #[test]
    fn empty_line_parses_from_single_newline() {
        let (rem, parsed) = line("\n").expect("empty line should parse");

        assert!(matches!(parsed, RawEventLine::Empty));
        assert_eq!(rem, "");
    }

    #[test]
    fn empty_line_parses_from_single_carriage_return() {
        let (rem, parsed) = line("\r").expect("empty CR line should parse");

        assert!(matches!(parsed, RawEventLine::Empty));
        assert_eq!(rem, "");
    }

    #[test]
    fn empty_line_parses_from_crlf() {
        let (rem, parsed) = line("\r\n").expect("empty CRLF line should parse");

        assert!(matches!(parsed, RawEventLine::Empty));
        assert_eq!(rem, "");
    }
}
