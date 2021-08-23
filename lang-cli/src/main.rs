//! [`glsl-lang`](https://crates.io/crates/glsl-lang) debugging CLI.
//!
//! *This is only a prototype for debugging, more options will be added in later updates.*
//!
//! # Usage
//!
//! Print GLSL AST to the standard output:
//! ```bash
//! $ cargo run < source.glsl
//! TranslationUnit
//!   ExternalDeclaration@0:0..45 `Declaration`
//!     Declaration@0:0..45 `Block`
//!       [...]
//! ```

#![deny(missing_docs)]

use std::io::prelude::*;

use argh::FromArgs;

use glsl_lang::ast::{NodeDisplay, TranslationUnit};
use glsl_lang::parse::Parse;

fn output_text(output: &mut dyn std::io::Write, tu: TranslationUnit) -> std::io::Result<()> {
    writeln!(output, "{}", tu.display())?;
    Ok(())
}

#[cfg(feature = "json")]
fn output_json(output: &mut dyn std::io::Write, tu: TranslationUnit) -> std::io::Result<()> {
    serde_json::to_writer(output, &tu)?;
    Ok(())
}

fn output_glsl(output: &mut dyn std::io::Write, tu: TranslationUnit) -> std::io::Result<()> {
    let mut s = String::new();

    glsl_lang::transpiler::glsl::show_translation_unit(
        &mut s,
        &tu,
        glsl_lang::transpiler::glsl::FormattingState::default(),
    )
    .unwrap();

    write!(output, "{}", s)?;

    Ok(())
}

#[derive(Debug, FromArgs)]
/// glsl-lang command-line interface
struct Opts {
    #[argh(option, default = "\"text\".to_owned()")]
    /// output format (text, json or glsl)
    format: String,

    #[argh(positional)]
    /// input file path
    path: Option<String>,
}

/// CLI entry point
fn main() -> Result<(), std::io::Error> {
    let args: Opts = argh::from_env();

    // Figure out output format
    let output_fn = match args.format.as_str() {
        "text" => output_text,
        #[cfg(feature = "json")]
        "json" => output_json,
        "glsl" => output_glsl,
        other => panic!("unknown output format: {}", other),
    };

    let mut s = String::new();

    // Read input from argument or stdin
    if let Some(path) = args.path.as_deref() {
        s = std::fs::read_to_string(path)?;
    } else {
        std::io::stdin().read_to_string(&mut s)?;
    }

    match glsl_lang::ast::TranslationUnit::parse(s.as_str()) {
        Ok(tu) => {
            output_fn(&mut std::io::stdout(), tu)?;
        }
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    }

    Ok(())
}
