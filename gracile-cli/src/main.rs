use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use gracile_core::{Engine, Value};

// ── CLI definition ─────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "gracile",
    about = "Render Gracile templates from the command line",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render a template file
    Render(RenderArgs),
}

#[derive(clap::Args)]
struct RenderArgs {
    /// Template file to render
    template: PathBuf,

    /// Context data: a JSON file path or an inline JSON string
    #[arg(long, short)]
    data: Option<String>,

    /// Write output to a file instead of stdout
    #[arg(long, short)]
    output: Option<PathBuf>,

    /// Error on undefined variables instead of rendering empty
    #[arg(long)]
    strict: bool,
}

// ── Entry point ────────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Render(args) => match run_render(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {}", e);
                ExitCode::FAILURE
            }
        },
    }
}

fn run_render(args: RenderArgs) -> Result<(), String> {
    // Read template source.
    let source = std::fs::read_to_string(&args.template)
        .map_err(|e| format!("cannot read '{}': {}", args.template.display(), e))?;

    // Parse context.
    let context = match args.data {
        None => HashMap::new(),
        Some(ref s) => parse_data(s)?,
    };

    // Build engine with a loader that resolves includes relative to the
    // template's own directory.
    let template_dir = args
        .template
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let mut engine = Engine::new().with_template_loader(move |name| {
        let path = template_dir.join(name);
        std::fs::read_to_string(&path).map_err(|e| gracile_core::Error::RenderError {
            message: format!("cannot load '{}': {}", path.display(), e),
        })
    });

    if args.strict {
        engine = engine.with_strict();
    }

    let output = engine.render(&source, context).map_err(|e| e.to_string())?;

    // Write output.
    match args.output {
        Some(path) => std::fs::write(&path, &output)
            .map_err(|e| format!("cannot write '{}': {}", path.display(), e)),
        None => {
            print!("{}", output);
            Ok(())
        }
    }
}

// ── Data parsing ───────────────────────────────────────────────────────────────

fn parse_data(input: &str) -> Result<HashMap<String, Value>, String> {
    // If it looks like a file path, try reading it first.
    let json_str = if !input.trim_start().starts_with('{') && !input.trim_start().starts_with('[') {
        std::fs::read_to_string(input)
            .map_err(|e| format!("cannot read data file '{}': {}", input, e))?
    } else {
        input.to_string()
    };

    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("invalid JSON: {}", e))?;

    match parsed {
        serde_json::Value::Object(map) => Ok(map
            .into_iter()
            .map(|(k, v)| (k, json_to_value(v)))
            .collect()),
        other => Err(format!(
            "data must be a JSON object, got {}",
            other.type_name()
        )),
    }
}

fn json_to_value(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => Value::Array(arr.into_iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, json_to_value(v)))
                .collect(),
        ),
    }
}

// ── serde_json helpers ─────────────────────────────────────────────────────────

trait JsonTypeName {
    fn type_name(&self) -> &'static str;
}

impl JsonTypeName for serde_json::Value {
    fn type_name(&self) -> &'static str {
        match self {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "bool",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
    }
}
