![gracile-wasm](gracile-wasm.png)

# @gracile-rs/wasm

WebAssembly bindings for the gracile templating engine. Use gracile templates from any JavaScript runtime — Node.js, Deno, Bun, or the browser.

## Installation

```sh
npm install @gracile-rs/wasm
```

## Usage

### Quick render

```js
import { render } from '@gracile-rs/wasm';

const html = render('Hello, {name}!', { name: 'World' });
```

### Engine with template loader

```js
import { Engine } from '@gracile-rs/wasm';
import { readFileSync } from 'node:fs';

const engine = new Engine();
engine.strictMode();
engine.setTemplateLoader(name => readFileSync(`./templates/${name}`, 'utf8'));

const html = engine.renderName('page.html', { title: 'Home', year: 2026 });
```

### Custom filters

```js
const engine = new Engine();
engine.registerFilter('shout', v => v.toUpperCase() + '!!!');

const html = engine.render('{title | shout}', { title: 'hello' });
// → "HELLO!!!"
```

## API

| Method | Description |
|---|---|
| `render(template, context)` | Render a template string |
| `new Engine()` | Create a configurable engine instance |
| `engine.strictMode()` | Throw on undefined variables |
| `engine.setTemplateLoader(fn)` | Resolve template names on demand |
| `engine.registerTemplate(name, src)` | Pre-register a named template |
| `engine.registerFilter(name, fn)` | Add a custom filter |
| `engine.render(template, context)` | Render a template string |
| `engine.renderName(name, context)` | Render a named template via the loader |
