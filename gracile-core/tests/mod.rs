use gracile_core::Value;
use std::collections::HashMap;

mod fixtures;

fn ctx(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn obj(pairs: &[(&str, Value)]) -> Value {
    Value::Object(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    )
}

fn arr(values: Vec<Value>) -> Value {
    Value::Array(values)
}

/// Convert a `serde_json::Value` object into a `HashMap<String, Value>`.
/// Used by tests that prefer the `json!` literal syntax over hand-built contexts.
fn json_ctx(v: serde_json::Value) -> HashMap<String, Value> {
    fn convert(v: serde_json::Value) -> Value {
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
            serde_json::Value::Array(a) => Value::Array(a.into_iter().map(convert).collect()),
            serde_json::Value::Object(o) => {
                Value::Object(o.into_iter().map(|(k, v)| (k, convert(v))).collect())
            }
        }
    }
    v.as_object()
        .expect("json_ctx: top-level value must be a JSON object")
        .iter()
        .map(|(k, v)| (k.clone(), convert(v.clone())))
        .collect()
}
