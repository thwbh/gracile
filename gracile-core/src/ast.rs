//! Abstract syntax tree types for Gracile templates.

/// A parsed template — an ordered sequence of nodes.
#[derive(Debug, Clone)]
pub struct Template {
    pub nodes: Vec<Node>,
}

/// A single node inside a template body.
#[derive(Debug, Clone)]
pub enum Node {
    RawText(String),
    Comment(String),
    ExprTag(ExprTag),
    IfBlock(IfBlock),
    EachBlock(EachBlock),
    SnippetBlock(SnippetBlock),
    RawBlock(String),
    RenderTag(RenderTag),
    ConstTag(ConstTag),
    IncludeTag(IncludeTag),
    DebugTag(DebugTag),
}

/// `{= expr}` — escaped interpolation; `{~ expr}` — raw/unescaped interpolation.
#[derive(Debug, Clone)]
pub struct ExprTag {
    pub expr: Expr,
    /// `true` → raw output (`{~ expr}`), `false` → HTML-escaped output (`{= expr}`).
    pub raw: bool,
}

/// `{#if cond}...{:else if cond}...{:else}...{/if}`
#[derive(Debug, Clone)]
pub struct IfBlock {
    /// First entry is the `{#if}` branch; subsequent entries are `{:else if}` branches.
    pub branches: Vec<IfBranch>,
    /// Body of the `{:else}` branch, if present.
    pub else_body: Option<Vec<Node>>,
}

#[derive(Debug, Clone)]
pub struct IfBranch {
    pub condition: Expr,
    pub body: Vec<Node>,
}

/// `{#each expr as pattern, index, loop}...{:else}...{/each}`
#[derive(Debug, Clone)]
pub struct EachBlock {
    pub iterable: Expr,
    pub pattern: Pattern,
    pub index_binding: Option<String>,
    /// Optional third binding exposing loop metadata: `{ index, length, first, last }`.
    pub loop_binding: Option<String>,
    pub body: Vec<Node>,
    pub else_body: Option<Vec<Node>>,
}

/// `{#snippet name(p1, p2)}...{/snippet}`
#[derive(Debug, Clone)]
pub struct SnippetBlock {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Node>,
}

/// `{@render name(args)}`
#[derive(Debug, Clone)]
pub struct RenderTag {
    pub name: String,
    pub args: Vec<Expr>,
}

/// `{@const name = expr}`
#[derive(Debug, Clone)]
pub struct ConstTag {
    pub name: String,
    pub expr: Expr,
}

/// `{@include "path"}`
#[derive(Debug, Clone)]
pub struct IncludeTag {
    pub path: String,
}

/// `{@debug [expr]}`
#[derive(Debug, Clone)]
pub struct DebugTag {
    pub expr: Option<Expr>,
}

/// Variable binding pattern used in `{#each}`.
#[derive(Debug, Clone)]
pub enum Pattern {
    /// `{#each items as item}` — binds the whole element.
    Ident(String),
    /// `{#each items as { name, email }}` — destructures object keys.
    Destructure(Vec<String>),
}

/// A Gracile expression.
#[derive(Debug, Clone)]
pub enum Expr {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Expr>),
    Ident(String),
    /// `object.property`
    MemberAccess {
        object: Box<Expr>,
        property: String,
    },
    /// `object[index]`
    IndexAccess {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    /// `expr | filter1 | filter2(arg)`
    Filter {
        expr: Box<Expr>,
        filters: Vec<FilterApplication>,
    },
    /// `cond ? then : else`
    Ternary {
        condition: Box<Expr>,
        consequent: Box<Expr>,
        alternate: Box<Expr>,
    },
    /// Binary operators.
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Unary operators.
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    /// `expr is [not] test_name`
    Test {
        expr: Box<Expr>,
        negated: bool,
        test_name: String,
    },
    /// `expr [not] in collection`
    Membership {
        expr: Box<Expr>,
        negated: bool,
        collection: Box<Expr>,
    },
}

/// A single filter in a filter chain: `name(arg1, arg2)`.
#[derive(Debug, Clone)]
pub struct FilterApplication {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOp {
    NullCoalesce, // ??
    Or,           // ||
    And,          // &&
    Eq,           // ==
    Neq,          // !=
    Lt,           // <
    Gt,           // >
    Lte,          // <=
    Gte,          // >=
    Add,          // + (numbers → addition; strings → concatenation)
    Sub,          // -
    Mul,          // *
    Div,          // /
    Mod,          // %
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOp {
    Not, // !
    Neg, // -
}
