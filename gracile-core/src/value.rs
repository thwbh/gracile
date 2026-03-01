//! Runtime value types used during template evaluation.

use std::collections::HashMap;
use std::fmt;

/// A runtime value in the Gracile template engine.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    /// `false`, `null`, `0`, `0.0`, and `""` are falsy. Everything else
    /// (including empty arrays `[]`) is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(_) => true,
            Value::Object(_) => true,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    /// Returns the display string for this value (used in interpolation).
    pub fn to_display_string(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 && f.abs() < 1e15 {
                    format!("{}", *f as i64)
                } else {
                    f.to_string()
                }
            }
            Value::String(s) => s.clone(),
            Value::Array(arr) => {
                let parts: Vec<String> = arr.iter().map(|v| v.to_display_string()).collect();
                parts.join(",")
            }
            Value::Object(_) => "[object Object]".to_string(),
        }
    }

    /// Serialises the value as a JSON string.
    pub fn to_json_string(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => {
                let mut out = String::with_capacity(s.len() + 2);
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        c => out.push(c),
                    }
                }
                out.push('"');
                out
            }
            Value::Array(arr) => {
                let parts: Vec<String> = arr.iter().map(|v| v.to_json_string()).collect();
                format!("[{}]", parts.join(","))
            }
            Value::Object(obj) => {
                let mut pairs: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", json_escape_str(k), v.to_json_string()))
                    .collect();
                pairs.sort(); // deterministic output
                format!("{{{}}}", pairs.join(","))
            }
        }
    }

    /// Returns the HTML-escaped display string (safe for insertion into HTML).
    pub fn html_escaped(&self) -> String {
        html_escape(&self.to_display_string())
    }

    pub fn length(&self) -> Option<usize> {
        match self {
            Value::String(s) => Some(s.chars().count()),
            Value::Array(a) => Some(a.len()),
            Value::Object(o) => Some(o.len()),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Value::String(s) => s.is_empty(),
            Value::Array(a) => a.is_empty(),
            Value::Object(o) => o.is_empty(),
            _ => false,
        }
    }
}

/// HTML-escapes `&`, `<`, `>`, `"`, `'`.
pub fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            c => out.push(c),
        }
    }
    out
}

/// URL percent-encodes a string (unreserved characters are left as-is).
pub fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn json_escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}
impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}
impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}
impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}
impl From<HashMap<String, Value>> for Value {
    fn from(m: HashMap<String, Value>) -> Self {
        Value::Object(m)
    }
}

// ── Serde integration (feature = "serde") ────────────────────────────────────

#[cfg(feature = "serde")]
impl Value {
    /// Convert any [`serde::Serialize`] value into a gracile [`Value`].
    ///
    /// This is the bridge between Rust's type system and the template engine.
    /// It goes through [`serde_json`] as an intermediate, so any type that
    /// serialises to a JSON object can be used as a render context.
    pub fn from_serialize<T: serde::Serialize>(val: &T) -> Self {
        serde_json::to_value(val)
            .map(Into::into)
            .unwrap_or(Value::Null)
    }
}

#[cfg(feature = "serde")]
impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else {
                    Value::Float(n.as_f64().unwrap_or(f64::NAN))
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(a) => Value::Array(a.into_iter().map(Into::into).collect()),
            serde_json::Value::Object(o) => {
                Value::Object(o.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}
