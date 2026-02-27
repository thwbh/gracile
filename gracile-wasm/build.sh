#!/usr/bin/env bash
# Build the WASM package and set the correct npm package name.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

wasm-pack build --target bundler "$@"

# wasm-pack names the package after the crate ("gracile-wasm"); rename to the
# scoped npm name so `import { render } from '@gracile/wasm'` works.
node - <<'EOF'
const fs = require('fs');
const path = 'pkg/package.json';
const pkg = JSON.parse(fs.readFileSync(path, 'utf8'));
pkg.name = '@gracile-rs/wasm';
fs.writeFileSync(path, JSON.stringify(pkg, null, 2) + '\n');
console.log('package name set to', pkg.name);
EOF
