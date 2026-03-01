//! Template renderer that evaluates the AST against a context to produce output.

use std::collections::HashMap;

use crate::ast::*;
use crate::error::{Error, Result};
use crate::value::{Value, html_escape, urlencode};

// ─── Filter / Loader ──────────────────────────────────────────────────────────

pub type FilterFn =
    Box<dyn Fn(Value, Vec<Value>) -> crate::error::Result<Value> + Send + Sync + 'static>;

/// Signature for a template loader function.
///
/// Receives a template name and returns the template source, or an error if
/// the template cannot be found.  This lets you configure an `Engine` to load
/// templates from the filesystem, a database, a map, or any other source
/// without having to pre-register every template up front.
///
/// ```rust
/// use gracile_core::{Engine, Value};
/// use std::collections::HashMap;
///
/// let engine = Engine::new()
///     .with_template_loader(|name| {
///         match name {
///             "greeting" => Ok("Hello, {= name}!".to_string()),
///             other => Err(gracile_core::Error::RenderError {
///                 message: format!("unknown template '{}'", other),
///             }),
///         }
///     });
///
/// let mut ctx = HashMap::new();
/// ctx.insert("name".to_string(), Value::from("World"));
/// let out = engine.render_name("greeting", ctx).unwrap();
/// assert_eq!(out, "Hello, World!");
/// ```
pub type LoaderFn = Box<dyn Fn(&str) -> crate::error::Result<String> + Send + Sync + 'static>;

// ─── Engine ───────────────────────────────────────────────────────────────────

