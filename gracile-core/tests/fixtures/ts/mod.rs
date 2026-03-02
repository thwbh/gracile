use gracile_core::Value;

use crate::{arr, ctx, fixtures::render_file, obj};

fn fields(_name: &str, typed_name: &str) -> Value {
    arr(vec![
        obj(&[
            ("serializedName", Value::from("name")),
            ("isOptional", Value::from(true)),
            (typed_name, Value::from("string")),
            (
                "tags",
                Value::from(vec![Value::from("text"), Value::from("personal")]),
            ),
        ]),
        obj(&[
            ("serializedName", Value::from("id")),
            ("isOptional", Value::from(false)),
            (typed_name, Value::from("number")),
            (
                "tags",
                Value::from(vec![Value::from("numeric"), Value::from("unique")]),
            ),
        ]),
    ])
}

#[test]
fn interface() {
    let output = render_file(
        "ts/templates/interface.ts.gtl",
        ctx(&[
            ("name", Value::from("MyInterface")),
            ("fields", fields("name", "typescriptType")),
        ]),
    );
    insta::assert_snapshot!(output);
}

#[test]
fn interface_typed() {
    let output = render_file(
        "ts/templates/interface_typed.ts.gtl",
        ctx(&[
            ("name", Value::from("MyInterface<T>")),
            ("fields", fields("name", "typescriptType")),
        ]),
    );
    insta::assert_snapshot!(output);
}
