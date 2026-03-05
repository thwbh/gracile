//! Hand-written recursive descent parser that builds the AST from tokens.

use crate::ast::*;
use crate::error::{Error, Result, Span};
use crate::lexer::{Token, TokenKind};

/// Hand-written recursive descent parser state.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse(mut self) -> Result<Template> {
        let nodes = self.parse_nodes()?;
        self.expect_eof()?;
        Ok(Template { nodes })
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_span(&self) -> Span {
        self.peek().span.clone()
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect_close(&mut self) -> Result<()> {
        match self.peek_kind() {
            TokenKind::Close => {
                self.advance();
                Ok(())
            }
            other => Err(Error::ParseError {
                message: format!("Expected '}}', got {}", other),
                span: self.peek_span(),
            }),
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        match self.peek_kind().clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            other => Err(Error::ParseError {
                message: format!("Expected identifier, got {}", other),
                span: self.peek_span(),
            }),
        }
    }

    fn expect_string(&mut self) -> Result<String> {
        match self.peek_kind().clone() {
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(Error::ParseError {
                message: format!("Expected string literal, got {}", other),
                span: self.peek_span(),
            }),
        }
    }

    fn expect_keyword(&mut self, expected: TokenKind) -> Result<()> {
        if *self.peek_kind() == expected {
            self.advance();
            Ok(())
        } else {
            Err(Error::ParseError {
                message: format!("Expected {}, got {}", expected, self.peek_kind()),
                span: self.peek_span(),
            })
        }
    }

    fn expect_eof(&mut self) -> Result<()> {
        match self.peek_kind() {
            TokenKind::Eof => Ok(()),
            other => Err(Error::ParseError {
                message: format!("Expected end of template, got {}", other),
                span: self.peek_span(),
            }),
        }
    }

    /// Parse nodes until a stop token (`ContinueOpen`, `BlockClose`, or `Eof`),
    /// then apply standalone-tag stripping.
    fn parse_nodes(&mut self) -> Result<Vec<Node>> {
        let mut nodes: Vec<Node> = Vec::new();

        loop {
            match self.peek_kind() {
                TokenKind::Eof | TokenKind::ContinueOpen | TokenKind::BlockClose => break,

                TokenKind::RawText(_) => {
                    let TokenKind::RawText(text) = self.advance().kind.clone() else {
                        unreachable!()
                    };
                    nodes.push(Node::RawText(text));
                }

                TokenKind::CommentOpen => {
                    self.advance(); // CommentOpen
                    let body = match self.peek_kind().clone() {
                        TokenKind::CommentBody(b) => {
                            self.advance();
                            b
                        }
                        _ => String::new(),
                    };
                    match self.peek_kind() {
                        TokenKind::CommentClose => {
                            self.advance();
                        }
                        _ => {
                            return Err(Error::ParseError {
                                message: "Expected comment close '!}'".to_string(),
                                span: self.peek_span(),
                            });
                        }
                    }
                    nodes.push(Node::Comment(body));
                }

                TokenKind::ExprOpen => {
                    self.advance(); // ExprOpen
                    let expr = self.parse_expr()?;
                    self.expect_close()?;
                    nodes.push(Node::ExprTag(ExprTag { expr, raw: false }));
                }

                TokenKind::ExprOpenRaw => {
                    self.advance(); // ExprOpenRaw
                    let expr = self.parse_expr()?;
                    self.expect_close()?;
                    nodes.push(Node::ExprTag(ExprTag { expr, raw: true }));
                }

                TokenKind::BlockOpen => {
                    let node = self.parse_block()?;
                    nodes.push(node);
                }

                TokenKind::SpecialOpen => {
                    let node = self.parse_special()?;
                    nodes.push(node);
                }

                other => {
                    return Err(Error::ParseError {
                        message: format!("Unexpected token {}", other),
                        span: self.peek_span(),
                    });
                }
            }
        }

        strip_standalone(&mut nodes);
        Ok(nodes)
    }

    fn parse_block(&mut self) -> Result<Node> {
        self.advance(); // BlockOpen

        match self.peek_kind().clone() {
            TokenKind::KwIf => {
                self.advance(); // `if`
                let cond = self.parse_expr()?;
                self.expect_close()?;
                self.parse_if_block(cond)
            }
            TokenKind::KwEach => {
                self.advance(); // `each`
                let iterable = self.parse_expr()?;
                self.expect_keyword(TokenKind::KwAs)?;
                let pattern = self.parse_pattern()?;
                let index_binding = if self.peek_kind() == &TokenKind::Comma {
                    self.advance(); // `,`
                    Some(self.expect_ident()?)
                } else {
                    None
                };
                let loop_binding = if self.peek_kind() == &TokenKind::Comma {
                    self.advance(); // `,`
                    Some(self.expect_ident()?)
                } else {
                    None
                };
                self.expect_close()?;
                self.parse_each_block(iterable, pattern, index_binding, loop_binding)
            }
            TokenKind::KwSnippet => {
                self.advance(); // `snippet`
                let name = self.expect_ident()?;
                self.expect(&TokenKind::LParen)?;
                let params = self.parse_param_list()?;
                self.expect(&TokenKind::RParen)?;
                self.expect_close()?;
                self.parse_snippet_block(name, params)
            }
            TokenKind::KwRaw => {
                self.advance(); // `raw`
                self.expect_close()?;
                let body = match self.peek_kind().clone() {
                    TokenKind::RawBody(b) => {
                        self.advance();
                        b
                    }
                    _ => String::new(),
                };
                self.expect(&TokenKind::BlockClose)?;
                self.expect_keyword(TokenKind::KwRaw)?;
                self.expect_close()?;
                Ok(Node::RawBlock(body))
            }
            other => Err(Error::ParseError {
                message: format!("Unknown block keyword {}", other),
                span: self.peek_span(),
            }),
        }
    }

    fn parse_if_block(&mut self, first_cond: Expr) -> Result<Node> {
        let first_body = self.parse_nodes()?;
        let mut branches = vec![IfBranch {
            condition: first_cond,
            body: first_body,
        }];
        let mut else_body: Option<Vec<Node>> = None;

        loop {
            match self.peek_kind() {
                TokenKind::ContinueOpen => {
                    self.advance(); // ContinueOpen
                    self.expect_keyword(TokenKind::KwElse)?;
                    if self.peek_kind() == &TokenKind::KwIf {
                        self.advance(); // `if`
                        let cond = self.parse_expr()?;
                        self.expect_close()?;
                        let body = self.parse_nodes()?;
                        branches.push(IfBranch {
                            condition: cond,
                            body,
                        });
                    } else {
                        self.expect_close()?;
                        else_body = Some(self.parse_nodes()?);
                    }
                }
                TokenKind::BlockClose => {
                    self.advance(); // BlockClose
                    self.expect_keyword(TokenKind::KwIf)?;
                    self.expect_close()?;
                    break;
                }
                other => {
                    return Err(Error::ParseError {
                        message: format!(
                            "Expected {{:else}}, {{:else if}}, or {{/if}}, got {}",
                            other
                        ),
                        span: self.peek_span(),
                    });
                }
            }
        }

        Ok(Node::IfBlock(IfBlock {
            branches,
            else_body,
        }))
    }

    fn parse_each_block(
        &mut self,
        iterable: Expr,
        pattern: Pattern,
        index_binding: Option<String>,
        loop_binding: Option<String>,
    ) -> Result<Node> {
        let body = self.parse_nodes()?;
        let mut else_body: Option<Vec<Node>> = None;

        loop {
            match self.peek_kind() {
                TokenKind::ContinueOpen => {
                    self.advance(); // ContinueOpen
                    self.expect_keyword(TokenKind::KwElse)?;
                    self.expect_close()?;
                    else_body = Some(self.parse_nodes()?);
                }
                TokenKind::BlockClose => {
                    self.advance(); // BlockClose
                    self.expect_keyword(TokenKind::KwEach)?;
                    self.expect_close()?;
                    break;
                }
                other => {
                    return Err(Error::ParseError {
                        message: format!("Expected {{:else}} or {{/each}}, got {}", other),
                        span: self.peek_span(),
                    });
                }
            }
        }

        Ok(Node::EachBlock(EachBlock {
            iterable,
            pattern,
            index_binding,
            loop_binding,
            body,
            else_body,
        }))
    }

    fn parse_snippet_block(&mut self, name: String, params: Vec<String>) -> Result<Node> {
        let body = self.parse_nodes()?;
        match self.peek_kind() {
            TokenKind::BlockClose => {
                self.advance(); // BlockClose
                self.expect_keyword(TokenKind::KwSnippet)?;
                self.expect_close()?;
            }
            other => {
                return Err(Error::ParseError {
                    message: format!("Expected {{/snippet}}, got {}", other),
                    span: self.peek_span(),
                });
            }
        }
        Ok(Node::SnippetBlock(SnippetBlock { name, params, body }))
    }

    fn parse_special(&mut self) -> Result<Node> {
        self.advance(); // SpecialOpen

        match self.peek_kind().clone() {
            TokenKind::KwRender => {
                self.advance(); // `render`
                let name = self.expect_ident()?;
                self.expect(&TokenKind::LParen)?;
                let args = self.parse_arg_list()?;
                self.expect(&TokenKind::RParen)?;
                self.expect_close()?;
                Ok(Node::RenderTag(RenderTag { name, args }))
            }
            TokenKind::KwConst => {
                self.advance(); // `const`
                let name = self.expect_ident()?;
                self.expect(&TokenKind::Assign)?;
                let expr = self.parse_expr()?;
                self.expect_close()?;
                Ok(Node::ConstTag(ConstTag { name, expr }))
            }
            TokenKind::KwInclude => {
                self.advance(); // `include`
                let path = self.expect_string()?;
                self.expect_close()?;
                Ok(Node::IncludeTag(IncludeTag { path }))
            }
            TokenKind::KwDebug => {
                self.advance(); // `debug`
                let expr = if self.peek_kind() != &TokenKind::Close {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect_close()?;
                Ok(Node::DebugTag(DebugTag { expr }))
            }
            other => Err(Error::ParseError {
                message: format!("Unknown special tag keyword {}", other),
                span: self.peek_span(),
            }),
        }
    }

    fn parse_pattern(&mut self) -> Result<Pattern> {
        if self.peek_kind() == &TokenKind::LBraceD {
            self.advance(); // `{`
            let mut names = Vec::new();
            loop {
                if self.peek_kind() == &TokenKind::RBraceD {
                    self.advance(); // `}`
                    break;
                }
                names.push(self.expect_ident()?);
                if self.peek_kind() == &TokenKind::Comma {
                    self.advance();
                    if self.peek_kind() == &TokenKind::RBraceD {
                        self.advance();
                        break;
                    }
                }
            }
            Ok(Pattern::Destructure(names))
        } else {
            Ok(Pattern::Ident(self.expect_ident()?))
        }
    }

    fn parse_param_list(&mut self) -> Result<Vec<String>> {
        let mut params = Vec::new();
        if self.peek_kind() == &TokenKind::RParen {
            return Ok(params);
        }
        params.push(self.expect_ident()?);
        while self.peek_kind() == &TokenKind::Comma {
            self.advance();
            if self.peek_kind() == &TokenKind::RParen {
                break;
            }
            params.push(self.expect_ident()?);
        }
        Ok(params)
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        if self.peek_kind() == &TokenKind::RParen {
            return Ok(args);
        }
        args.push(self.parse_expr()?);
        while self.peek_kind() == &TokenKind::Comma {
            self.advance();
            if self.peek_kind() == &TokenKind::RParen {
                break;
            }
            args.push(self.parse_expr()?);
        }
        Ok(args)
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_filter_expr()
    }

    fn parse_filter_expr(&mut self) -> Result<Expr> {
        let expr = self.parse_ternary()?;
        let mut filters = Vec::new();
        while self.peek_kind() == &TokenKind::Pipe {
            self.advance();
            let name = self.expect_ident()?;
            let args = if self.peek_kind() == &TokenKind::LParen {
                self.advance();
                let a = self.parse_arg_list()?;
                self.expect(&TokenKind::RParen)?;
                a
            } else {
                Vec::new()
            };
            filters.push(FilterApplication { name, args });
        }
        if filters.is_empty() {
            Ok(expr)
        } else {
            Ok(Expr::Filter {
                expr: Box::new(expr),
                filters,
            })
        }
    }

    fn parse_ternary(&mut self) -> Result<Expr> {
        let cond = self.parse_nullish()?;
        if self.peek_kind() == &TokenKind::Question {
            self.advance();
            let consequent = self.parse_expr()?;
            self.expect(&TokenKind::Colon)?;
            let alternate = self.parse_expr()?;
            Ok(Expr::Ternary {
                condition: Box::new(cond),
                consequent: Box::new(consequent),
                alternate: Box::new(alternate),
            })
        } else {
            Ok(cond)
        }
    }

    fn parse_nullish(&mut self) -> Result<Expr> {
        let mut left = self.parse_or()?;
        while self.peek_kind() == &TokenKind::NullCoalesce {
            self.advance();
            let right = self.parse_or()?;
            left = Expr::Binary {
                op: BinaryOp::NullCoalesce,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;
        while self.peek_kind() == &TokenKind::Or {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_equality()?;
        while self.peek_kind() == &TokenKind::And {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Eq => BinaryOp::Eq,
                TokenKind::Neq => BinaryOp::Neq,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut left = self.parse_test()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Lte => BinaryOp::Lte,
                TokenKind::Gte => BinaryOp::Gte,
                _ => break,
            };
            self.advance();
            let right = self.parse_test()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_test(&mut self) -> Result<Expr> {
        let expr = self.parse_membership()?;
        if self.peek_kind() == &TokenKind::KwIs {
            self.advance();
            let negated = if self.peek_kind() == &TokenKind::KwNot {
                self.advance();
                true
            } else {
                false
            };
            let test_name = self.expect_ident()?;
            Ok(Expr::Test {
                expr: Box::new(expr),
                negated,
                test_name,
            })
        } else {
            Ok(expr)
        }
    }

    fn parse_membership(&mut self) -> Result<Expr> {
        let expr = self.parse_additive()?;
        match self.peek_kind() {
            TokenKind::KwIn => {
                self.advance();
                let collection = self.parse_additive()?;
                Ok(Expr::Membership {
                    expr: Box::new(expr),
                    negated: false,
                    collection: Box::new(collection),
                })
            }
            TokenKind::KwNot => {
                self.advance();
                self.expect_keyword(TokenKind::KwIn)?;
                let collection = self.parse_additive()?;
                Ok(Expr::Membership {
                    expr: Box::new(expr),
                    negated: true,
                    collection: Box::new(collection),
                })
            }
            _ => Ok(expr),
        }
    }

    fn parse_additive(&mut self) -> Result<Expr> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Add => BinaryOp::Add,
                TokenKind::Sub => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Mul => BinaryOp::Mul,
                TokenKind::Div => BinaryOp::Div,
                TokenKind::Mod => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        match self.peek_kind() {
            TokenKind::Bang => {
                self.advance();
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(self.parse_unary()?),
                })
            }
            TokenKind::Sub => {
                self.advance();
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_kind() {
                TokenKind::Dot => {
                    self.advance();
                    let prop = self.expect_ident()?;
                    expr = Expr::MemberAccess {
                        object: Box::new(expr),
                        property: prop,
                    };
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&TokenKind::RBracket)?;
                    expr = Expr::IndexAccess {
                        object: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        match self.peek_kind().clone() {
            TokenKind::Null => {
                self.advance();
                Ok(Expr::Null)
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::IntLit(i) => {
                self.advance();
                Ok(Expr::Int(i))
            }
            TokenKind::FloatLit(f) => {
                self.advance();
                Ok(Expr::Float(f))
            }
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(Expr::String(s))
            }
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Expr::Ident(name))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                if self.peek_kind() != &TokenKind::RBracket {
                    elements.push(self.parse_expr()?);
                    while self.peek_kind() == &TokenKind::Comma {
                        self.advance();
                        if self.peek_kind() == &TokenKind::RBracket {
                            break;
                        }
                        elements.push(self.parse_expr()?);
                    }
                }
                self.expect(&TokenKind::RBracket)?;
                Ok(Expr::Array(elements))
            }
            other => Err(Error::ParseError {
                message: format!("Expected expression, got {}", other),
                span: self.peek_span(),
            }),
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&Token> {
        if std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind) {
            Ok(self.advance())
        } else {
            Err(Error::ParseError {
                message: format!("Expected {}, got {}", kind, self.peek_kind()),
                span: self.peek_span(),
            })
        }
    }
}

/// Strip standalone block tags from their surrounding lines.
///
/// A block-level tag is "standalone" when:
/// - the text before it on its line is entirely whitespace (spaces/tabs), and
/// - the text after it on its line is entirely whitespace (spaces/tabs) + newline.
///
/// When standalone, the tag's entire line is removed from the output:
/// - the whitespace prefix is stripped from the preceding raw-text node,
/// - the trailing newline is stripped from the following raw-text node,
/// - the opening newline is stripped from the block's first body raw-text node,
/// - the closing whitespace prefix is stripped from the block's last body raw-text node.
fn strip_standalone(nodes: &mut Vec<Node>) {
    let len = nodes.len();
    for i in 0..len {
        if !is_standalone_eligible(&nodes[i]) {
            continue;
        }

        // For block nodes that have Vec<Node> bodies (if/each/snippet): require that
        // the first body node is RawText starting with '\n'.  This confirms the opener
        // tag was immediately followed by a newline and therefore IS on its own line.
        // Without this check, inline blocks like `{#if cond}text{/if}` would wrongly
        // be treated as standalone and their bodies would be stripped.
        if has_vec_body(&nodes[i]) {
            match block_first_body_text(&nodes[i]) {
                Some(s) if s.starts_with('\n') => {} // opener was on its own line — OK
                _ => continue,                       // inline or empty body — skip
            }
        }

        // Check: is text after the last newline in the preceding node blank?
        let prefix_blank = match i.checked_sub(1) {
            None => true, // start of body — treat as after a newline
            Some(prev) => match &nodes[prev] {
                Node::RawText(s) => after_last_nl(s).chars().all(|c| c == ' ' || c == '\t'),
                _ => false,
            },
        };
        if !prefix_blank {
            continue;
        }

        // Check: is text before the first newline in the following node blank?
        let suffix_blank = match nodes.get(i + 1) {
            None => true, // end of body — treat as before a newline
            Some(Node::RawText(s)) => before_first_nl(s).chars().all(|c| c == ' ' || c == '\t'),
            Some(_) => false,
        };
        if !suffix_blank {
            continue;
        }

        // Strip the blank prefix from the preceding raw-text node.
        if i > 0
            && let Node::RawText(s) = &mut nodes[i - 1]
        {
            trim_line_tail(s);
        }

        // Strip the rest of the tag's line (blank spaces + newline) from the
        // start of the following raw-text node.
        if i + 1 < len
            && let Node::RawText(s) = &mut nodes[i + 1]
        {
            strip_through_first_nl(s);
        }

        // Strip the opening/closing newlines inside block bodies.
        strip_block_body_edges(&mut nodes[i]);
    }

    // Remove raw-text nodes that became empty after stripping.
    nodes.retain(|n| !matches!(n, Node::RawText(s) if s.is_empty()));
}

/// Whether a node triggers standalone-line stripping.
fn is_standalone_eligible(node: &Node) -> bool {
    matches!(
        node,
        Node::IfBlock(_)
            | Node::EachBlock(_)
            | Node::SnippetBlock(_)
            | Node::RawBlock(_)
            | Node::RenderTag(_)
            | Node::ConstTag(_)
            | Node::IncludeTag(_)
            | Node::DebugTag(_)
            | Node::Comment(_)
    )
}

/// Whether a node has a `Vec<Node>` body (as opposed to no body or a String body).
fn has_vec_body(node: &Node) -> bool {
    matches!(
        node,
        Node::IfBlock(_) | Node::EachBlock(_) | Node::SnippetBlock(_)
    )
}

/// Return the text of the first body node if it is `RawText`, otherwise `None`.
fn block_first_body_text(node: &Node) -> Option<&str> {
    let body = match node {
        Node::IfBlock(b) => &b.branches[0].body,
        Node::EachBlock(b) => &b.body,
        Node::SnippetBlock(b) => &b.body,
        _ => return None,
    };
    match body.first() {
        Some(Node::RawText(s)) => Some(s.as_str()),
        _ => None,
    }
}

/// Strip the opening newline (from the opener tag's line) from the first body
/// node, and the closing whitespace prefix (from the closer tag's line) from
/// the last body node.
fn strip_block_body_edges(node: &mut Node) {
    match node {
        Node::IfBlock(b) => {
            // Every branch body is bounded by standalone tags on both sides
            // ({#if}, {:else if}, {:else}, {/if}).  Strip the head and tail of
            // each branch so all those lines disappear from the output.
            for branch in &mut b.branches {
                strip_body_head(&mut branch.body);
                strip_body_tail(&mut branch.body);
            }
            if let Some(eb) = &mut b.else_body {
                strip_body_head(eb);
                strip_body_tail(eb);
            }
        }
        Node::EachBlock(b) => {
            strip_body_head(&mut b.body);
            if let Some(eb) = &mut b.else_body {
                strip_body_tail(&mut b.body); // blank line before {:else}
                strip_body_head(eb); // newline after {:else}
                strip_body_tail(eb); // blank line before {/each}
            } else {
                strip_body_tail(&mut b.body);
            }
        }
        Node::SnippetBlock(b) => {
            strip_body_head(&mut b.body);
            strip_body_tail(&mut b.body);
        }
        _ => {}
    }
}

fn strip_body_head(body: &mut Vec<Node>) {
    if let Some(Node::RawText(s)) = body.first_mut() {
        // Only strip when the first "line" of the body (before the first \n) is
        // entirely blank — meaning the opener tag occupied its own line.
        let should_strip = match s.find('\n') {
            Some(pos) => s[..pos].chars().all(|c| c == ' ' || c == '\t'),
            None => false, // body is on the same line as the opener — don't touch
        };
        if should_strip {
            strip_through_first_nl(s);
            if s.is_empty() {
                body.remove(0);
            }
        }
    }
}

fn strip_body_tail(body: &mut Vec<Node>) {
    if let Some(Node::RawText(s)) = body.last_mut() {
        // Only strip when the last "line" of the body (after the last \n) is
        // entirely blank — meaning the closer tag occupied its own line.
        let blank_tail = match s.rfind('\n') {
            Some(pos) => s[pos + 1..].chars().all(|c| c == ' ' || c == '\t'),
            None => false, // body is on the same line as the closer — don't touch
        };
        if blank_tail {
            trim_line_tail(s);
            if s.is_empty() {
                body.pop();
            }
        }
    }
}

/// Returns the substring after the last `\n` (or the whole string if none).
fn after_last_nl(s: &str) -> &str {
    match s.rfind('\n') {
        Some(pos) => &s[pos + 1..],
        None => s,
    }
}

/// Returns the substring before the first `\n` (or the whole string if none).
fn before_first_nl(s: &str) -> &str {
    match s.find('\n') {
        Some(pos) => &s[..pos],
        None => s,
    }
}

/// Strip everything after the last `\n`, keeping the `\n` itself.
/// If there is no `\n`, clear the whole string (it was a blank opening prefix).
fn trim_line_tail(s: &mut String) {
    match s.rfind('\n') {
        Some(pos) => s.truncate(pos + 1),
        None => s.clear(),
    }
}

/// Strip everything up to and including the first `\n` (handling `\r\n` too).
/// If there is no `\n`, clear the whole string (everything is on the tag's line).
fn strip_through_first_nl(s: &mut String) {
    if let Some(pos) = s.find('\n') {
        *s = s[pos + 1..].to_string();
    } else {
        s.clear();
    }
}

/// Parse a token stream into a [`Template`] AST.
pub fn parse(tokens: Vec<Token>) -> Result<Template> {
    Parser::new(tokens).parse()
}
