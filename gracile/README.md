![gracile](gracile-stamp.png)

# gracile

The `gracile` crate is the top-level convenience package. It re-exports everything from [`gracile-core`](../gracile-core) so you can depend on a single crate with a clean name.

```toml
[dependencies]
gracile = "0.1"
```

```rust
use gracile::{Engine, Value};
use std::collections::HashMap;

let engine = Engine::new()
    .with_strict()
    .with_template_loader(|name| {
        std::fs::read_to_string(format!("templates/{}", name))
            .map_err(|e| gracile::Error::RenderError {
                message: e.to_string(),
            })
    });

let mut ctx = HashMap::new();
ctx.insert("name".into(), Value::from("World"));

let output = engine.render_name("greeting.html", ctx)?;
```

See the [main repository](https://github.com/thwbh/gracile) for the full feature overview.
