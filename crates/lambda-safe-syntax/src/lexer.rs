use crate::SyntaxError;
use lambda_safe_model::{Diagnostic, SourceSpan, codes};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Ident(String),
    String(String),
    Number(i64),
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftAngle,
    RightAngle,
    Colon,
    Semicolon,
    Comma,
    Assign,
    Plus,
    Minus,
    Star,
    Bang,
    LessEqual,
    GreaterEqual,
    EqualEqual,
    BangEqual,
    AndAnd,
    OrOr,
    Implies,
    Eof,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub span: SourceSpan,
}

pub(crate) fn lex(source: &str) -> Result<Vec<Token>, SyntaxError> {
    const MAX_TOKENS: usize = 262_144;
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut line = 1;
    let mut column = 1;

    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b' ' | b'\t' | b'\r' => {
                index += 1;
                column += 1;
            }
            b'\n' => {
                index += 1;
                line += 1;
                column = 1;
            }
            b'#' => skip_line(bytes, &mut index, &mut column),
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                column += 2;
                skip_line(bytes, &mut index, &mut column);
            }
            b'"' => {
                let start = index;
                let start_column = column;
                index += 1;
                column += 1;
                let mut escaped = false;
                while index < bytes.len() {
                    let current = bytes[index];
                    if current == b'\n' || current == b'\r' {
                        return Err(invalid(
                            "string literals cannot span lines",
                            start,
                            index,
                            line,
                            start_column,
                        ));
                    }
                    index += 1;
                    column += 1;
                    if escaped {
                        escaped = false;
                    } else if current == b'\\' {
                        escaped = true;
                    } else if current == b'"' {
                        break;
                    }
                }
                if bytes.get(index.saturating_sub(1)) != Some(&b'"') {
                    return Err(invalid(
                        "unterminated string literal",
                        start,
                        index,
                        line,
                        start_column,
                    ));
                }
                let raw = &source[start..index];
                let value = serde_json::from_str::<String>(raw).map_err(|error| {
                    invalid(
                        format!("invalid string escape: {error}"),
                        start,
                        index,
                        line,
                        start_column,
                    )
                })?;
                tokens.push(Token {
                    kind: TokenKind::String(value),
                    span: SourceSpan {
                        start,
                        end: index,
                        line,
                        column: start_column,
                    },
                });
            }
            b'0'..=b'9' => {
                let start = index;
                let start_column = column;
                while matches!(bytes.get(index), Some(b'0'..=b'9')) {
                    index += 1;
                    column += 1;
                }
                let raw = &source[start..index];
                let value = raw.parse::<i64>().map_err(|error| {
                    invalid(
                        format!("integer literal is outside the i64 range: {error}"),
                        start,
                        index,
                        line,
                        start_column,
                    )
                })?;
                tokens.push(Token {
                    kind: TokenKind::Number(value),
                    span: SourceSpan {
                        start,
                        end: index,
                        line,
                        column: start_column,
                    },
                });
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let start = index;
                let start_column = column;
                while matches!(
                    bytes.get(index),
                    Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
                ) {
                    index += 1;
                    column += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Ident(source[start..index].to_owned()),
                    span: SourceSpan {
                        start,
                        end: index,
                        line,
                        column: start_column,
                    },
                });
            }
            _ => {
                let start = index;
                let start_column = column;
                let (kind, width) = match (byte, bytes.get(index + 1).copied()) {
                    (b'<', Some(b'=')) => (TokenKind::LessEqual, 2),
                    (b'>', Some(b'=')) => (TokenKind::GreaterEqual, 2),
                    (b'=', Some(b'=')) => (TokenKind::EqualEqual, 2),
                    (b'!', Some(b'=')) => (TokenKind::BangEqual, 2),
                    (b'&', Some(b'&')) => (TokenKind::AndAnd, 2),
                    (b'|', Some(b'|')) => (TokenKind::OrOr, 2),
                    (b'=', Some(b'>')) => (TokenKind::Implies, 2),
                    (b'{', _) => (TokenKind::LeftBrace, 1),
                    (b'}', _) => (TokenKind::RightBrace, 1),
                    (b'(', _) => (TokenKind::LeftParen, 1),
                    (b')', _) => (TokenKind::RightParen, 1),
                    (b'[', _) => (TokenKind::LeftBracket, 1),
                    (b']', _) => (TokenKind::RightBracket, 1),
                    (b'<', _) => (TokenKind::LeftAngle, 1),
                    (b'>', _) => (TokenKind::RightAngle, 1),
                    (b':', _) => (TokenKind::Colon, 1),
                    (b';', _) => (TokenKind::Semicolon, 1),
                    (b',', _) => (TokenKind::Comma, 1),
                    (b'=', _) => (TokenKind::Assign, 1),
                    (b'+', _) => (TokenKind::Plus, 1),
                    (b'-', _) => (TokenKind::Minus, 1),
                    (b'*', _) => (TokenKind::Star, 1),
                    (b'!', _) => (TokenKind::Bang, 1),
                    _ => {
                        return Err(invalid(
                            format!("unexpected character `{}`", char::from(byte)),
                            start,
                            start + 1,
                            line,
                            start_column,
                        ));
                    }
                };
                index += width;
                column += width;
                tokens.push(Token {
                    kind,
                    span: SourceSpan {
                        start,
                        end: index,
                        line,
                        column: start_column,
                    },
                });
            }
        }
        if tokens.len() > MAX_TOKENS {
            return Err(invalid(
                format!("source contains more than {MAX_TOKENS} lexical tokens"),
                index.saturating_sub(1),
                index,
                line,
                column.saturating_sub(1),
            ));
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        span: SourceSpan {
            start: source.len(),
            end: source.len(),
            line,
            column,
        },
    });
    Ok(tokens)
}

fn skip_line(bytes: &[u8], index: &mut usize, column: &mut usize) {
    while *index < bytes.len() && bytes[*index] != b'\n' {
        *index += 1;
        *column += 1;
    }
}

fn invalid(
    message: impl Into<String>,
    start: usize,
    end: usize,
    line: usize,
    column: usize,
) -> SyntaxError {
    SyntaxError::new(
        Diagnostic::error(codes::INVALID_SOURCE, message).at(SourceSpan {
            start,
            end,
            line,
            column,
        }),
    )
}

