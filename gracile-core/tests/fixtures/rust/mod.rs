/// Simple gracile Rust templates
use serde_json::json;

use crate::{fixtures::render_file, json_ctx};

#[test]
fn struct_test() {
    let output = render_file(
        "rust/templates/struct.rs.gtl",
        json_ctx(json!({
            "imports": ["std::collections::HashMap"],
            "accessModifier": "pub",
            "structName": "User",
            "fields": [
                {
                    "accessModifier": "pub",
                    "name": "id",
                    "type": "i64"
                },
                {
                    "accessModifier": "pub",
                    "name": "username",
                    "type": "String"
                },
                {
                    "accessModifier": "pub",
                    "name": "data",
                    "type": "HashMap<String, String>"
                }
            ]
        })),
    );

    insta::assert_snapshot!(output);
}
