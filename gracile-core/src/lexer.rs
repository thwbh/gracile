//! Tokenizer that converts raw template source into a stream of tokens.

use crate::error::{Error, Result, Span};

// ─── Token types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ── Template level ────────────────────────────────────────────────────
    /// Verbatim text outside any tag.
    RawText(String),
    /// Content of a raw block (between `{#raw}` and `{/raw}`).
    RawBody(String),
    /// Content of a comment (between `{!` and `!}`).
    CommentBody(String),

    // ── Tag openers ───────────────────────────────────────────────────────
    BlockOpen,    // {#
    ContinueOpen, // {:
    BlockClose,   // {/
    SpecialOpen,  // {@
    CommentOpen,  // {!
    ExprOpen,     // {=  (escaped expression interpolation)
    ExprOpenRaw,  // {~  (raw/unescaped expression interpolation)

    // ── Tag closer ────────────────────────────────────────────────────────
    Close,        // }
    CommentClose, // !}

    // ── Keywords (context-sensitive, inside tags) ─────────────────────────
    KwIf,
    KwElse,
    KwEach,
    KwAs,
    KwSnippet,
    KwRaw,
    KwRender,
    KwConst,
    KwInclude,
    KwDebug,
    KwIs,
    KwNot,
    KwIn,

    // ── Literals ──────────────────────────────────────────────────────────
    StringLit(String),
    IntLit(i64),
    FloatLit(f64),
    True,
    False,
    Null,

    // ── Identifier ────────────────────────────────────────────────────────
    Ident(String),

    // ── Operators ─────────────────────────────────────────────────────────
    Pipe,         // |
    Or,           // ||
    And,          // &&
    Question,     // ?
    NullCoalesce, // ??
    Colon,        // :
    Eq,           // ==
    Neq,          // !=
    Assign,       // =
    Lt,           // <
    Gt,           // >
    Lte,          // <=
    Gte,          // >=
    Add,          // +
    Sub,          // -
    Mul,          // *
    Div,          // /
    Mod,          // %
    Bang,         // ! (unary NOT)
    Dot,          // .

    // ── Punctuation ───────────────────────────────────────────────────────
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LBraceD,  // {  (destructuring open)
    RBraceD,  // }  (destructuring close)
    Comma,    // ,

    Eof,
}

/// A lexed token with source position.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

