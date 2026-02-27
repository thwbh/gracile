use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// ── Value conversion ──────────────────────────────────────────────────────────

fn js_to_value(val: &JsValue) -> gracile_core::Value {
    if val.is_null() || val.is_undefined() {
        gracile_core::Value::Null
    } else if let Some(b) = val.as_bool() {
        gracile_core::Value::Bool(b)
    } else if let Some(f) = val.as_f64() {
        if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
            gracile_core::Value::Int(f as i64)
        } else {
            gracile_core::Value::Float(f)
        }
    } else if let Some(s) = val.as_string() {
        gracile_core::Value::String(s)
    } else if js_sys::Array::is_array(val) {
        let arr = js_sys::Array::from(val);
        gracile_core::Value::Array(arr.iter().map(|v| js_to_value(&v)).collect())
    } else if val.is_object() {
        gracile_core::Value::Object(js_object_to_map(val))
    } else {
        gracile_core::Value::Null
    }
}

fn js_object_to_map(val: &JsValue) -> HashMap<String, gracile_core::Value> {
    let obj = js_sys::Object::from(val.clone());
    let mut map = HashMap::new();
    for entry in js_sys::Object::entries(&obj).iter() {
        let pair = js_sys::Array::from(&entry);
        if let Some(key) = pair.get(0).as_string() {
            map.insert(key, js_to_value(&pair.get(1)));
        }
    }
    map
}

fn gracile_value_to_js(val: &gracile_core::Value) -> JsValue {
    match val {
        gracile_core::Value::Null => JsValue::null(),
        gracile_core::Value::Bool(b) => JsValue::from_bool(*b),
        gracile_core::Value::Int(i) => JsValue::from_f64(*i as f64),
        gracile_core::Value::Float(f) => JsValue::from_f64(*f),
        gracile_core::Value::String(s) => JsValue::from_str(s),
        gracile_core::Value::Array(arr) => {
            let js_arr = js_sys::Array::new();
            for v in arr {
                js_arr.push(&gracile_value_to_js(v));
            }
            js_arr.into()
        }
        gracile_core::Value::Object(obj) => {
            let js_obj = js_sys::Object::new();
            for (k, v) in obj {
                let _ =
                    js_sys::Reflect::set(&js_obj, &JsValue::from_str(k), &gracile_value_to_js(v));
            }
            js_obj.into()
        }
    }
}

fn js_err(e: gracile_core::Error) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn js_context(context: JsValue) -> HashMap<String, gracile_core::Value> {
    if context.is_null() || context.is_undefined() {
        HashMap::new()
    } else {
        js_object_to_map(&context)
    }
}

// ── JS callback wrapper (Send + Sync for single-threaded wasm32) ──────────────

struct JsCallback(js_sys::Function);

// SAFETY: wasm32 targets are always single-threaded.
unsafe impl Send for JsCallback {}
unsafe impl Sync for JsCallback {}

// ── Standalone render function ────────────────────────────────────────────────

/// Render a gracile template with a plain JS object as context.
///
/// ```js
/// import { render } from '@gracile-rs/wasm';
/// const html = render('Hello, {name}!', { name: 'World' });
/// ```
#[wasm_bindgen]
pub fn render(template: &str, context: JsValue) -> Result<String, JsValue> {
    gracile_core::Engine::new()
        .render(template, js_context(context))
        .map_err(js_err)
}

// ── Engine class ──────────────────────────────────────────────────────────────

