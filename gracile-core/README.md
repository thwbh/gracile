![gracile-core](gracile-core.png)

# gracile-core

The engine at the heart of Gracile. Contains the lexer, parser, AST, and renderer. Use this crate directly if you want to embed the engine into your own library or framework without the re-export layer.

```toml
[dependencies]
gracile-core = "0.1"
```

```rust
use gracile_core::{Engine, Value, FilterFn};
use std::collections::HashMap;

let engine = Engine::new()
    .with_strict()
    .with_template_loader(|name| {
        std::fs::read_to_string(format!("templates/{}", name))
            .map_err(|e| gracile_core::Error::RenderError {
                message: e.to_string(),
            })
    })
    .register_filter("shout", |val, _args| {
        match val {
            Value::String(s) => Ok(Value::String(format!("{}!!!", s.to_uppercase()))),
            other => Ok(other),
        }
    });

let mut ctx = HashMap::new();
ctx.insert("title".into(), Value::from("hello world"));

let output = engine.render("{title | shout}", ctx)?;
// → "HELLO WORLD!!!"
```

Most users should depend on the [`gracile`](../gracile) crate instead, which re-exports this crate's public API under a cleaner name.