// ─── Lexer ───────────────────────────────────────────────────────────────────

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
    offset: usize,
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Lexer {
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            offset: 0,
        }
    }

    /// Tokenise the entire source and return the token stream.
    pub fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        self.lex_template(&mut tokens)?;
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: self.span(),
        });
        Ok(tokens)
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    fn span(&self) -> Span {
        Span::new(self.line, self.col, self.offset)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn matches_at(&self, offset: usize, s: &str) -> bool {
        s.chars()
            .enumerate()
            .all(|(i, c)| self.peek_at(offset + i) == Some(c))
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        self.offset += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn advance_if(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    // ── Template-mode lexing ─────────────────────────────────────────────

    fn lex_template(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        while !self.at_end() {
            if self.peek() != Some('{') {
                self.lex_raw_text(tokens);
                continue;
            }

            let sigil = self.peek_at(1);

            match sigil {
                Some('#') => {
                    // Detect raw block: {#raw}
                    if self.matches_at(2, "raw}") {
                        self.lex_raw_block(tokens)?;
                    } else {
                        let span = self.span();
                        self.advance(); // `{`
                        self.advance(); // `#`
                        tokens.push(mk(TokenKind::BlockOpen, span));
                        self.lex_tag(tokens)?;
                    }
                }
                Some(':') => {
                    let span = self.span();
                    self.advance(); // `{`
                    self.advance(); // `:`
                    tokens.push(mk(TokenKind::ContinueOpen, span));
                    self.lex_tag(tokens)?;
                }
                Some('/') => {
                    let span = self.span();
                    self.advance(); // `{`
                    self.advance(); // `/`
                    tokens.push(mk(TokenKind::BlockClose, span));
                    self.lex_tag(tokens)?;
                }
                Some('@') => {
                    let span = self.span();
                    self.advance(); // `{`
                    self.advance(); // `@`
                    tokens.push(mk(TokenKind::SpecialOpen, span));
                    self.lex_tag(tokens)?;
                }
                Some('!') => {
                    let span = self.span();
                    self.advance(); // `{`
                    self.advance(); // `!`
                    tokens.push(mk(TokenKind::CommentOpen, span));
                    self.lex_comment(tokens)?;
                }
                Some('=') => {
                    // `{=` — escaped expression interpolation.
                    let span = self.span();
                    self.advance(); // `{`
                    self.advance(); // `=`
                    tokens.push(mk(TokenKind::ExprOpen, span));
                    self.lex_tag(tokens)?;
                }
                Some('~') => {
                    // `{~` — raw (unescaped) expression interpolation.
                    let span = self.span();
                    self.advance(); // `{`
                    self.advance(); // `~`
                    tokens.push(mk(TokenKind::ExprOpenRaw, span));
                    self.lex_tag(tokens)?;
                }
                Some('\\') => {
                    // `{\=` → literal `{=`; `{\~` → literal `{~`.
                    // Any other `{\X` falls through to the bare-`{` branch.
                    let escaped = self.peek_at(2);
                    if matches!(escaped, Some('=') | Some('~')) {
                        let span = self.span();
                        self.advance(); // `{`
                        self.advance(); // `\`
                        let sigil = self.advance().unwrap(); // `=` or `~`
                        let mut text = format!("{{{sigil}");
                        // absorb further non-`{` characters into the same RawText token
                        while !self.at_end() && self.peek() != Some('{') {
                            text.push(self.advance().unwrap());
                        }
                        tokens.push(mk(TokenKind::RawText(text), span));
                    } else {
                        // Bare `{\` not followed by a recognised escape — literal text.
                        let span = self.span();
                        self.advance(); // consume `{`
                        let mut text = String::from("{");
                        while !self.at_end() && self.peek() != Some('{') {
                            text.push(self.advance().unwrap());
                        }
                        tokens.push(mk(TokenKind::RawText(text), span));
                    }
                }
                _ => {
                    // Bare `{` not followed by a recognised sigil — always literal text.
                    let span = self.span();
                    self.advance(); // consume `{`
                    let mut text = String::from("{");
                    while !self.at_end() && self.peek() != Some('{') {
                        text.push(self.advance().unwrap());
                    }
                    tokens.push(mk(TokenKind::RawText(text), span));
                }
            }
        }
        Ok(())
    }

    fn lex_raw_text(&mut self, tokens: &mut Vec<Token>) {
        let span = self.span();
        let mut text = String::new();
        while !self.at_end() && self.peek() != Some('{') {
            text.push(self.advance().unwrap());
        }
        if !text.is_empty() {
            tokens.push(mk(TokenKind::RawText(text), span));
        }
    }

    // ── Raw block: {#raw}...{/raw} ───────────────────────────────────────

    fn lex_raw_block(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        let open_span = self.span();
        // Consume `{#raw}`  (6 chars)
        for _ in 0..6 {
            self.advance();
        }
        tokens.push(mk(TokenKind::BlockOpen, open_span));
        tokens.push(mk(TokenKind::KwRaw, self.span()));
        tokens.push(mk(TokenKind::Close, self.span()));

        // Scan body until `{/raw}`
        let body_span = self.span();
        let mut body = String::new();
        loop {
            if self.at_end() {
                return Err(Error::LexError {
                    message: "Unclosed {#raw} block — expected {/raw}".to_string(),
                    span: self.span(),
                });
            }
            if self.matches_at(0, "{/raw}") {
                break;
            }
            body.push(self.advance().unwrap());
        }
        tokens.push(mk(TokenKind::RawBody(body), body_span));

        // Consume `{/raw}` (6 chars)
        let close_span = self.span();
        for _ in 0..6 {
            self.advance();
        }
        tokens.push(mk(TokenKind::BlockClose, close_span.clone()));
        tokens.push(mk(TokenKind::KwRaw, close_span.clone()));
        tokens.push(mk(TokenKind::Close, close_span));
        Ok(())
    }

    // ── Tag-mode lexing ──────────────────────────────────────────────────

    /// Tokenise the contents of a tag until the matching `}`.
    /// Tracks brace depth so that destructuring patterns like `{ name, age }`
    /// inside `{#each ... as { name, age }}` don't prematurely close the tag.
    fn lex_tag(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        let mut brace_depth: usize = 0;
        loop {
            self.skip_ws();
            if self.at_end() {
                return Err(Error::LexError {
                    message: "Unexpected end of input inside tag".to_string(),
                    span: self.span(),
                });
            }

            if brace_depth == 0 && self.peek() == Some('}') {
                let span = self.span();
                self.advance();
                tokens.push(mk(TokenKind::Close, span));
                return Ok(());
            }

            if self.peek() == Some('{') {
                brace_depth += 1;
                let span = self.span();
                self.advance();
                tokens.push(mk(TokenKind::LBraceD, span));
                continue;
            }

            if self.peek() == Some('}') {
                // brace_depth > 0 here
                brace_depth -= 1;
                let span = self.span();
                self.advance();
                tokens.push(mk(TokenKind::RBraceD, span));
                continue;
            }

            let tok = self.next_tag_token()?;
            tokens.push(tok);
        }
    }

    // ── Comment-mode lexing ──────────────────────────────────────────────

    fn lex_comment(&mut self, tokens: &mut Vec<Token>) -> Result<()> {
        let body_span = self.span();
        let mut body = String::new();
        loop {
            if self.at_end() {
                return Err(Error::LexError {
                    message: "Unclosed comment — expected !}".to_string(),
                    span: self.span(),
                });
            }
            if self.peek() == Some('!') && self.peek_at(1) == Some('}') {
                let close_span = self.span();
                self.advance(); // `!`
                self.advance(); // `}`
                tokens.push(mk(TokenKind::CommentBody(body), body_span));
                tokens.push(mk(TokenKind::CommentClose, close_span));
                return Ok(());
            }
            body.push(self.advance().unwrap());
        }
    }

    // ── Token-level helpers ───────────────────────────────────────────────

    fn skip_ws(&mut self) {
        while matches!(
            self.peek(),
            Some(' ') | Some('\t') | Some('\n') | Some('\r')
        ) {
            self.advance();
        }
    }

    fn next_tag_token(&mut self) -> Result<Token> {
        let span = self.span();
        let c = self.peek().unwrap();

        match c {
            '"' | '\'' => {
                let s = self.lex_string(c)?;
                Ok(mk(TokenKind::StringLit(s), span))
            }
            '0'..='9' => {
                let kind = self.lex_number()?;
                Ok(mk(kind, span))
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let name = self.lex_ident();
                let kind = keyword_or_ident(name);
                Ok(mk(kind, span))
            }
            '|' => {
                self.advance();
                let kind = if self.advance_if('|') {
                    TokenKind::Or
                } else {
                    TokenKind::Pipe
                };
                Ok(mk(kind, span))
            }
            '&' => {
                self.advance();
                if self.advance_if('&') {
                    Ok(mk(TokenKind::And, span))
                } else {
                    Err(Error::LexError {
                        message: "Expected '&&' — lone '&' is not valid".to_string(),
                        span,
                    })
                }
            }
            '?' => {
                self.advance();
                let kind = if self.advance_if('?') {
                    TokenKind::NullCoalesce
                } else {
                    TokenKind::Question
                };
                Ok(mk(kind, span))
            }
            ':' => {
                self.advance();
                Ok(mk(TokenKind::Colon, span))
            }
            '=' => {
                self.advance();
                let kind = if self.advance_if('=') {
                    TokenKind::Eq
                } else {
                    TokenKind::Assign
                };
                Ok(mk(kind, span))
            }
            '!' => {
                self.advance();
                let kind = if self.advance_if('=') {
                    TokenKind::Neq
                } else {
                    TokenKind::Bang
                };
                Ok(mk(kind, span))
            }
            '<' => {
                self.advance();
                let kind = if self.advance_if('=') {
                    TokenKind::Lte
                } else {
                    TokenKind::Lt
                };
                Ok(mk(kind, span))
            }
            '>' => {
                self.advance();
                let kind = if self.advance_if('=') {
                    TokenKind::Gte
                } else {
                    TokenKind::Gt
                };
                Ok(mk(kind, span))
            }
            '+' => {
                self.advance();
                Ok(mk(TokenKind::Add, span))
            }
            '-' => {
                self.advance();
                Ok(mk(TokenKind::Sub, span))
            }
            '*' => {
                self.advance();
                Ok(mk(TokenKind::Mul, span))
            }
            '/' => {
                self.advance();
                Ok(mk(TokenKind::Div, span))
            }
            '%' => {
                self.advance();
                Ok(mk(TokenKind::Mod, span))
            }
            '.' => {
                self.advance();
                Ok(mk(TokenKind::Dot, span))
            }
            '(' => {
                self.advance();
                Ok(mk(TokenKind::LParen, span))
            }
            ')' => {
                self.advance();
                Ok(mk(TokenKind::RParen, span))
            }
            '[' => {
                self.advance();
                Ok(mk(TokenKind::LBracket, span))
            }
            ']' => {
                self.advance();
                Ok(mk(TokenKind::RBracket, span))
            }
            ',' => {
                self.advance();
                Ok(mk(TokenKind::Comma, span))
            }
            other => Err(Error::LexError {
                message: format!("Unexpected character '{}' inside tag", other),
                span,
            }),
        }
    }

    fn lex_ident(&mut self) -> String {
        let mut s = String::new();
        while matches!(
            self.peek(),
            Some('a'..='z') | Some('A'..='Z') | Some('0'..='9') | Some('_')
        ) {
            s.push(self.advance().unwrap());
        }
        s
    }

    fn lex_string(&mut self, quote: char) -> Result<String> {
        self.advance(); // opening quote
        let mut s = String::new();
        loop {
            match self.advance() {
                None => {
                    return Err(Error::LexError {
                        message: "Unterminated string literal".to_string(),
                        span: self.span(),
                    });
                }
                Some(c) if c == quote => break,
                Some('\\') => {
                    let esc_span = self.span();
                    match self.advance() {
                        Some('"') => s.push('"'),
                        Some('\'') => s.push('\''),
                        Some('\\') => s.push('\\'),
                        Some('n') => s.push('\n'),
                        Some('r') => s.push('\r'),
                        Some('t') => s.push('\t'),
                        Some('0') => s.push('\0'),
                        Some('u') => {
                            if !self.advance_if('{') {
                                return Err(Error::LexError {
                                    message: "Expected '{' after \\u".to_string(),
                                    span: self.span(),
                                });
                            }
                            let mut hex = String::new();
                            while matches!(
                                self.peek(),
                                Some('0'..='9') | Some('a'..='f') | Some('A'..='F')
                            ) {
                                hex.push(self.advance().unwrap());
                            }
                            if !self.advance_if('}') {
                                return Err(Error::LexError {
                                    message: "Expected '}' after unicode escape".to_string(),
                                    span: self.span(),
                                });
                            }
                            let code =
                                u32::from_str_radix(&hex, 16).map_err(|_| Error::LexError {
                                    message: format!("Invalid unicode escape \\u{{{}}}", hex),
                                    span: esc_span.clone(),
                                })?;
                            s.push(char::from_u32(code).ok_or(Error::LexError {
                                message: format!("Invalid unicode codepoint U+{:04X}", code),
                                span: esc_span,
                            })?);
                        }
                        Some(c) => {
                            return Err(Error::LexError {
                                message: format!("Unknown escape sequence '\\{}'", c),
                                span: esc_span,
                            });
                        }
                        None => {
                            return Err(Error::LexError {
                                message: "Unterminated escape sequence".to_string(),
                                span: esc_span,
                            });
                        }
                    }
                }
                Some(c) => s.push(c),
            }
        }
        Ok(s)
    }

    fn lex_number(&mut self) -> Result<TokenKind> {
        let mut s = String::new();
        while matches!(self.peek(), Some('0'..='9')) {
            s.push(self.advance().unwrap());
        }
        if self.peek() == Some('.') && matches!(self.peek_at(1), Some('0'..='9')) {
            s.push(self.advance().unwrap()); // '.'
            while matches!(self.peek(), Some('0'..='9')) {
                s.push(self.advance().unwrap());
            }
            if matches!(self.peek(), Some('e') | Some('E')) {
                s.push(self.advance().unwrap());
                if matches!(self.peek(), Some('+') | Some('-')) {
                    s.push(self.advance().unwrap());
                }
                while matches!(self.peek(), Some('0'..='9')) {
                    s.push(self.advance().unwrap());
                }
            }
            let span = self.span();
            let f: f64 = s.parse().map_err(|_| Error::LexError {
                message: format!("Invalid float literal '{}'", s),
                span,
            })?;
            Ok(TokenKind::FloatLit(f))
        } else {
            let span = self.span();
            let i: i64 = s.parse().map_err(|_| Error::LexError {
                message: format!("Invalid integer literal '{}'", s),
                span,
            })?;
            Ok(TokenKind::IntLit(i))
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn mk(kind: TokenKind, span: Span) -> Token {
    Token { kind, span }
}

fn keyword_or_ident(s: String) -> TokenKind {
    match s.as_str() {
        "if" => TokenKind::KwIf,
        "else" => TokenKind::KwElse,
        "each" => TokenKind::KwEach,
        "as" => TokenKind::KwAs,
        "snippet" => TokenKind::KwSnippet,
        "raw" => TokenKind::KwRaw,
        "render" => TokenKind::KwRender,
        "const" => TokenKind::KwConst,
        "include" => TokenKind::KwInclude,
        "debug" => TokenKind::KwDebug,
        "is" => TokenKind::KwIs,
        "not" => TokenKind::KwNot,
        "in" => TokenKind::KwIn,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "null" => TokenKind::Null,
        _ => TokenKind::Ident(s),
    }
}

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn tokenize(src: &str) -> Result<Vec<Token>> {
    Lexer::new(src).tokenize()
}