/// The Gracile templating engine.
///
/// ```rust
/// use gracile_core::{Engine, Value};
/// use std::collections::HashMap;
///
/// let mut ctx = HashMap::new();
/// ctx.insert("name".to_string(), Value::from("World"));
/// let output = Engine::new().render("Hello, {= name}!", ctx).unwrap();
/// assert_eq!(output, "Hello, World!");
/// ```
/// Signature for user-supplied filter functions.
///
/// Receives the piped value and any parenthesised arguments, returns the
/// transformed value or an error.
///
/// ```rust
/// use gracile_core::{Engine, Value, FilterFn};
///
/// let engine = Engine::new()
///     .register_filter("shout", |val, _args| {
///         match val {
///             Value::String(s) => Ok(Value::String(format!("{}!!!", s.to_uppercase()))),
///             other => Ok(other),
///         }
///     });
/// ```
pub struct Engine {
    /// Raise an error on undefined variables / properties instead of returning null.
    pub strict: bool,
    /// Registered templates (name → source) available to `{@include}`.
    templates: HashMap<String, String>,
    /// User-registered filters, checked before built-ins.
    filters: HashMap<String, FilterFn>,
    /// Optional loader called when a template name is not in `templates`.
    loader: Option<LoaderFn>,
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Engine {
            strict: false,
            templates: HashMap::new(),
            filters: HashMap::new(),
            loader: None,
        }
    }

    /// Enable strict mode (undefined variables are errors, not null).
    pub fn with_strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Register a custom filter function.
    ///
    /// User filters take precedence over built-ins, so you can override them if
    /// needed. The filter receives the piped value and any parenthesised
    /// arguments evaluated to `Value`.
    pub fn register_filter<F>(mut self, name: impl Into<String>, f: F) -> Self
    where
        F: Fn(Value, Vec<Value>) -> crate::error::Result<Value> + Send + Sync + 'static,
    {
        self.filters.insert(name.into(), Box::new(f));
        self
    }

    /// Register a named template that can be referenced by `{@include "name"}`.
    pub fn register_template(mut self, name: impl Into<String>, source: impl Into<String>) -> Self {
        self.templates.insert(name.into(), source.into());
        self
    }

    /// Set a loader function that is called when a template name is not found
    /// in the pre-registered map.
    ///
    /// This lets you lazily load templates from the filesystem, a cache, or
    /// any other source without having to pre-register them all up front.
    ///
    /// ```rust
    /// use gracile_core::Engine;
    /// use std::collections::HashMap;
    ///
    /// # std::fs::write("/tmp/hello.html", "Hello, {= name}!").unwrap();
    /// let engine = Engine::new()
    ///     .with_template_loader(|name| {
    ///         std::fs::read_to_string(format!("/tmp/{}", name))
    ///             .map_err(|e| gracile_core::Error::RenderError {
    ///                 message: format!("cannot load '{}': {}", name, e),
    ///             })
    ///     });
    /// ```
    pub fn with_template_loader<F>(mut self, loader: F) -> Self
    where
        F: Fn(&str) -> crate::error::Result<String> + Send + Sync + 'static,
    {
        self.loader = Some(Box::new(loader));
        self
    }

    /// Resolve a template by name (pre-registered or via the loader) and render it.
    ///
    /// ```rust
    /// use gracile_core::{Engine, Value};
    /// use std::collections::HashMap;
    ///
    /// let engine = Engine::new()
    ///     .with_template_loader(|name| match name {
    ///         "greet" => Ok("Hello, {= who}!".to_string()),
    ///         other => Err(gracile_core::Error::RenderError {
    ///             message: format!("unknown template '{}'", other),
    ///         }),
    ///     });
    ///
    /// let mut ctx = HashMap::new();
    /// ctx.insert("who".to_string(), Value::from("World"));
    /// assert_eq!(engine.render_name("greet", ctx).unwrap(), "Hello, World!");
    /// ```
    pub fn render_name(&self, name: &str, context: HashMap<String, Value>) -> Result<String> {
        let source = self.resolve_template(name)?;
        self.render(&source, context)
    }

    /// Resolve a template name to its source string.
    pub(crate) fn resolve_template(&self, name: &str) -> Result<String> {
        if let Some(src) = self.templates.get(name) {
            return Ok(src.clone());
        }
        if let Some(ref loader) = self.loader {
            return loader(name);
        }
        Err(Error::RenderError {
            message: format!("Template '{}' not found in engine registry", name),
        })
    }

    /// Render `source` against `context` and return the produced string.
    pub fn render(&self, source: &str, context: HashMap<String, Value>) -> Result<String> {
        let tokens = crate::lexer::tokenize(source)?;
        let template = crate::parser::parse(tokens)?;
        let mut renderer = Renderer::new(self, context);
        renderer.render_template(&template)
    }

    /// Pre-compile a source string into a `Template` AST (for repeated rendering).
    pub fn compile(&self, source: &str) -> Result<Template> {
        let tokens = crate::lexer::tokenize(source)?;
        crate::parser::parse(tokens)
    }

    /// Render a pre-compiled template.
    pub fn render_template(
        &self,
        template: &Template,
        context: HashMap<String, Value>,
    ) -> Result<String> {
        let mut renderer = Renderer::new(self, context);
        renderer.render_template(template)
    }
}

// ─── Serde context helpers (feature = "serde") ───────────────────────────────

#[cfg(feature = "serde")]
fn context_from_serialize<S: serde::Serialize>(ctx: &S) -> Result<HashMap<String, Value>> {
    let json = serde_json::to_value(ctx).map_err(|e| Error::RenderError {
        message: e.to_string(),
    })?;
    match Value::from(json) {
        Value::Object(map) => Ok(map),
        other => Err(Error::RenderError {
            message: format!(
                "render context must serialise to a JSON object, got {}",
                other.type_name()
            ),
        }),
    }
}

#[cfg(feature = "serde")]
impl Engine {
    /// Like [`render`][Engine::render] but accepts any [`serde::Serialize`] value as context.
    ///
    /// This allows passing plain Rust structs annotated with `#[derive(Serialize)]`,
    /// or a [`serde_json::json!`] literal, instead of building a
    /// `HashMap<String, Value>` by hand.
    ///
    /// ```rust
    /// # use gracile_core::Engine;
    /// # use serde::Serialize;
    /// #[derive(Serialize)]
    /// struct Ctx { name: String }
    ///
    /// let out = Engine::new()
    ///     .render_from("{= name}", &Ctx { name: "World".into() })
    ///     .unwrap();
    /// assert_eq!(out, "World");
    /// ```
    pub fn render_from<S: serde::Serialize>(&self, source: &str, ctx: &S) -> Result<String> {
        self.render(source, context_from_serialize(ctx)?)
    }

