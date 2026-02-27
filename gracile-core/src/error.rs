//! Error types and diagnostics for lexing, parsing, and rendering.

use std::fmt;

/// Source location (1-based line and column).
#[derive(Debug, Clone)]
pub struct Span {
    pub line: u32,
    pub col: u32,
    pub offset: usize,
}

impl Span {
    pub fn new(line: u32, col: u32, offset: usize) -> Self {
        Span { line, col, offset }
    }

    pub fn unknown() -> Self {
        Span {
            line: 0,
            col: 0,
            offset: 0,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// All errors produced by Gracile.
#[derive(Debug)]
pub enum Error {
    LexError { message: String, span: Span },
    ParseError { message: String, span: Span },
    RenderError { message: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::LexError { message, span } => {
                write!(f, "Lex error at {}: {}", span, message)
            }
            Error::ParseError { message, span } => {
                write!(f, "Parse error at {}: {}", span, message)
            }
            Error::RenderError { message } => {
                write!(f, "Render error: {}", message)
            }
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