/// A configurable gracile template engine.
///
/// ```js
/// import { Engine } from '@gracile-rs/wasm';
/// import { readFileSync } from 'node:fs';
///
/// const engine = new Engine();
/// engine.strictMode();
/// engine.setTemplateLoader(name => readFileSync(`./templates/${name}`, 'utf8'));
/// engine.registerFilter('shout', v => v.toUpperCase() + '!!');
///
/// // Render a named template loaded from disk:
/// const html = engine.renderName('page.html', { year: 2026 });
///
/// // Or render an inline template that includes others:
/// const html2 = engine.render('{@include "header.html"} body', { year: 2026 });
/// ```
#[wasm_bindgen]
pub struct Engine {
    strict: bool,
    templates: HashMap<String, String>,
    filters: Vec<(String, JsCallback)>,
    loader: Option<JsCallback>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        Engine {
            strict: false,
            templates: HashMap::new(),
            filters: Vec::new(),
            loader: None,
        }
    }

    /// Enable strict mode: undefined variables throw instead of rendering empty.
    pub fn strict_mode(&mut self) {
        self.strict = true;
    }

    /// Pre-register a named template for use with `{@include "name"}`.
    pub fn register_template(&mut self, name: &str, source: &str) {
        self.templates.insert(name.to_string(), source.to_string());
    }

    /// Register a custom filter function.
    ///
    /// The function receives `(value, ...args)` and must return a value.
    /// Throw from the function to signal a render error.
    ///
    /// ```js
    /// engine.registerFilter('shout', v => v.toUpperCase() + '!!');
    /// engine.registerFilter('truncate', (v, n) => v.slice(0, n) + '…');
    /// ```
    pub fn register_filter(&mut self, name: &str, callback: js_sys::Function) {
        self.filters.push((name.to_string(), JsCallback(callback)));
    }

    /// Set a template loader function.
    ///
    /// Called whenever a template name is needed but has not been pre-registered.
    /// The function receives the template name and must return the source string,
    /// or throw to signal an error.
    ///
    /// ```js
    /// // Node.js — load from a templates directory
    /// import { readFileSync } from 'node:fs';
    /// engine.setTemplateLoader(name => readFileSync(`./templates/${name}`, 'utf8'));
    ///
    /// // Browser — synchronous XHR
    /// engine.setTemplateLoader(name => {
    ///   const xhr = new XMLHttpRequest();
    ///   xhr.open('GET', `/templates/${name}`, false);
    ///   xhr.send();
    ///   if (xhr.status !== 200) throw new Error(`template not found: ${name}`);
    ///   return xhr.responseText;
    /// });
    /// ```
    pub fn set_template_loader(&mut self, callback: js_sys::Function) {
        self.loader = Some(JsCallback(callback));
    }

    /// Build the inner `gracile_core::Engine` from the current configuration.
    fn build_inner(&self) -> gracile_core::Engine {
        let mut engine = gracile_core::Engine::new();

        if self.strict {
            engine = engine.with_strict();
        }

        for (name, src) in &self.templates {
            engine = engine.register_template(name, src);
        }

        for (filter_name, cb) in &self.filters {
            let func = cb.0.clone();
            let name_for_err = filter_name.clone();
            engine = engine.register_filter(filter_name.clone(), move |val, args| {
                let js_args = js_sys::Array::new();
                js_args.push(&gracile_value_to_js(&val));
                for a in &args {
                    js_args.push(&gracile_value_to_js(a));
                }
                let result = func.apply(&JsValue::NULL, &js_args).map_err(|e| {
                    gracile_core::Error::RenderError {
                        message: format!(
                            "filter '{}' threw: {}",
                            name_for_err,
                            e.as_string().unwrap_or_default()
                        ),
                    }
                })?;
                Ok(js_to_value(&result))
            });
        }

        if let Some(ref loader_cb) = self.loader {
            let func = loader_cb.0.clone();
            engine = engine.with_template_loader(move |name| {
                let result = func
                    .call1(&JsValue::NULL, &JsValue::from_str(name))
                    .map_err(|e| gracile_core::Error::RenderError {
                        message: format!(
                            "template loader threw for '{}': {}",
                            name,
                            e.as_string().unwrap_or_default()
                        ),
                    })?;
                result
                    .as_string()
                    .ok_or_else(|| gracile_core::Error::RenderError {
                        message: format!("template loader must return a string for '{}'", name),
                    })
            });
        }

        engine
    }

    /// Render a template string with a plain JS object as context.
    pub fn render(&self, template: &str, context: JsValue) -> Result<String, JsValue> {
        self.build_inner()
            .render(template, js_context(context))
            .map_err(js_err)
    }

    /// Resolve a template by name (via loader or pre-registered map) and render it.
    ///
    /// ```js
    /// engine.setTemplateLoader(name => readFileSync(`./templates/${name}`, 'utf8'));
    /// const html = engine.renderName('page.html', { title: 'Home' });
    /// ```
    pub fn render_name(&self, name: &str, context: JsValue) -> Result<String, JsValue> {
        self.build_inner()
            .render_name(name, js_context(context))
            .map_err(js_err)
    }
}