    /// Like [`render_name`][Engine::render_name] but accepts any [`serde::Serialize`] as context.
    pub fn render_name_from<S: serde::Serialize>(&self, name: &str, ctx: &S) -> Result<String> {
        self.render_name(name, context_from_serialize(ctx)?)
    }

    /// Like [`render_template`][Engine::render_template] but accepts any [`serde::Serialize`] as context.
    pub fn render_template_from<S: serde::Serialize>(
        &self,
        template: &Template,
        ctx: &S,
    ) -> Result<String> {
        self.render_template(template, context_from_serialize(ctx)?)
    }
}

// ─── Renderer ─────────────────────────────────────────────────────────────────

struct Renderer<'e> {
    engine: &'e Engine,
    /// Scope stack: innermost scope is last.
    scopes: Vec<HashMap<String, Value>>,
    /// Hoisted snippet definitions (name → AST node).
    snippets: HashMap<String, SnippetBlock>,
}

impl<'e> Renderer<'e> {
    fn new(engine: &'e Engine, root_context: HashMap<String, Value>) -> Self {
        Renderer {
            engine,
            scopes: vec![root_context],
            snippets: HashMap::new(),
        }
    }

    // ── Variable lookup ───────────────────────────────────────────────────

    fn lookup(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        None
    }

    fn lookup_value(&self, name: &str) -> Result<Value> {
        match self.lookup(name) {
            Some(v) => Ok(v.clone()),
            None if self.engine.strict => Err(Error::RenderError {
                message: format!("Undefined variable '{}'", name),
            }),
            None => Ok(Value::Null),
        }
    }

