pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod renderer;
pub mod value;

pub use error::{Error, Result};
pub use renderer::{Engine, FilterFn, LoaderFn};
pub use value::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn ctx(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn render(src: &str, pairs: &[(&str, Value)]) -> String {
        Engine::new()
            .render(src, ctx(pairs))
            .expect("render failed")
    }

    // ── Interpolation ──────────────────────────────────────────────────────

    #[test]
    fn basic_interpolation() {
        assert_eq!(
            render("Hello, {name}!", &[("name", Value::from("World"))]),
            "Hello, World!"
        );
    }

    #[test]
    fn auto_escape() {
        let out = render("{s}", &[("s", Value::from("<b>hi</b>"))]);
        assert_eq!(out, "&lt;b&gt;hi&lt;/b&gt;");
    }

    #[test]
    fn raw_html_tag() {
        let out = render("{@html s}", &[("s", Value::from("<b>hi</b>"))]);
        assert_eq!(out, "<b>hi</b>");
    }

    #[test]
    fn member_access() {
        let mut obj = HashMap::new();
        obj.insert("name".to_string(), Value::from("Alice"));
        let out = render("{user.name}", &[("user", Value::Object(obj))]);
        assert_eq!(out, "Alice");
    }

    #[test]
    fn comment_stripped() {
        assert_eq!(render("a{! comment !}b", &[]), "ab");
    }

    // ── Control flow ──────────────────────────────────────────────────────

    #[test]
    fn if_true() {
        let out = render("{#if flag}yes{/if}", &[("flag", Value::Bool(true))]);
        assert_eq!(out, "yes");
    }

    #[test]
    fn if_false() {
        let out = render("{#if flag}yes{/if}", &[("flag", Value::Bool(false))]);
        assert_eq!(out, "");
    }

    #[test]
    fn if_else() {
        let out = render(
            "{#if flag}yes{:else}no{/if}",
            &[("flag", Value::Bool(false))],
        );
        assert_eq!(out, "no");
    }

    #[test]
    fn if_else_if() {
        let out = render(
            "{#if x == 1}one{:else if x == 2}two{:else}other{/if}",
            &[("x", Value::Int(2))],
        );
        assert_eq!(out, "two");
    }

    // ── Each block ────────────────────────────────────────────────────────

    #[test]
    fn each_basic() {
        let items = Value::Array(vec![Value::from("a"), Value::from("b"), Value::from("c")]);
        let out = render("{#each items as item}{item}{/each}", &[("items", items)]);
        assert_eq!(out, "abc");
    }

    #[test]
    fn each_with_index() {
        let items = Value::Array(vec![Value::from("x"), Value::from("y")]);
        let out = render(
            "{#each items as item, i}{i}:{item} {/each}",
            &[("items", items)],
        );
        assert_eq!(out, "0:x 1:y ");
    }

    #[test]
    fn each_else_empty() {
        let out = render(
            "{#each items as item}{item}{:else}empty{/each}",
            &[("items", Value::Array(vec![]))],
        );
        assert_eq!(out, "empty");
    }

    #[test]
    fn each_destructure() {
        let mut obj = HashMap::new();
        obj.insert("name".to_string(), Value::from("Alice"));
        obj.insert("age".to_string(), Value::Int(30));
        let items = Value::Array(vec![Value::Object(obj)]);
        let out = render(
            "{#each items as { name, age }}{name}={age}{/each}",
            &[("items", items)],
        );
        assert_eq!(out, "Alice=30");
    }

    // ── Snippets ──────────────────────────────────────────────────────────

    #[test]
    fn snippet_and_render() {
        let out = render(
            "{#snippet greet(who)}Hello, {who}!{/snippet}{@render greet(\"World\")}",
            &[],
        );
        assert_eq!(out, "Hello, World!");
    }

    // ── Raw block ─────────────────────────────────────────────────────────

    #[test]
    fn raw_block() {
        let out = render("{#raw}{name} is {#if} not parsed{/raw}", &[]);
        assert_eq!(out, "{name} is {#if} not parsed");
    }

    // ── Const tag ─────────────────────────────────────────────────────────

    #[test]
    fn const_tag() {
        let out = render("{@const x = 42}{x}", &[]);
        assert_eq!(out, "42");
    }

    // ── Expressions ───────────────────────────────────────────────────────

    #[test]
    fn ternary() {
        let out = render("{x > 0 ? \"pos\" : \"non-pos\"}", &[("x", Value::Int(5))]);
        assert_eq!(out, "pos");
    }

    #[test]
    fn nullish_coalesce() {
        let out = render("{name ?? \"default\"}", &[("name", Value::Null)]);
        assert_eq!(out, "default");
    }

    #[test]
    fn string_concat() {
        let out = render(
            "{a + \" \" + b}",
            &[("a", Value::from("Hello")), ("b", Value::from("World"))],
        );
        assert_eq!(out, "Hello World");
    }

    #[test]
    fn arithmetic() {
        let out = render("{a + b * 2}", &[("a", Value::Int(1)), ("b", Value::Int(3))]);
        assert_eq!(out, "7");
    }

    #[test]
    fn unary_not() {
        let out = render("{#if !flag}yes{/if}", &[("flag", Value::Bool(false))]);
        assert_eq!(out, "yes");
    }

    // ── Filters ───────────────────────────────────────────────────────────

    #[test]
    fn filter_upper() {
        let out = render("{s | upper}", &[("s", Value::from("hello"))]);
        assert_eq!(out, "HELLO");
    }

    #[test]
    fn filter_lower() {
        let out = render("{s | lower}", &[("s", Value::from("HELLO"))]);
        assert_eq!(out, "hello");
    }

    #[test]
    fn filter_truncate() {
        let out = render("{s | truncate(5)}", &[("s", Value::from("Hello, World!"))]);
        assert_eq!(out, "He...");
    }

    #[test]
    fn filter_join() {
        let items = Value::Array(vec![Value::from("a"), Value::from("b"), Value::from("c")]);
        let out = render("{items | join(\", \")}", &[("items", items)]);
        assert_eq!(out, "a, b, c");
    }

    #[test]
    fn filter_length() {
        let out = render("{s | length}", &[("s", Value::from("hello"))]);
        assert_eq!(out, "5");
    }

    #[test]
    fn filter_default() {
        let out = render("{name | default(\"anon\")}", &[("name", Value::Null)]);
        assert_eq!(out, "anon");
    }

    #[test]
    fn filter_round() {
        let out = render("{n | round(2)}", &[("n", Value::Float(1.23456))]);
        assert_eq!(out, "1.23");
    }

    #[test]
    fn filter_chain() {
        let out = render(
            "{s | lower | capitalize}",
            &[("s", Value::from("HELLO WORLD"))],
        );
        assert_eq!(out, "Hello world");
    }

    // ── is tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_defined() {
        let out = render(
            "{#if x is defined}yes{:else}no{/if}",
            &[("x", Value::Int(1))],
        );
        assert_eq!(out, "yes");
        let out = render("{#if x is defined}yes{:else}no{/if}", &[]);
        assert_eq!(out, "no");
    }

    #[test]
    fn test_empty() {
        let out = render("{#if s is empty}yes{/if}", &[("s", Value::from(""))]);
        assert_eq!(out, "yes");
    }

    // ── in operator ───────────────────────────────────────────────────────

    #[test]
    fn in_array() {
        let items = Value::Array(vec![Value::from("a"), Value::from("b")]);
        let out = render(
            "{#if x in items}yes{:else}no{/if}",
            &[("x", Value::from("a")), ("items", items)],
        );
        assert_eq!(out, "yes");
    }

    // ── Error messages ────────────────────────────────────────────────────

    #[test]
    fn show_error_messages() {
        use std::collections::HashMap as M;
        let cases: &[(&str, &str)] = &[
            ("unclosed string", r#"Hello {"world}"#),
            ("unclosed if block", r#"{#if true}hello"#),
            ("bad special tag", r#"{@foo bar}"#),
            ("unknown filter", r#"{name | shout}"#),
        ];
        for (label, tmpl) in cases {
            let err = Engine::new().render(tmpl, M::new()).unwrap_err();
            println!("[{}]\n  template: {:?}\n  error:    {}\n", label, tmpl, err);
        }
        let err = Engine::new()
            .with_strict()
            .render("{missing}", M::new())
            .unwrap_err();
        println!("[strict undefined]\n  error: {}\n", err);
    }

    // ── Standalone line stripping ─────────────────────────────────────────
    //
    // When a block tag is the only thing on a line (optionally preceded by
    // spaces/tabs), that entire line is silently removed from the output.
    // This mirrors the Handlebars "standalone line" rule and means no explicit
    // trim modifier syntax is needed.

    #[test]
    fn standalone_block_strips_its_line() {
        // The {#if} and {/if} tags each occupy their own line; both lines should
        // disappear, leaving only the inner content.
        let out = render("before\n{#if true}\nyes\n{/if}\nafter", &[]);
        assert_eq!(out, "before\nyes\nafter");
    }

    #[test]
    fn standalone_with_indentation() {
        // Indentation before the tag is also stripped.
        let out = render("before\n  {#if true}\n    yes\n  {/if}\nafter", &[]);
        assert_eq!(out, "before\n    yes\nafter");
    }

    #[test]
    fn inline_block_not_standalone() {
        // A block tag that shares a line with content is NOT standalone.
        let out = render("a {#if true}b{/if} c", &[]);
        assert_eq!(out, "a b c");
    }

    #[test]
    fn standalone_each_strips_its_line() {
        let items = Value::Array(vec![Value::from("x"), Value::from("y")]);
        let out = render(
            "list:\n{#each items as item}\n- {item}\n{/each}\ndone",
            &[("items", items)],
        );
        assert_eq!(out, "list:\n- x\n- y\ndone");
    }

    // ── Value unit tests ──────────────────────────────────────────────────

    #[test]
    fn value_is_truthy() {
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::Int(1).is_truthy());
        assert!(Value::Int(-1).is_truthy());
        assert!(!Value::Float(0.0).is_truthy());
        assert!(Value::Float(1.5).is_truthy());
        assert!(!Value::String(String::new()).is_truthy());
        assert!(Value::String("x".into()).is_truthy());
        assert!(Value::Array(vec![]).is_truthy()); // empty array is truthy
        assert!(Value::Array(vec![Value::Int(1)]).is_truthy());
        assert!(Value::Object(HashMap::new()).is_truthy());
    }

    #[test]
    fn value_is_null() {
        assert!(Value::Null.is_null());
        assert!(!Value::Bool(false).is_null());
        assert!(!Value::Int(0).is_null());
        assert!(!Value::String(String::new()).is_null());
    }

    #[test]
    fn value_type_names() {
        assert_eq!(Value::Null.type_name(), "null");
        assert_eq!(Value::Bool(true).type_name(), "bool");
        assert_eq!(Value::Int(1).type_name(), "int");
        assert_eq!(Value::Float(1.0).type_name(), "float");
        assert_eq!(Value::String("x".into()).type_name(), "string");
        assert_eq!(Value::Array(vec![]).type_name(), "array");
        assert_eq!(Value::Object(HashMap::new()).type_name(), "object");
    }

    #[test]
    fn value_display_string() {
        assert_eq!(Value::Null.to_display_string(), "null");
        assert_eq!(Value::Bool(true).to_display_string(), "true");
        assert_eq!(Value::Bool(false).to_display_string(), "false");
        assert_eq!(Value::Int(42).to_display_string(), "42");
        assert_eq!(Value::Float(1.5).to_display_string(), "1.5");
        // Whole-number floats display without decimal point
        assert_eq!(Value::Float(2.0).to_display_string(), "2");
        assert_eq!(Value::String("hi".into()).to_display_string(), "hi");
        assert_eq!(
            Value::Array(vec![Value::Int(1), Value::Int(2)]).to_display_string(),
            "1,2"
        );
        assert_eq!(
            Value::Object(HashMap::new()).to_display_string(),
            "[object Object]"
        );
    }

    #[test]
    fn value_json_string() {
        assert_eq!(Value::Null.to_json_string(), "null");
        assert_eq!(Value::Bool(true).to_json_string(), "true");
        assert_eq!(Value::Bool(false).to_json_string(), "false");
        assert_eq!(Value::Int(7).to_json_string(), "7");
        assert_eq!(Value::Float(1.5).to_json_string(), "1.5");
        assert_eq!(Value::String("hello".into()).to_json_string(), r#""hello""#);
        // String escaping
        assert_eq!(
            Value::String("a\"b\\c\nd\re\t".into()).to_json_string(),
            r#""a\"b\\c\nd\re\t""#
        );
        // Array
        assert_eq!(
            Value::Array(vec![Value::Int(1), Value::Bool(true), Value::Null]).to_json_string(),
            "[1,true,null]"
        );
        // Object (keys are sorted for determinism)
        let mut obj = HashMap::new();
        obj.insert("z".to_string(), Value::Int(1));
        obj.insert("a".to_string(), Value::Int(2));
        assert_eq!(Value::Object(obj).to_json_string(), r#"{"a":2,"z":1}"#);
    }

    #[test]
    fn value_length_and_is_empty() {
        assert_eq!(Value::String("hello".into()).length(), Some(5));
        assert_eq!(
            Value::Array(vec![Value::Int(1), Value::Int(2)]).length(),
            Some(2)
        );
        assert_eq!(Value::Object(HashMap::new()).length(), Some(0));
        assert_eq!(Value::Null.length(), None);
        assert_eq!(Value::Int(1).length(), None);

        assert!(Value::String(String::new()).is_empty());
        assert!(!Value::String("x".into()).is_empty());
        assert!(Value::Array(vec![]).is_empty());
        assert!(!Value::Array(vec![Value::Null]).is_empty());
        assert!(Value::Object(HashMap::new()).is_empty());
        // Non-string/array/object always returns false
        assert!(!Value::Null.is_empty());
        assert!(!Value::Int(0).is_empty());
        assert!(!Value::Bool(false).is_empty());
    }

    #[test]
    fn value_html_escape_fn() {
        use crate::value::html_escape;
        assert_eq!(
            html_escape(r#"<div class="x">&it's</div>"#),
            "&lt;div class=&quot;x&quot;&gt;&amp;it&#x27;s&lt;/div&gt;"
        );
        assert_eq!(html_escape("no specials"), "no specials");
    }

    #[test]
    fn value_urlencode_fn() {
        use crate::value::urlencode;
        assert_eq!(urlencode("hello world"), "hello%20world");
        assert_eq!(urlencode("a-b_c.d~e"), "a-b_c.d~e"); // unreserved chars pass through
        assert_eq!(urlencode("a+b=c&d"), "a%2Bb%3Dc%26d");
        assert_eq!(urlencode(""), "");
    }

    #[test]
    fn value_from_impls() {
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(Value::from(false), Value::Bool(false));
        assert_eq!(Value::from(42i64), Value::Int(42));
        assert_eq!(Value::from(1.5f64), Value::Float(1.5));
        assert_eq!(Value::from("hi"), Value::String("hi".into()));
        assert_eq!(Value::from("hi".to_string()), Value::String("hi".into()));
        assert_eq!(
            Value::from(vec![Value::Null]),
            Value::Array(vec![Value::Null])
        );
        let mut m = HashMap::new();
        m.insert("k".to_string(), Value::Int(1));
        assert!(matches!(Value::from(m), Value::Object(_)));
    }

    #[test]
    fn value_display_trait() {
        assert_eq!(format!("{}", Value::Int(99)), "99");
        assert_eq!(format!("{}", Value::from("hi")), "hi");
    }

    // ── Error display ─────────────────────────────────────────────────────

    #[test]
    fn error_display_variants() {
        use crate::error::{Error, Span};
        let lex = Error::LexError {
            message: "bad token".into(),
            span: Span::new(3, 7, 0),
        };
        assert!(lex.to_string().contains("Lex error at 3:7: bad token"));
        let parse = Error::ParseError {
            message: "bad syntax".into(),
            span: Span::unknown(),
        };
        assert!(parse.to_string().contains("Parse error at 0:0: bad syntax"));
        let render = Error::RenderError {
            message: "bad render".into(),
        };
        assert!(render.to_string().contains("Render error: bad render"));
    }

    // ── Context-aware `+` ────────────────────────────────────────────────

    #[test]
    fn add_two_strings() {
        let out = render(
            "{a + ' ' + b}",
            &[("a", Value::from("Hello")), ("b", Value::from("World"))],
        );
        assert_eq!(out, "Hello World");
    }

    #[test]
    fn add_int_and_string_coerces_to_string() {
        let out = render("{count + ' items'}", &[("count", Value::Int(5))]);
        assert_eq!(out, "5 items");
    }

    #[test]
    fn add_null_and_string_coerces() {
        let out = render("{x + ' end'}", &[("x", Value::Null)]);
        assert_eq!(out, "null end");
    }

    // ── Strict mode ───────────────────────────────────────────────────────

    #[test]
    fn strict_null_property_access_errors() {
        let err = Engine::new()
            .with_strict()
            .render("{x.y}", ctx(&[("x", Value::Null)]))
            .unwrap_err();
        assert!(err.to_string().contains("null"));
    }

    #[test]
    fn strict_missing_property_errors() {
        let mut obj = HashMap::new();
        obj.insert("a".to_string(), Value::Int(1));
        let err = Engine::new()
            .with_strict()
            .render("{x.b}", ctx(&[("x", Value::Object(obj))]))
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn strict_array_out_of_bounds_errors() {
        let arr = Value::Array(vec![Value::Int(1)]);
        let err = Engine::new()
            .with_strict()
            .render("{a[5]}", ctx(&[("a", arr)]))
            .unwrap_err();
        assert!(err.to_string().contains("out of bounds"));
    }

    #[test]
    fn lax_out_of_bounds_returns_null() {
        let arr = Value::Array(vec![Value::Int(1)]);
        let out = render("{a[5] ?? 'none'}", &[("a", arr)]);
        assert_eq!(out, "none");
    }

    #[test]
    fn lax_null_member_returns_null() {
        let out = render("{x.y ?? 'nil'}", &[("x", Value::Null)]);
        assert_eq!(out, "nil");
    }

    #[test]
    fn member_access_non_object_strict_errors() {
        let err = Engine::new()
            .with_strict()
            .render("{a.b}", ctx(&[("a", Value::Int(1))]))
            .unwrap_err();
        assert!(err.to_string().contains("Cannot access property"));
    }

    #[test]
    fn member_access_non_object_lax_is_null() {
        let out = render("{a.b ?? 'nil'}", &[("a", Value::Int(1))]);
        assert_eq!(out, "nil");
    }

    // ── Arithmetic errors ─────────────────────────────────────────────────

    #[test]
    fn division_by_zero_int() {
        let err = Engine::new()
            .render("{a / 0}", ctx(&[("a", Value::Int(5))]))
            .unwrap_err();
        assert!(err.to_string().contains("Division by zero"));
    }

    #[test]
    fn division_by_zero_float() {
        let err = Engine::new()
            .render("{a / 0.0}", ctx(&[("a", Value::Float(1.0))]))
            .unwrap_err();
        assert!(err.to_string().contains("Division by zero"));
    }

    #[test]
    fn subtraction_non_numbers_errors() {
        let err = Engine::new()
            .render(
                "{a - b}",
                ctx(&[("a", Value::from("x")), ("b", Value::from("y"))]),
            )
            .unwrap_err();
        assert!(err.to_string().contains("Arithmetic requires numbers"));
    }

    #[test]
    fn unary_neg_non_number_errors() {
        let err = Engine::new()
            .render("{-a}", ctx(&[("a", Value::from("x"))]))
            .unwrap_err();
        assert!(err.to_string().contains("negate"));
    }

    // ── Comparison ────────────────────────────────────────────────────────

    #[test]
    fn compare_incompatible_types_errors() {
        let err = Engine::new()
            .render(
                "{a < b}",
                ctx(&[("a", Value::Int(1)), ("b", Value::from("x"))]),
            )
            .unwrap_err();
        assert!(err.to_string().contains("Cannot compare"));
    }

    #[test]
    fn compare_strings_lexicographically() {
        assert_eq!(
            render(
                "{a < b ? 'yes' : 'no'}",
                &[("a", Value::from("abc")), ("b", Value::from("xyz"))]
            ),
            "yes"
        );
    }

    #[test]
    fn compare_int_and_float() {
        assert_eq!(
            render(
                "{a >= b ? 'yes' : 'no'}",
                &[("a", Value::Int(2)), ("b", Value::Float(1.5))]
            ),
            "yes"
        );
    }

    // ── is tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_undefined() {
        assert_eq!(render("{#if x is undefined}yes{:else}no{/if}", &[]), "yes");
        assert_eq!(
            render(
                "{#if x is undefined}yes{:else}no{/if}",
                &[("x", Value::Int(1))]
            ),
            "no"
        );
    }

    #[test]
    fn test_none() {
        assert_eq!(
            render("{#if x is none}yes{/if}", &[("x", Value::Null)]),
            "yes"
        );
        assert_eq!(
            render("{#if x is none}no{:else}yes{/if}", &[("x", Value::Int(1))]),
            "yes"
        );
    }

    #[test]
    fn test_truthy_falsy() {
        assert_eq!(
            render("{#if x is truthy}yes{/if}", &[("x", Value::Int(1))]),
            "yes"
        );
        assert_eq!(
            render(
                "{#if x is truthy}no{:else}yes{/if}",
                &[("x", Value::Int(0))]
            ),
            "yes"
        );
        assert_eq!(
            render("{#if x is falsy}yes{/if}", &[("x", Value::Int(0))]),
            "yes"
        );
        assert_eq!(
            render(
                "{#if x is falsy}no{:else}yes{/if}",
                &[("x", Value::Bool(true))]
            ),
            "yes"
        );
    }

    #[test]
    fn test_string_number_iterable() {
        assert_eq!(
            render("{#if x is string}yes{/if}", &[("x", Value::from("hi"))]),
            "yes"
        );
        assert_eq!(
            render(
                "{#if x is string}no{:else}yes{/if}",
                &[("x", Value::Int(1))]
            ),
            "yes"
        );
        assert_eq!(
            render("{#if x is number}yes{/if}", &[("x", Value::Int(1))]),
            "yes"
        );
        assert_eq!(
            render("{#if x is number}yes{/if}", &[("x", Value::Float(1.0))]),
            "yes"
        );
        assert_eq!(
            render(
                "{#if x is iterable}yes{/if}",
                &[("x", Value::Array(vec![]))]
            ),
            "yes"
        );
        assert_eq!(
            render(
                "{#if x is iterable}no{:else}yes{/if}",
                &[("x", Value::Int(1))]
            ),
            "yes"
        );
    }

    #[test]
    fn test_odd_even_non_number_errors() {
        let err = Engine::new()
            .render("{#if x is odd}y{/if}", ctx(&[("x", Value::from("a"))]))
            .unwrap_err();
        assert!(err.to_string().contains("odd"));
        let err = Engine::new()
            .render("{#if x is even}y{/if}", ctx(&[("x", Value::from("a"))]))
            .unwrap_err();
        assert!(err.to_string().contains("even"));
    }

    #[test]
    fn test_unknown_in_strict_mode_errors() {
        let err = Engine::new()
            .with_strict()
            .render("{#if x is foobar}y{/if}", ctx(&[("x", Value::Int(1))]))
            .unwrap_err();
        assert!(err.to_string().contains("Unknown test"));
    }

    #[test]
    fn test_is_not() {
        assert_eq!(
            render("{#if x is not empty}yes{/if}", &[("x", Value::from("hi"))]),
            "yes"
        );
        assert_eq!(
            render(
                "{#if x is not empty}no{:else}yes{/if}",
                &[("x", Value::from(""))]
            ),
            "yes"
        );
        // x absent → value is null → defined=false → is not defined=true
        assert_eq!(render("{#if x is not defined}yes{/if}", &[]), "yes");
        // x=Null → defined=false → is not defined=true → renders the if-body
        assert_eq!(
            render(
                "{#if x is not defined}yes{:else}no{/if}",
                &[("x", Value::Null)]
            ),
            "yes"
        );
    }

    // ── Membership ────────────────────────────────────────────────────────

    #[test]
    fn in_string_substring() {
        let out = render(
            "{#if x in s}yes{:else}no{/if}",
            &[("x", Value::from("ell")), ("s", Value::from("hello"))],
        );
        assert_eq!(out, "yes");
    }

    #[test]
    fn in_object_checks_keys() {
        let mut obj = HashMap::new();
        obj.insert("name".to_string(), Value::Int(1));
        assert_eq!(
            render("{#if 'name' in o}yes{/if}", &[("o", Value::Object(obj))]),
            "yes"
        );
    }

    #[test]
    fn not_in_array() {
        let arr = Value::Array(vec![Value::from("a"), Value::from("b")]);
        assert_eq!(
            render("{#if 'c' not in items}yes{/if}", &[("items", arr)]),
            "yes"
        );
    }

    #[test]
    fn in_incompatible_type_errors() {
        let err = Engine::new()
            .render("{#if 1 in x}y{/if}", ctx(&[("x", Value::Int(42))]))
            .unwrap_err();
        assert!(err.to_string().contains("'in' operator"));
    }

    // ── Index access ──────────────────────────────────────────────────────

    #[test]
    fn index_negative_wraps() {
        let arr = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(render("{a[-1]}", &[("a", arr)]), "3");
    }

    #[test]
    fn index_object_by_string_key() {
        let mut obj = HashMap::new();
        obj.insert("key".to_string(), Value::from("val"));
        assert_eq!(render("{o['key']}", &[("o", Value::Object(obj))]), "val");
    }

    #[test]
    fn index_non_integer_on_array_errors() {
        let arr = Value::Array(vec![Value::Int(1)]);
        let err = Engine::new()
            .render("{a['x']}", ctx(&[("a", arr)]))
            .unwrap_err();
        assert!(err.to_string().contains("integer"));
    }

    #[test]
    fn index_into_null_strict_errors() {
        let err = Engine::new()
            .with_strict()
            .render("{a[0]}", ctx(&[("a", Value::Null)]))
            .unwrap_err();
        assert!(err.to_string().contains("null"));
    }

    #[test]
    fn index_into_non_collection_errors() {
        let err = Engine::new()
            .render("{a[0]}", ctx(&[("a", Value::Int(5))]))
            .unwrap_err();
        assert!(err.to_string().contains("Cannot index"));
    }

    // ── Each with non-array ───────────────────────────────────────────────

    #[test]
    fn each_non_array_errors() {
        let err = Engine::new()
            .render(
                "{#each x as item}{item}{/each}",
                ctx(&[("x", Value::Int(1))]),
            )
            .unwrap_err();
        assert!(err.to_string().contains("array"));
    }

    #[test]
    fn each_null_iterable_uses_else() {
        let out = render(
            "{#each x as item}{item}{:else}empty{/each}",
            &[("x", Value::Null)],
        );
        assert_eq!(out, "empty");
    }

    // ── Snippet / render errors ───────────────────────────────────────────

    #[test]
    fn render_unknown_snippet_errors() {
        let err = Engine::new()
            .render("{@render missing()}", ctx(&[]))
            .unwrap_err();
        assert!(err.to_string().contains("Unknown snippet"));
    }

    #[test]
    fn render_wrong_arg_count_errors() {
        let err = Engine::new()
            .render("{#snippet foo(a, b)}ok{/snippet}{@render foo(1)}", ctx(&[]))
            .unwrap_err();
        assert!(err.to_string().contains("expects"));
    }

    // ── Include errors ────────────────────────────────────────────────────

    #[test]
    fn include_not_found_errors() {
        let err = Engine::new()
            .render("{@include 'no_such.html'}", ctx(&[]))
            .unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("template")
                || err.to_string().contains("not found")
        );
    }

    // ── Filters ───────────────────────────────────────────────────────────

    #[test]
    fn filter_replace() {
        let out = render("{s | replace('o', '0')}", &[("s", Value::from("foobar"))]);
        assert_eq!(out, "f00bar");
    }

    #[test]
    fn filter_split_and_join() {
        let out = render(
            "{s | split(',') | join(' ')}",
            &[("s", Value::from("a,b,c"))],
        );
        assert_eq!(out, "a b c");
    }

    #[test]
    fn filter_sort_and_reverse() {
        let arr = Value::Array(vec![Value::from("c"), Value::from("a"), Value::from("b")]);
        assert_eq!(
            render("{items | sort | join(',')}", &[("items", arr.clone())]),
            "a,b,c"
        );
        assert_eq!(
            render("{items | reverse | join(',')}", &[("items", arr)]),
            "b,a,c"
        );
    }

    #[test]
    fn filter_reverse_string() {
        assert_eq!(render("{s | reverse}", &[("s", Value::from("abc"))]), "cba");
    }

    #[test]
    fn filter_first_and_last() {
        let arr = Value::Array(vec![Value::Int(10), Value::Int(20), Value::Int(30)]);
        assert_eq!(render("{items | first}", &[("items", arr.clone())]), "10");
        assert_eq!(render("{items | last}", &[("items", arr)]), "30");
    }

    #[test]
    fn filter_first_last_empty_array() {
        // first/last on an empty array returns null, which renders as "null"
        let arr = Value::Array(vec![]);
        assert_eq!(render("{items | first}", &[("items", arr.clone())]), "null");
        assert_eq!(render("{items | last}", &[("items", arr)]), "null");
    }

    #[test]
    fn filter_json() {
        let arr = Value::Array(vec![Value::Int(1), Value::Bool(false), Value::Null]);
        assert_eq!(render("{v | json}", &[("v", arr)]), "[1,false,null]");
    }

    #[test]
    fn filter_json_object() {
        let mut obj = HashMap::new();
        obj.insert("x".to_string(), Value::Int(1));
        // json output contains quotes → use {@html} to bypass auto-escaping
        let out = Engine::new()
            .render("{@html v | json}", ctx(&[("v", Value::Object(obj))]))
            .unwrap();
        assert_eq!(out, r#"{"x":1}"#);
    }

    #[test]
    fn filter_urlencode() {
        let out = render("{s | urlencode}", &[("s", Value::from("a b+c"))]);
        assert_eq!(out, "a%20b%2Bc");
    }

    #[test]
    fn filter_escape_inside_html_tag() {
        let out = render("{@html s | escape}", &[("s", Value::from("<b>bold</b>"))]);
        assert_eq!(out, "&lt;b&gt;bold&lt;/b&gt;");
    }

    #[test]
    fn filter_unknown_errors() {
        let err = Engine::new()
            .render("{s | nosuchfilter}", ctx(&[("s", Value::from("x"))]))
            .unwrap_err();
        assert!(err.to_string().contains("Unknown filter"));
    }

    #[test]
    fn filter_wrong_type_string_filters() {
        for tmpl in [
            "{n | upper}",
            "{n | lower}",
            "{n | capitalize}",
            "{n | trim}",
            "{n | truncate(5)}",
            "{n | split(',')}",
            "{n | urlencode}",
        ] {
            let err = Engine::new()
                .render(tmpl, ctx(&[("n", Value::Int(1))]))
                .unwrap_err();
            assert!(
                matches!(err, crate::Error::RenderError { .. }),
                "expected RenderError for {tmpl}"
            );
        }
    }

    #[test]
    fn filter_wrong_type_collection_filters() {
        for tmpl in ["{n | sort}", "{n | join}", "{n | first}", "{n | last}"] {
            let err = Engine::new()
                .render(tmpl, ctx(&[("n", Value::from("x"))]))
                .unwrap_err();
            assert!(
                matches!(err, crate::Error::RenderError { .. }),
                "expected RenderError for {tmpl}"
            );
        }
    }

    #[test]
    fn filter_reverse_wrong_type_errors() {
        let err = Engine::new()
            .render("{n | reverse}", ctx(&[("n", Value::Int(1))]))
            .unwrap_err();
        assert!(matches!(err, crate::Error::RenderError { .. }));
    }

    #[test]
    fn filter_round_wrong_type_errors() {
        let err = Engine::new()
            .render("{n | round}", ctx(&[("n", Value::from("x"))]))
            .unwrap_err();
        assert!(matches!(err, crate::Error::RenderError { .. }));
    }

    #[test]
    fn filter_length_wrong_type_errors() {
        let err = Engine::new()
            .render("{n | length}", ctx(&[("n", Value::Int(1))]))
            .unwrap_err();
        assert!(matches!(err, crate::Error::RenderError { .. }));
    }

    // ── render_name / compile / loader ────────────────────────────────────

    #[test]
    fn render_name_via_loader() {
        let engine = Engine::new().with_template_loader(|name| match name {
            "greet" => Ok("Hello, {who}!".to_string()),
            other => Err(crate::Error::RenderError {
                message: format!("not found: {other}"),
            }),
        });
        let mut c = HashMap::new();
        c.insert("who".to_string(), Value::from("World"));
        assert_eq!(engine.render_name("greet", c).unwrap(), "Hello, World!");
    }

    #[test]
    fn render_name_loader_missing_errors() {
        let engine = Engine::new().with_template_loader(|name| {
            Err(crate::Error::RenderError {
                message: format!("not found: {name}"),
            })
        });
        let err = engine.render_name("missing", HashMap::new()).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn compile_and_render_template() {
        let engine = Engine::new();
        let tpl = engine.compile("Hello, {name}!").unwrap();
        let mut c = HashMap::new();
        c.insert("name".to_string(), Value::from("World"));
        let out = engine.render_template(&tpl, c).unwrap();
        assert_eq!(out, "Hello, World!");
    }

    // ── Lexer error paths ─────────────────────────────────────────────────

    #[test]
    fn lex_unterminated_string_errors() {
        let err = Engine::new().render("{\"unclosed}", ctx(&[])).unwrap_err();
        assert!(matches!(err, crate::Error::LexError { .. }));
    }

    #[test]
    fn lex_unknown_escape_errors() {
        let err = Engine::new().render("{\"\\z\"}", ctx(&[])).unwrap_err();
        assert!(err.to_string().contains("escape"));
    }

    #[test]
    fn lex_unicode_escape() {
        // \u{41} = 'A'
        assert_eq!(render("{\"\\u{41}\"}", &[]), "A");
        // emoji
        assert_eq!(render("{\"\\u{1F600}\"}", &[]), "😀");
    }

    #[test]
    fn lex_lone_ampersand_errors() {
        let err = Engine::new()
            .render(
                "{a & b}",
                ctx(&[("a", Value::Int(1)), ("b", Value::Int(2))]),
            )
            .unwrap_err();
        assert!(err.to_string().contains("&&"));
    }

    #[test]
    fn lex_unclosed_comment_errors() {
        let err = Engine::new().render("{! unclosed", ctx(&[])).unwrap_err();
        assert!(matches!(err, crate::Error::LexError { .. }));
    }

    #[test]
    fn lex_unclosed_raw_block_errors() {
        let err = Engine::new()
            .render("{#raw}unclosed", ctx(&[]))
            .unwrap_err();
        assert!(matches!(err, crate::Error::LexError { .. }));
    }

    #[test]
    fn lex_float_scientific_notation() {
        assert_eq!(render("{1.5e1}", &[]), "15");
        assert_eq!(render("{1.0e2}", &[]), "100");
    }

    // ── Parser error paths ────────────────────────────────────────────────

    #[test]
    fn parse_unclosed_if_errors() {
        let err = Engine::new()
            .render("{#if true}unclosed", ctx(&[]))
            .unwrap_err();
        assert!(matches!(err, crate::Error::ParseError { .. }));
    }

    #[test]
    fn parse_unknown_special_tag_errors() {
        let err = Engine::new().render("{@unknown}", ctx(&[])).unwrap_err();
        assert!(matches!(err, crate::Error::ParseError { .. }));
    }

    #[test]
    fn parse_array_literal() {
        let out = render("{[1, 2, 3] | join(',')}", &[]);
        assert_eq!(out, "1,2,3");
    }

    #[test]
    fn parse_nested_array_literal() {
        let out = render("{['a', 'b', 'c'] | length}", &[]);
        assert_eq!(out, "3");
    }
}
