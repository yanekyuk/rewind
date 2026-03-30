use super::{VdfDocument, VdfError, VdfMap, VdfValue};
use nom::{
    bytes::complete::take_while,
    character::complete::char,
    multi::many0,
    IResult,
};

/// Parse a VDF text string into a VdfDocument.
pub fn parse(input: &str) -> Result<VdfDocument, VdfError> {
    let input = strip_comments(input);
    match parse_document(input.trim()) {
        Ok(("", doc)) => Ok(doc),
        Ok((remaining, _)) => Err(VdfError::ParseError(format!(
            "unexpected trailing content: {:?}",
            &remaining[..remaining.len().min(50)]
        ))),
        Err(e) => Err(VdfError::ParseError(format!("parse failed: {}", e))),
    }
}

/// Strip VDF line comments (lines starting with //).
fn strip_comments(input: &str) -> String {
    input
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whitespace consumer (spaces, tabs, newlines).
fn ws(input: &str) -> IResult<&str, &str> {
    take_while(|c: char| c.is_ascii_whitespace())(input)
}

/// Parse a quoted string: "content"
fn quoted_string(input: &str) -> IResult<&str, String> {
    let (input, _) = ws(input)?;
    let (input, _) = char('"')(input)?;
    let mut result = String::new();
    let mut remaining = input;
    loop {
        if remaining.is_empty() {
            return Err(nom::Err::Failure(nom::error::Error::new(
                remaining,
                nom::error::ErrorKind::Char,
            )));
        }
        let ch = remaining.chars().next().unwrap();
        remaining = &remaining[ch.len_utf8()..];
        if ch == '"' {
            return Ok((remaining, result));
        }
        if ch == '\\' {
            if let Some(esc) = remaining.chars().next() {
                remaining = &remaining[esc.len_utf8()..];
                match esc {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    other => {
                        result.push('\\');
                        result.push(other);
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
}

/// Parse a brace-delimited map: { ... }
fn brace_map(input: &str) -> IResult<&str, VdfMap> {
    let (input, _) = ws(input)?;
    let (input, _) = char('{')(input)?;
    let (input, pairs) = many0(key_value_pair)(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('}')(input)?;
    Ok((input, pairs))
}

/// Parse a key-value pair: "key" "value" or "key" { ... }
fn key_value_pair(input: &str) -> IResult<&str, (String, VdfValue)> {
    let (input, key) = quoted_string(input)?;
    let (input, _) = ws(input)?;

    // Peek to see if next non-whitespace is '{' (nested map) or '"' (string value)
    let trimmed = input.trim_start();
    if trimmed.starts_with('{') {
        let (input, map) = brace_map(input)?;
        Ok((input, (key, VdfValue::Map(map))))
    } else {
        let (input, value) = quoted_string(input)?;
        Ok((input, (key, VdfValue::String(value))))
    }
}

/// Parse a complete VDF document: "RootKey" { ... }
fn parse_document(input: &str) -> IResult<&str, VdfDocument> {
    let (input, key) = quoted_string(input)?;
    let (input, map) = brace_map(input)?;
    let (input, _) = ws(input)?;
    Ok((input, VdfDocument { key, value: VdfValue::Map(map) }))
}