    fn push_scope(&mut self, scope: HashMap<String, Value>) {
        self.scopes.push(scope);
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    // ── Template rendering ────────────────────────────────────────────────

    fn render_template(&mut self, template: &Template) -> Result<String> {
        // Hoist snippet definitions.
        self.hoist_snippets(&template.nodes);
        self.render_nodes(&template.nodes)
    }

    fn hoist_snippets(&mut self, nodes: &[Node]) {
        for node in nodes {
            if let Node::SnippetBlock(s) = node {
                self.snippets.insert(s.name.clone(), s.clone());
            }
        }
    }

    fn render_nodes(&mut self, nodes: &[Node]) -> Result<String> {
        let mut out = String::new();
        for node in nodes {
            out.push_str(&self.render_node(node)?);
        }
        Ok(out)
    }

    fn render_node(&mut self, node: &Node) -> Result<String> {
        match node {
            Node::RawText(t) => Ok(t.clone()),
            Node::Comment(_) => Ok(String::new()),
            Node::ExprTag(t) => {
                let val = self.eval_expr(&t.expr)?;
                if t.raw {
                    Ok(val.to_display_string())
                } else {
                    Ok(val.html_escaped())
                }
            }
            Node::IfBlock(b) => self.render_if(b),
            Node::EachBlock(b) => self.render_each(b),
            Node::SnippetBlock(_) => Ok(String::new()), // snippets render only when @render'd
            Node::RawBlock(content) => Ok(content.clone()),
            Node::RenderTag(t) => self.render_render_tag(t),
            Node::ConstTag(t) => {
                let val = self.eval_expr(&t.expr)?;
                // Insert into the innermost scope.
                self.scopes.last_mut().unwrap().insert(t.name.clone(), val);
                Ok(String::new())
            }
            Node::IncludeTag(t) => self.render_include(t),
            Node::DebugTag(t) => self.render_debug(t),
        }
    }

    fn render_if(&mut self, block: &IfBlock) -> Result<String> {
        for branch in &block.branches {
            let cond = self.eval_expr(&branch.condition)?;
            if cond.is_truthy() {
                return self.render_nodes(&branch.body);
            }
        }
        if let Some(else_body) = &block.else_body {
            return self.render_nodes(else_body);
        }
        Ok(String::new())
    }

    fn render_each(&mut self, block: &EachBlock) -> Result<String> {
        let iterable = self.eval_expr(&block.iterable)?;
        let items = match iterable {
            Value::Array(arr) => arr,
            Value::Null => Vec::new(),
            other => {
                return Err(Error::RenderError {
                    message: format!("{{#each}} expects an array, got {}", other.type_name()),
                });
            }
        };

        if items.is_empty() {
            if let Some(else_body) = &block.else_body {
                return self.render_nodes(else_body);
            }
            return Ok(String::new());
        }

        let len = items.len();
        let mut out = String::new();
        for (i, item) in items.iter().enumerate() {
            let mut scope = HashMap::new();
            // Bind the loop variable(s).
            match &block.pattern {
                Pattern::Ident(name) => {
                    scope.insert(name.clone(), item.clone());
                }
                Pattern::Destructure(keys) => {
                    if let Value::Object(map) = item {
                        for key in keys {
                            let val = map.get(key).cloned().unwrap_or(Value::Null);
                            scope.insert(key.clone(), val);
                        }
                    } else {
                        return Err(Error::RenderError {
                            message: format!(
                                "Destructuring pattern requires an object, got {}",
                                item.type_name()
                            ),
                        });
                    }
                }
            }
            if let Some(idx_name) = &block.index_binding {
                scope.insert(idx_name.clone(), Value::Int(i as i64));
            }
            if let Some(loop_name) = &block.loop_binding {
                let mut meta = HashMap::new();
                meta.insert("index".to_string(), Value::Int(i as i64));
                meta.insert("length".to_string(), Value::Int(len as i64));
                meta.insert("first".to_string(), Value::Bool(i == 0));
                meta.insert("last".to_string(), Value::Bool(i == len - 1));
                scope.insert(loop_name.clone(), Value::Object(meta));
            }
            self.push_scope(scope);
            out.push_str(&self.render_nodes(&block.body)?);
            self.pop_scope();
        }
        Ok(out)
    }

    fn render_render_tag(&mut self, tag: &RenderTag) -> Result<String> {
        let snippet = self
            .snippets
            .get(&tag.name)
            .cloned()
            .ok_or_else(|| Error::RenderError {
                message: format!("Unknown snippet '{}'", tag.name),
            })?;
        if snippet.params.len() != tag.args.len() {
            return Err(Error::RenderError {
                message: format!(
                    "Snippet '{}' expects {} argument(s), got {}",
                    tag.name,
                    snippet.params.len(),
                    tag.args.len()
                ),
            });
        }
        // Evaluate arguments in the current scope before entering the snippet scope.
        let mut arg_values = Vec::with_capacity(tag.args.len());
        for arg in &tag.args {
            arg_values.push(self.eval_expr(arg)?);
        }
        let mut scope = HashMap::new();
        for (name, val) in snippet.params.iter().zip(arg_values) {
            scope.insert(name.clone(), val);
        }
        self.push_scope(scope);
        let result = self.render_nodes(&snippet.body.clone());
        self.pop_scope();
        result
    }

    fn render_include(&mut self, tag: &IncludeTag) -> Result<String> {
        let source = self.engine.resolve_template(&tag.path)?;
        // Collect the current top-scope context snapshot for the included template.
        let ctx: HashMap<String, Value> = self
            .scopes
            .iter()
            .flat_map(|s| s.iter().map(|(k, v)| (k.clone(), v.clone())))
            .collect();
        self.engine.render(&source, ctx)
    }

    fn render_debug(&self, tag: &DebugTag) -> Result<String> {
        // In a library context we emit nothing; the intent is dev tooling.
        // A host application can replace this behaviour by post-processing the AST.
        let _ = tag;
        Ok(String::new())
    }

    // ── Expression evaluation ─────────────────────────────────────────────

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::Null => Ok(Value::Null),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Int(i) => Ok(Value::Int(*i)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Array(elements) => {
                let mut arr = Vec::with_capacity(elements.len());
                for e in elements {
                    arr.push(self.eval_expr(e)?);
                }
                Ok(Value::Array(arr))
            }
            Expr::Ident(name) => self.lookup_value(name),
            Expr::MemberAccess { object, property } => {
                let obj = self.eval_expr(object)?;
                self.get_property(&obj, property)
            }
            Expr::IndexAccess { object, index } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;
                self.get_index(&obj, &idx)
            }
            Expr::Filter { expr, filters } => {
                let mut val = self.eval_expr(expr)?;
                for f in filters {
                    let mut arg_vals = Vec::with_capacity(f.args.len());
                    for a in &f.args {
                        arg_vals.push(self.eval_expr(a)?);
                    }
                    val = if let Some(custom) = self.engine.filters.get(f.name.as_str()) {
                        custom(val, arg_vals)?
                    } else {
                        apply_filter(val, &f.name, arg_vals)?
                    };
                }
                Ok(val)
            }
            Expr::Ternary {
                condition,
                consequent,
                alternate,
            } => {
                let cond = self.eval_expr(condition)?;
                if cond.is_truthy() {
                    self.eval_expr(consequent)
                } else {
                    self.eval_expr(alternate)
                }
            }
            Expr::Binary { op, left, right } => self.eval_binary(op, left, right),
            Expr::Unary { op, operand } => self.eval_unary(op, operand),
            Expr::Test {
                expr,
                negated,
                test_name,
            } => {
                let val = self.eval_expr(expr)?;
                let result = eval_test(&val, test_name, self)?;
                Ok(Value::Bool(if *negated { !result } else { result }))
            }
            Expr::Membership {
                expr,
                negated,
                collection,
            } => {
                let val = self.eval_expr(expr)?;
                let coll = self.eval_expr(collection)?;
                let member = eval_membership(&val, &coll)?;
                Ok(Value::Bool(if *negated { !member } else { member }))
            }
        }
    }

    fn get_property(&self, obj: &Value, prop: &str) -> Result<Value> {
        match obj {
            Value::Object(map) => Ok(map.get(prop).cloned().unwrap_or(Value::Null)).and_then(|v| {
                if self.engine.strict
                    && !obj.is_null()
                    && let Value::Object(map) = obj
                    && !map.contains_key(prop)
                {
                    return Err(Error::RenderError {
                        message: format!("Property '{}' not found on object", prop),
                    });
                }
                Ok(v)
            }),
            Value::Null => {
                if self.engine.strict {
                    Err(Error::RenderError {
                        message: format!("Cannot access property '{}' on null", prop),
                    })
                } else {
                    Ok(Value::Null)
                }
            }
            other => {
                if self.engine.strict {
                    Err(Error::RenderError {
                        message: format!(
                            "Cannot access property '{}' on {}",
                            prop,
                            other.type_name()
                        ),
                    })
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }

    fn get_index(&self, obj: &Value, idx: &Value) -> Result<Value> {
        match obj {
            Value::Array(arr) => {
                let i = match idx {
                    Value::Int(i) => *i,
                    other => {
                        return Err(Error::RenderError {
                            message: format!(
                                "Array index must be an integer, got {}",
                                other.type_name()
                            ),
                        });
                    }
                };
                let len = arr.len() as i64;
                let i = if i < 0 { len + i } else { i };
                if i < 0 || i >= len {
                    if self.engine.strict {
                        Err(Error::RenderError {
                            message: format!("Array index {} out of bounds (len {})", i, len),
                        })
                    } else {
                        Ok(Value::Null)
                    }
                } else {
                    Ok(arr[i as usize].clone())
                }
            }
            Value::Object(map) => {
                let key = match idx {
                    Value::String(s) => s.clone(),
                    other => other.to_display_string(),
                };
                Ok(map.get(&key).cloned().unwrap_or(Value::Null))
            }
            Value::Null => {
                if self.engine.strict {
                    Err(Error::RenderError {
                        message: "Cannot index into null".to_string(),
                    })
                } else {
                    Ok(Value::Null)
                }
            }
            other => Err(Error::RenderError {
                message: format!("Cannot index into {}", other.type_name()),
            }),
        }
    }

    fn eval_binary(&mut self, op: &BinaryOp, left: &Expr, right: &Expr) -> Result<Value> {
        // Short-circuit operators evaluated first.
        match op {
            BinaryOp::Or => {
                let l = self.eval_expr(left)?;
                if l.is_truthy() {
                    return Ok(l);
                }
                return self.eval_expr(right);
            }
            BinaryOp::And => {
                let l = self.eval_expr(left)?;
                if !l.is_truthy() {
                    return Ok(l);
                }
                return self.eval_expr(right);
            }
            BinaryOp::NullCoalesce => {
                let l = self.eval_expr(left)?;
                if !l.is_null() {
                    return Ok(l);
                }
                return self.eval_expr(right);
            }
            _ => {}
        }

        let l = self.eval_expr(left)?;
        let r = self.eval_expr(right)?;

        match op {
            BinaryOp::Eq => Ok(Value::Bool(values_equal(&l, &r))),
            BinaryOp::Neq => Ok(Value::Bool(!values_equal(&l, &r))),
            BinaryOp::Lt => {
                compare_values(&l, &r).map(|o| Value::Bool(o == std::cmp::Ordering::Less))
            }
            BinaryOp::Gt => {
                compare_values(&l, &r).map(|o| Value::Bool(o == std::cmp::Ordering::Greater))
            }
            BinaryOp::Lte => {
                compare_values(&l, &r).map(|o| Value::Bool(o != std::cmp::Ordering::Greater))
            }
            BinaryOp::Gte => {
                compare_values(&l, &r).map(|o| Value::Bool(o != std::cmp::Ordering::Less))
            }
            BinaryOp::Add => match (&l, &r) {
                // At least one operand is a string → string concatenation.
                (Value::String(_), _) | (_, Value::String(_)) => Ok(Value::String(
                    l.to_display_string() + &r.to_display_string(),
                )),
                // Both are numeric → arithmetic addition.
                _ => numeric_op(&l, &r, |a, b| a + b, |a, b| a + b),
            },
            BinaryOp::Sub => numeric_op(&l, &r, |a, b| a - b, |a, b| a - b),
            BinaryOp::Mul => numeric_op(&l, &r, |a, b| a * b, |a, b| a * b),
            BinaryOp::Div => {
                let is_zero =
                    matches!(&r, Value::Int(0)) || matches!(&r, Value::Float(f) if *f == 0.0);
                if is_zero {
                    Err(Error::RenderError {
                        message: "Division by zero".to_string(),
                    })
                } else {
                    numeric_op(&l, &r, |a, b| a / b, |a, b| a / b)
                }
            }
            BinaryOp::Mod => numeric_op(&l, &r, |a, b| a % b, |a, b| a % b),
            BinaryOp::Or | BinaryOp::And | BinaryOp::NullCoalesce => unreachable!(),
        }
    }

    fn eval_unary(&mut self, op: &UnaryOp, operand: &Expr) -> Result<Value> {
        let val = self.eval_expr(operand)?;
        match op {
            UnaryOp::Not => Ok(Value::Bool(!val.is_truthy())),
            UnaryOp::Neg => match val {
                Value::Int(i) => Ok(Value::Int(-i)),
                Value::Float(f) => Ok(Value::Float(-f)),
                other => Err(Error::RenderError {
                    message: format!("Cannot negate {}", other.type_name()),
                }),
            },
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

fn eval_test(val: &Value, test_name: &str, renderer: &Renderer) -> Result<bool> {
    match test_name {
        "defined" => Ok(!matches!(val, Value::Null)),
        "undefined" => Ok(matches!(val, Value::Null)),
        "none" => Ok(matches!(val, Value::Null)),
        "odd" => match val {
            Value::Int(i) => Ok(i % 2 != 0),
            other => Err(Error::RenderError {
                message: format!("Test 'odd' requires a number, got {}", other.type_name()),
            }),
        },
        "even" => match val {
            Value::Int(i) => Ok(i % 2 == 0),
            other => Err(Error::RenderError {
                message: format!("Test 'even' requires a number, got {}", other.type_name()),
            }),
        },
        "empty" => Ok(val.is_empty()),
        "truthy" => Ok(val.is_truthy()),
        "falsy" => Ok(!val.is_truthy()),
        "string" => Ok(matches!(val, Value::String(_))),
        "number" => Ok(matches!(val, Value::Int(_) | Value::Float(_))),
        "iterable" => Ok(matches!(val, Value::Array(_))),
        unknown => {
            if renderer.engine.strict {
                Err(Error::RenderError {
                    message: format!("Unknown test '{}'", unknown),
                })
            } else {
                Ok(false)
            }
        }
    }
}

// ─── Membership ───────────────────────────────────────────────────────────────

fn eval_membership(val: &Value, collection: &Value) -> Result<bool> {
    match collection {
        Value::Array(arr) => Ok(arr.contains(val)),
        Value::Object(map) => {
            let key = val.to_display_string();
            Ok(map.contains_key(&key))
        }
        Value::String(haystack) => {
            let needle = val.to_display_string();
            Ok(haystack.contains(&needle[..]))
        }
        other => Err(Error::RenderError {
            message: format!(
                "'in' operator requires an array, object, or string, got {}",
                other.type_name()
            ),
        }),
    }
}

// ─── Comparisons ─────────────────────────────────────────────────────────────

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Int(x), Value::Float(y)) => (*x as f64) == *y,
        (Value::Float(x), Value::Int(y)) => *x == (*y as f64),
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

fn compare_values(a: &Value, b: &Value) -> Result<std::cmp::Ordering> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(x.cmp(y)),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).ok_or(Error::RenderError {
            message: "Cannot compare NaN".to_string(),
        }),
        (Value::Int(x), Value::Float(y)) => (*x as f64).partial_cmp(y).ok_or(Error::RenderError {
            message: "Cannot compare NaN".to_string(),
        }),
        (Value::Float(x), Value::Int(y)) => x.partial_cmp(&(*y as f64)).ok_or(Error::RenderError {
            message: "Cannot compare NaN".to_string(),
        }),
        (Value::String(x), Value::String(y)) => Ok(x.cmp(y)),
        _ => Err(Error::RenderError {
            message: format!("Cannot compare {} and {}", a.type_name(), b.type_name()),
        }),
    }
}

fn numeric_op(
    l: &Value,
    r: &Value,
    int_op: impl Fn(i64, i64) -> i64,
    float_op: impl Fn(f64, f64) -> f64,
) -> Result<Value> {
    match (l, r) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(*a, *b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(float_op(*a as f64, *b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(float_op(*a, *b as f64))),
        _ => Err(Error::RenderError {
            message: format!(
                "Arithmetic requires numbers, got {} and {}",
                l.type_name(),
                r.type_name()
            ),
        }),
    }
}

// ─── Built-in filters ─────────────────────────────────────────────────────────

fn apply_filter(val: Value, name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        // ── String filters ────────────────────────────────────────────────
        "upper" => {
            let s = require_string(&val, "upper")?;
            Ok(Value::String(s.to_uppercase()))
        }
        "lower" => {
            let s = require_string(&val, "lower")?;
            Ok(Value::String(s.to_lowercase()))
        }
        "capitalize" => {
            let s = require_string(&val, "capitalize")?;
            let mut chars = s.chars();
            let out = match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            };
            Ok(Value::String(out))
        }
        "trim" => {
            let s = require_string(&val, "trim")?;
            Ok(Value::String(s.trim().to_string()))
        }
        "truncate" => {
            let s = require_string(&val, "truncate")?;
            let len = require_int_arg(&args, 0, "truncate")? as usize;
            if s.chars().count() <= len {
                Ok(Value::String(s.to_string()))
            } else {
                let truncated: String = s.chars().take(len.saturating_sub(3)).collect();
                Ok(Value::String(truncated + "..."))
            }
        }
        "replace" => {
            let s = require_string(&val, "replace")?;
            let from = require_string_arg(&args, 0, "replace")?;
            let to = require_string_arg(&args, 1, "replace")?;
            Ok(Value::String(s.replace(&from[..], &to[..])))
        }
        "split" => {
            let s = require_string(&val, "split")?;
            let sep = require_string_arg(&args, 0, "split")?;
            let parts: Vec<Value> = s
                .split(&sep[..])
                .map(|p| Value::String(p.to_string()))
                .collect();
            Ok(Value::Array(parts))
        }

        // ── Collection filters ────────────────────────────────────────────
        "sort" => {
            let mut arr = require_array(val, "sort")?;
            arr.sort_by(|a, b| compare_values(a, b).unwrap_or(std::cmp::Ordering::Equal));
            Ok(Value::Array(arr))
        }
        "reverse" => match val {
            Value::Array(mut arr) => {
                arr.reverse();
                Ok(Value::Array(arr))
            }
            Value::String(s) => Ok(Value::String(s.chars().rev().collect())),
            other => Err(Error::RenderError {
                message: format!(
                    "Filter 'reverse' expects array or string, got {}",
                    other.type_name()
                ),
            }),
        },
        "join" => {
            let arr = require_array(val, "join")?;
            let sep = if args.is_empty() {
                String::new()
            } else {
                require_string_arg(&args, 0, "join")?.to_string()
            };
            let parts: Vec<String> = arr.iter().map(|v| v.to_display_string()).collect();
            Ok(Value::String(parts.join(&sep)))
        }
        "first" => {
            let arr = require_array(val, "first")?;
            Ok(arr.into_iter().next().unwrap_or(Value::Null))
        }
        "last" => {
            let arr = require_array(val, "last")?;
            Ok(arr.into_iter().next_back().unwrap_or(Value::Null))
        }
        "length" => {
            let len = val.length().ok_or_else(|| Error::RenderError {
                message: format!(
                    "Filter 'length' expects string, array, or object, got {}",
                    val.type_name()
                ),
            })?;
            Ok(Value::Int(len as i64))
        }

        // ── Formatting filters ────────────────────────────────────────────
        "default" => {
            if val.is_null() {
                Ok(args.into_iter().next().unwrap_or(Value::Null))
            } else {
                Ok(val)
            }
        }
        "json" => Ok(Value::String(val.to_json_string())),
        "round" => {
            let precision = if args.is_empty() {
                0usize
            } else {
                require_int_arg(&args, 0, "round")? as usize
            };
            match val {
                Value::Int(i) => Ok(Value::Int(i)),
                Value::Float(f) => {
                    let factor = 10f64.powi(precision as i32);
                    Ok(Value::Float((f * factor).round() / factor))
                }
                other => Err(Error::RenderError {
                    message: format!("Filter 'round' expects a number, got {}", other.type_name()),
                }),
            }
        }

        // ── Escaping filters ──────────────────────────────────────────────
        "urlencode" => {
            let s = require_string(&val, "urlencode")?;
            Ok(Value::String(urlencode(s)))
        }
        "escape" => {
            let s = val.to_display_string();
            Ok(Value::String(html_escape(&s)))
        }

        unknown => Err(Error::RenderError {
            message: format!("Unknown filter '{}'", unknown),
        }),
    }
}

