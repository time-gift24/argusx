// Adapted from eventsource-stream v0.2.3 (MIT OR Apache-2.0).
// Local modifications:
// - Kept parser internals private to llm-client `sse` module.

use nom::branch::alt;
use nom::bytes::streaming::{tag, take_while, take_while1, take_while_m_n};
use nom::combinator::opt;
use nom::sequence::{preceded, terminated, tuple};
use nom::IResult;

/// Raw parsed SSE line.
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
pub fn is_cr(c: char) -> bool {
    c == '\u{000D}'
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
    matches!(
        c,
        '\u{0000}'..='\u{0009}' | '\u{000B}'..='\u{000C}' | '\u{000E}'..='\u{0039}' | '\u{003B}'..='\u{10FFFF}'
    )
}

#[inline]
pub fn is_any_char(c: char) -> bool {
    matches!(
        c,
        '\u{0000}'..='\u{0009}' | '\u{000B}'..='\u{000C}' | '\u{000E}'..='\u{10FFFF}'
    )
}

#[inline]
fn crlf(input: &str) -> IResult<&str, &str> {
    tag("\u{000D}\u{000A}")(input)
}

#[inline]
fn end_of_line(input: &str) -> IResult<&str, &str> {
    alt((
        crlf,
        take_while_m_n(1, 1, is_cr),
        take_while_m_n(1, 1, is_lf),
    ))(input)
}

#[inline]
fn comment(input: &str) -> IResult<&str, RawEventLine<'_>> {
    preceded(
        take_while_m_n(1, 1, is_colon),
        terminated(take_while(is_any_char), end_of_line),
    )(input)
    .map(|(input, _)| (input, RawEventLine::Comment))
}

#[inline]
fn field(input: &str) -> IResult<&str, RawEventLine<'_>> {
    terminated(
        tuple((
            take_while1(is_name_char),
            opt(preceded(
                take_while_m_n(1, 1, is_colon),
                preceded(opt(take_while_m_n(1, 1, is_space)), take_while(is_any_char)),
            )),
        )),
        end_of_line,
    )(input)
    .map(|(input, (field, data))| (input, RawEventLine::Field(field, data)))
}

#[inline]
fn empty(input: &str) -> IResult<&str, RawEventLine<'_>> {
    end_of_line(input).map(|(i, _)| (i, RawEventLine::Empty))
}

pub fn line(input: &str) -> IResult<&str, RawEventLine<'_>> {
    alt((comment, field, empty))(input)
}
