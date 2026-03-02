/// gracile meta-templates
use gracile_core::Value;

use crate::{arr, ctx, fixtures::render_file, obj};

#[test]
fn meta_template() {
    let output = render_file(
        "gracile/templates/template.gracile.gtl",
        ctx(&[
            ("name", Value::from("Article")),
            ("varName", Value::from("article")),
            (
                "fields",
                arr(vec![
                    obj(&[
                        ("name", Value::from("headline")),
                        ("label", Value::from("Headline")),
                        ("tag", Value::from("h2")),
                        ("class", Value::Null),
                        ("isHtml", Value::from(false)),
                        ("optional", Value::from(false)),
                    ]),
                    obj(&[
                        ("name", Value::from("body")),
                        ("label", Value::from("Body")),
                        ("tag", Value::from("div")),
                        ("class", Value::from("body")),
                        ("isHtml", Value::from(true)),
                        ("optional", Value::from(false)),
                    ]),
                    obj(&[
                        ("name", Value::from("summary")),
                        ("label", Value::from("Summary")),
                        ("tag", Value::from("p")),
                        ("class", Value::from("summary")),
                        ("isHtml", Value::from(false)),
                        ("optional", Value::from(true)),
                    ]),
                ]),
            ),
        ]),
    );
    insta::assert_snapshot!(output);
}
