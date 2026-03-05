//! Tokenizer that converts raw template source into a stream of tokens.

use crate::error::{Error, Result, Span};

/// All token variants produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// Verbatim text outside any tag.
    RawText(String),
    /// Content of a raw block (between `{#raw}` and `{/raw}`).
    RawBody(String),
    /// Content of a comment (between `{!` and `!}`).
    CommentBody(String),

    BlockOpen,    // {#
    ContinueOpen, // {:
    BlockClose,   // {/
    SpecialOpen,  // {@
    CommentOpen,  // {!
    ExprOpen,     // {=  (escaped expression interpolation)
    ExprOpenRaw,  // {~  (raw/unescaped expression interpolation)

    Close,        // }
    CommentClose, // !}

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
    KwIn,

    StringLit(String),
    IntLit(i64),
    FloatLit(f64),
    True,
    False,
    Null,

    Ident(String),

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

    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LBraceD,  // {  (destructuring open)
    RBraceD,  // }  (destructuring close)
    Comma,    // ,

    Eof,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TokenKind::RawText(_) => "raw text",
            TokenKind::RawBody(_) => "raw block content",
            TokenKind::CommentBody(_) => "comment content",

            TokenKind::BlockOpen => "'{#'",
            TokenKind::ContinueOpen => "'{:'",
            TokenKind::BlockClose => "'{/'",
            TokenKind::SpecialOpen => "'{@'",
            TokenKind::CommentOpen => "'{!'",
            TokenKind::ExprOpen => "'{='",
            TokenKind::ExprOpenRaw => "'{~'",
            TokenKind::Close => "'}'",
            TokenKind::CommentClose => "'!}'",

            TokenKind::KwIf => "keyword 'if'",
            TokenKind::KwElse => "keyword 'else'",
            TokenKind::KwEach => "keyword 'each'",
            TokenKind::KwAs => "keyword 'as'",
            TokenKind::KwSnippet => "keyword 'snippet'",
            TokenKind::KwRaw => "keyword 'raw'",
            TokenKind::KwRender => "keyword 'render'",
            TokenKind::KwConst => "keyword 'const'",
            TokenKind::KwInclude => "keyword 'include'",
            TokenKind::KwDebug => "keyword 'debug'",
            TokenKind::KwIs => "keyword 'is'",
            TokenKind::KwIn => "keyword 'in'",

            TokenKind::True => "'true'",
            TokenKind::False => "'false'",
            TokenKind::Null => "'null'",

            TokenKind::Pipe => "'|'",
            TokenKind::Or => "'||'",
            TokenKind::And => "'&&'",
            TokenKind::Question => "'?'",
            TokenKind::NullCoalesce => "'??'",
            TokenKind::Colon => "':'",
            TokenKind::Eq => "'=='",
            TokenKind::Neq => "'!='",
            TokenKind::Assign => "'='",
            TokenKind::Lt => "'<'",
            TokenKind::Gt => "'>'",
            TokenKind::Lte => "'<='",
            TokenKind::Gte => "'>='",
            TokenKind::Add => "'+'",
            TokenKind::Sub => "'-'",
            TokenKind::Mul => "'*'",
            TokenKind::Div => "'/'",
            TokenKind::Mod => "'%'",
            TokenKind::Bang => "'!'",
            TokenKind::Dot => "'.'",

            TokenKind::LParen => "'('",
            TokenKind::RParen => "')'",
            TokenKind::LBracket => "'['",
            TokenKind::RBracket => "']'",
            TokenKind::LBraceD => "'{'",
            TokenKind::RBraceD => "'}'",
            TokenKind::Comma => "','",

            TokenKind::Eof => "end of template",

            // Variants with data are handled below.
            TokenKind::StringLit(s) => return write!(f, "string '{s}'"),
            TokenKind::IntLit(n) => return write!(f, "integer '{n}'"),
            TokenKind::FloatLit(n) => return write!(f, "float '{n}'"),
            TokenKind::Ident(s) => return write!(f, "'{s}'"),
        };
        f.write_str(s)
    }
}

/// A lexed token with source position.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Character-by-character tokenizer.
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
        "in" => TokenKind::KwIn,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "null" => TokenKind::Null,
        _ => TokenKind::Ident(s),
    }
}

/// Tokenise a template source string and return the flat token stream.
pub fn tokenize(src: &str) -> Result<Vec<Token>> {
    Lexer::new(src).tokenize()
}
