![gracile banner](gracile-banner.png)

# gracile

A polyglot templating engine with [Svelte](https://svelte.dev)-inspired syntax. Write a template once — render it from Rust, the command line, or any JavaScript runtime with identical output. Not fast, but elegant.

## Why?

Most template engines are tied to a single runtime. Jinja2 is Python. Tera is Rust. Handlebars has ports but no guarantees of identical behaviour. **gracile** is designed from the ground up to render identically across runtimes — embed it in a Rust binary, call it from the CLI, or import it in Node, Deno, Bun, or the browser. One template, every environment.

The syntax comes from Svelte — concise, expressive, and already familiar to anyone who has written a Svelte component.

## What?

```
Hello, {name}!

{#if items}
  {#each items as { label, count }}
    - {label}: {count | default("n/a")}
  {/each}
{:else}
  Nothing here yet.
{/if}
```

**Expressions & control flow**
- `{expr}` — interpolation (HTML-escaped by default)
- `{@html expr}` — raw unescaped output
- `{#if} {:else if} {:else} {/if}` — conditionals
- `{#each items as pat, idx} {:else} {/each}` — loops with destructuring
- `{#snippet name(params)} {/snippet}` + `{@render name(args)}` — reusable fragments
- `{#raw} {/raw}` — verbatim passthrough
- `{@const name = expr}` — scoped bindings
- `{@include "name"}` — render a pre-registered template
- `{! comment !}` — stripped at render time

**Expressions** support `||`, `&&`, `??`, `? :`, `== != < > <= >=`, `in / not in`, `is / is not`, `+ - * / %`, `!`, `.` and `[]` access, array literals, and filter chains (`value | filter | filter`).

**Filters** — `upper`, `lower`, `capitalize`, `trim`, `truncate`, `replace`, `split`, `sort`, `reverse`, `join`, `first`, `last`, `length`, `default`, `json`, `round`, `urlencode`, `escape`

**`is` tests** — `defined`, `undefined`, `none`, `odd`, `even`, `empty`, `truthy`, `falsy`, `string`, `number`, `iterable`

**Strict mode** — errors on undefined variables instead of silently rendering empty strings.

## How?

### Rust

```rust
use gracile_core::{Engine, Value};
use std::collections::HashMap;

let mut ctx = HashMap::new();
ctx.insert("name".into(), Value::String("world".into()));

let output = Engine::new()
    .with_strict()
    .render("Hello, {name}!", ctx)?;
```

### CLI

```sh
gracile render template.html --data data.json
```

### JavaScript / WASM

```js
import { render } from '@gracile-rs/wasm';

const output = render('Hello, {name}!', { name: 'world' });
```

## Where?

| Package | Description |
|---|---|
| [`gracile-core`](gracile-core/) | Core Rust library |
| [`gracile-cli`](gracile-cli/) | Command-line interface |
| [`gracile-wasm`](gracile-wasm/) | WebAssembly / npm package |
