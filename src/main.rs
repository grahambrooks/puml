mod ast;
mod cli;
mod error;
mod layout;
mod parser;
mod render;

use anyhow::Context;
use clap::Parser as ClapParser;
use std::io::{self, Read, Write};

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    let (source, base_dir) = match &args.input {
        Some(path) => {
            let src =
                std::fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
            let dir = path.parent().map(|p| p.to_path_buf());
            (src, dir)
        }
        None => {
            let mut src = String::new();
            io::stdin()
                .read_to_string(&mut src)
                .context("reading stdin")?;
            (src, None)
        }
    };

    let diagrams = parser::preprocessor::preprocess(&source, base_dir.as_deref());

    if diagrams.is_empty() {
        eprintln!("puml: warning: no diagram content found");
        return Ok(());
    }

    for (i, diagram_src) in diagrams.iter().enumerate() {
        let mut src = diagram_src.clone();
        if let Some(ref t) = args.r#type {
            src.type_hint = Some(t.clone());
        }

        if args.verbose {
            eprintln!("--- diagram {} source ---\n{}", i + 1, src.content);
        }

        let ast = parser::parse(&src)?;

        let svg_doc = render::render(&ast);
        let svg_str = svg_doc.to_string();

        match &args.output {
            None => {
                io::stdout().write_all(svg_str.as_bytes())?;
            }
            Some(path) => {
                let out_path = if diagrams.len() > 1 {
                    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                    let ext = path
                        .extension()
                        .map(|e| format!(".{}", e.to_string_lossy()))
                        .unwrap_or_default();
                    path.with_file_name(format!("{}-{}{}", stem, i + 1, ext))
                } else {
                    path.clone()
                };
                std::fs::write(&out_path, svg_str.as_bytes())
                    .with_context(|| format!("writing {:?}", out_path))?;
                if args.verbose {
                    eprintln!("wrote {:?}", out_path);
                }
            }
        }
    }

    Ok(())
}
