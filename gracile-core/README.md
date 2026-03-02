![gracile-core](gracile-core.png)

# gracile-core

The engine at the heart of Gracile. Contains the lexer, parser, AST, and renderer. Use this crate directly if you want to embed the engine into your own library or framework without the re-export layer.

```toml
[dependencies]
gracile-core = "0.1"
```

```rust
use gracile_core::{Engine, Value, context};

let engine = Engine::new()
    .with_strict()
    .register_filter("shout", |val, _args| {
        match val {
            Value::String(s) => Ok(Value::String(format!("{}!!!", s.to_uppercase()))),
            other => Ok(other),
        }
    });

let ctx = context! {
    title => "hello world"
};

let output = engine.render("{= title | shout}", ctx)?;
// → "HELLO WORLD!!!"
```

## Syntax at a glance

| Syntax | Description |
|---|---|
| `{= expr }` | Interpolate (HTML-escaped) |
| `{~ expr }` | Interpolate raw HTML (unescaped) |
| `{! comment !}` | Comment, stripped from output |
| `{#if cond} … {:else} … {/if}` | Conditional |
| `{#each items as item, i} … {:else} … {/each}` | Loop with optional index |
| `{#each items as item, i, loop} … {/each}` | Loop with metadata (`loop.index`, `.length`, `.first`, `.last`) |
| `{#snippet name(params)} … {/snippet}` | Define a reusable snippet |
| `{@render name(args)}` | Render a snippet |
| `{@include "name"}` | Inline a pre-registered template |
| `{@const name = expr}` | Local binding |
| `{#raw} … {/raw}` | Verbatim block, no parsing |
| `{\=` / `{\~` | Escape: emit a literal `{=` / `{~` |

Most users should depend on the [`gracile`](../gracile) crate instead, which re-exports this crate's public API under a cleaner name.
