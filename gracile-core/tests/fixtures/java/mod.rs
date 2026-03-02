use crate::{fixtures::render_file, json_ctx};
use serde_json::json;

#[test]
fn class() {
    let output = render_file(
        "java/templates/class.java.gtl",
        json_ctx(json!({
            "packageName": "com.example.model",
            "imports": ["java.util.List", "java.util.Optional"],
            "timestamp": "2026-03-01",
            "accessModifier": "public",
            "isStatic": false,
            "className": "UserRecord",
            "members": [
                { "type": "String",       "name": "username" },
                { "type": "List<String>", "name": "roles"    },
            ]
        })),
    );

    insta::assert_snapshot!(output);
}

#[test]
fn interface() {
    let output = render_file(
        "java/templates/interface.java.gtl",
        json_ctx(json!({
            "packageName": "com.example.repository",
            "imports": ["java.util.List", "java.util.Optional"],
            "accessModifier": "public",
            "isSealed": false,
            "interfaceName": "Repository",
            "interfaceType": "<T>",
            "methods": [
                {
                    "accessModifier": "public",
                    "returnValue": "List",
                    "isTyped": true,
                    "name": "findAll",
                    "parameters": []
                },
                {
                    "accessModifier": "public",
                    "returnValue": "Optional",
                    "isTyped": true,
                    "name": "findById",
                    "parameters": [{ "type": "Long", "name": "id" }]
                },
                {
                    "accessModifier": "public",
                    "returnValue": "void",
                    "isTyped": false,
                    "name": "save",
                    "parameters": [{ "type": "T", "name": "entity" }]
                }
            ]
        })),
    );

    insta::assert_snapshot!(output);
}