// ── Filter argument helpers ───────────────────────────────────────────────────

fn require_string<'a>(val: &'a Value, filter: &str) -> Result<&'a str> {
    match val {
        Value::String(s) => Ok(s),
        other => Err(Error::RenderError {
            message: format!(
                "Filter '{}' expects a string, got {}",
                filter,
                other.type_name()
            ),
        }),
    }
}

fn require_array(val: Value, filter: &str) -> Result<Vec<Value>> {
    match val {
        Value::Array(arr) => Ok(arr),
        other => Err(Error::RenderError {
            message: format!(
                "Filter '{}' expects an array, got {}",
                filter,
                other.type_name()
            ),
        }),
    }
}

fn require_string_arg(args: &[Value], idx: usize, filter: &str) -> Result<String> {
    args.get(idx)
        .and_then(|v| {
            if let Value::String(s) = v {
                Some(s.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| Error::RenderError {
            message: format!("Filter '{}' argument {} must be a string", filter, idx + 1),
        })
}

fn require_int_arg(args: &[Value], idx: usize, filter: &str) -> Result<i64> {
    match args.get(idx) {
        Some(Value::Int(i)) => Ok(*i),
        Some(Value::Float(f)) => Ok(*f as i64),
        _ => Err(Error::RenderError {
            message: format!("Filter '{}' argument {} must be a number", filter, idx + 1),
        }),
    }
}
