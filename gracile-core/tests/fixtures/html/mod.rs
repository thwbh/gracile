use gracile_core::{Engine, Value};

use crate::{
    arr, ctx,
    fixtures::{fixture, render_file},
    obj,
};

#[test]
fn interpolation() {
    let output = render_file(
        "html/templates/interpolation.html.gtl",
        ctx(&[
            ("title", Value::from("My Article")),
            (
                "author",
                obj(&[
                    ("name", Value::from("Alice")),
                    ("email", Value::from("alice@example.com")),
                ]),
            ),
            (
                "stats",
                obj(&[("views", Value::Int(1_234)), ("rating", Value::Float(4.5))]),
            ),
        ]),
    );
    insta::assert_snapshot!(output);
}

#[test]
fn escaping() {
    let output = render_file(
        "html/templates/escaping.html.gtl",
        ctx(&[
            ("user_input", Value::from("<script>alert('xss')</script>")),
            ("html/templates_content", Value::from("<em>emphasis</em>")),
        ]),
    );
    insta::assert_snapshot!(output);
}

#[test]
fn control_flow() {
    let output = render_file(
        "html/templates/control_flow.html.gtl",
        ctx(&[("score", Value::Int(85))]),
    );
    insta::assert_snapshot!(output);
}

#[test]
fn each_with_index_and_else() {
    let output = render_file(
        "html/templates/each_basic.html.gtl",
        ctx(&[(
            "items",
            arr(vec![
                Value::from("apple"),
                Value::from("banana"),
                Value::from("cherry"),
            ]),
        )]),
    );
    insta::assert_snapshot!(output);
}

#[test]
fn each_empty_renders_else() {
    let output = render_file(
        "html/templates/each_basic.html.gtl",
        ctx(&[("items", arr(vec![]))]),
    );
    insta::with_settings!({
        description => "{:else} branch renders when the list is empty"
    }, {
        insta::assert_snapshot!(output);
    });
}

#[test]
fn each_destructure() {
    let output = render_file(
        "html/templates/each_destructure.html.gtl",
        ctx(&[(
            "products",
            arr(vec![
                obj(&[
                    ("name", Value::from("Widget")),
                    ("price", Value::from("$9.99")),
                    ("in_stock", Value::Bool(true)),
                ]),
                obj(&[
                    ("name", Value::from("Gadget")),
                    ("price", Value::from("$24.99")),
                    ("in_stock", Value::Bool(false)),
                ]),
            ]),
        )]),
    );
    insta::assert_snapshot!(output);
}

#[test]
fn filters() {
    let output = render_file(
        "html/templates/filters.html.gtl",
        ctx(&[
            ("name", Value::from("hello world")),
            ("bio", Value::from("This is a rather long biography text.")),
            (
                "tags",
                arr(vec![
                    Value::from("rust"),
                    Value::from("web"),
                    Value::from("templates"),
                ]),
            ),
            ("score", Value::Float(1.23456)),
            ("missing", Value::Null),
        ]),
    );
    insta::assert_snapshot!(output);
}

#[test]
fn expressions() {
    let output = render_file(
        "html/templates/expressions.html.gtl",
        ctx(&[
            ("a", Value::Int(3)),
            ("b", Value::Int(4)),
            ("greeting", Value::Null),
            ("first", Value::from("Jane")),
            ("last", Value::from("Doe")),
            ("x", Value::Int(5)),
            (
                "roles",
                arr(vec![Value::from("user"), Value::from("admin")]),
            ),
            ("age", Value::Int(42)),
            ("bio", Value::from("")),
        ]),
    );
    insta::assert_snapshot!(output);
}

#[test]
fn snippets() {
    let output = render_file("html/templates/snippets.html.gtl", ctx(&[]));
    insta::assert_snapshot!(output);
}

#[test]
fn const_and_raw() {
    let output = render_file(
        "html/templates/const_raw.html.gtl",
        ctx(&[("name", Value::from("World"))]),
    );
    insta::assert_snapshot!(output);
}

#[test]
fn nested_blocks() {
    let output = render_file(
        "html/templates/nested.html.gtl",
        ctx(&[(
            "groups",
            arr(vec![
                obj(&[
                    ("name", Value::from("Team A")),
                    (
                        "members",
                        arr(vec![
                            obj(&[
                                ("name", Value::from("Alice")),
                                ("role", Value::from("lead")),
                            ]),
                            obj(&[("name", Value::from("Bob")), ("role", Value::Null)]),
                        ]),
                    ),
                ]),
                obj(&[
                    ("name", Value::from("Team B")),
                    (
                        "members",
                        arr(vec![obj(&[
                            ("name", Value::from("Carol")),
                            ("role", Value::from("lead")),
                        ])]),
                    ),
                ]),
            ]),
        )]),
    );
    insta::assert_snapshot!(output);
}

#[test]
fn include_partial() {
    let head = fixture("html/templates/include_head.html.gtl");
    let output = Engine::new()
        .register_template("head", head)
        .render(
            &fixture("html/templates/include_main.html.gtl"),
            ctx(&[
                ("page", obj(&[("title", Value::from("Home"))])),
                ("user", obj(&[("name", Value::from("Bob"))])),
            ]),
        )
        .unwrap();
    insta::assert_snapshot!(output);
}

#[test]
fn custom_filter() {
    let output = Engine::new()
        .register_filter("shout", |val, _args| match val {
            Value::String(s) => Ok(Value::String(format!("{}!!!", s.to_uppercase()))),
            other => Ok(other),
        })
        .render(
            &fixture("html/templates/custom_filter.html.gtl"),
            ctx(&[("name", Value::from("Alice"))]),
        )
        .unwrap();
    insta::assert_snapshot!(output);
}
