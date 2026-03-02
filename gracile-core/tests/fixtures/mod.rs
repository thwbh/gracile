use gracile_core::{Engine, Value};
use std::collections::HashMap;

mod gracile;
mod html;
mod java;
mod rust;
mod ts;

pub fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name),
    )
    .unwrap_or_else(|_| panic!("fixture not found: {name}"))
}

pub fn render_file(name: &str, context: HashMap<String, Value>) -> String {
    Engine::new()
        .render(&fixture(name), context)
        .unwrap_or_else(|e| panic!("render failed for {name}: {e}"))
}
