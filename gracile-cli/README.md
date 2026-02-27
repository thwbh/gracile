![gracile-cli](gracile-cli.png)

# gracile-cli

Command-line interface for the Gracile templating engine. Render templates directly from the terminal with a JSON or TOML data file — no code required.

## Installation

```sh
cargo install gracile-cli
```

## Usage

```sh
# Render a template with data from a JSON file
gracile render template.html --data data.json

# Render with inline JSON data
gracile render template.html --data '{"name": "World"}'

# Write output to a file
gracile render template.html --data data.json --output out.html

# Enable strict mode (undefined variables are errors)
gracile render template.html --data data.json --strict
```

## Example

```
# template.html
Hello, {name}!

{#if items}
  {#each items as item}
    - {item}
  {/each}
{:else}
  Nothing here.
{/if}
```

```json
{ "name": "World", "items": ["one", "two", "three"] }
```

```sh
gracile render template.html --data data.json
```
