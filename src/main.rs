mod ast;
mod cli;
mod error;
mod init_genai;
mod layout;
mod parser;
mod render;
mod watch;

use anyhow::{anyhow, Context};
use clap::Parser as ClapParser;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    if let Some(cmd) = &args.command {
        return run_subcommand(cmd);
    }

    if args.watch {
        let input = args
            .input
            .as_ref()
            .ok_or_else(|| anyhow!("--watch requires an input file (cannot watch stdin)"))?;
        let output = args
            .output
            .as_ref()
            .ok_or_else(|| anyhow!("--watch requires --output (cannot watch with stdout sink)"))?;
        return watch::run(input, output, &args);
    }

    let (source, base_dir) = read_input(args.input.as_deref())?;
    render_once(&source, base_dir.as_deref(), args.output.as_deref(), &args).map(|_includes| ())
}

fn run_subcommand(cmd: &cli::Command) -> anyhow::Result<()> {
    match cmd {
        cli::Command::InitGenai(args) => {
            let cwd = std::env::current_dir().context("reading current directory")?;
            let dir = args.dir.clone().unwrap_or(cwd);
            let report = init_genai::run(&dir, args.force)?;
            init_genai::print_report(&report, args.force);
            Ok(())
        }
    }
}

fn read_input(input: Option<&Path>) -> anyhow::Result<(String, Option<PathBuf>)> {
    match input {
        Some(path) => {
            let src =
                std::fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
            let dir = path.parent().map(|p| p.to_path_buf());
            Ok((src, dir))
        }
        None => {
            let mut src = String::new();
            io::stdin()
                .read_to_string(&mut src)
                .context("reading stdin")?;
            Ok((src, None))
        }
    }
}

/// Run the preprocess → parse → layout → render pipeline once, writing to
/// `output` (or stdout when `None`). Returns the list of files reached via
/// `!include` so the watcher can subscribe to them on the next tick.
pub(crate) fn render_once(
    source: &str,
    base_dir: Option<&Path>,
    output: Option<&Path>,
    args: &cli::Args,
) -> anyhow::Result<Vec<PathBuf>> {
    let (diagrams, includes) = parser::preprocessor::preprocess_with_includes(source, base_dir);

    if diagrams.is_empty() {
        eprintln!("puml: warning: no diagram content found");
        return Ok(includes);
    }

    for (i, diagram_src) in diagrams.iter().enumerate() {
        let mut src = diagram_src.clone();
        if let Some(ref t) = args.r#type {
            src.type_hint = Some(t.clone());
        }
        if let Some(ref t) = args.theme {
            src.content = format!("skinparam theme {}\n{}", t, src.content);
        }

        if args.verbose {
            eprintln!("--- diagram {} source ---\n{}", i + 1, src.content);
        }

        let ast = parser::parse(&src)?;
        let svg_doc = render::render(&ast);
        let svg_str = svg_doc.to_string();

        match output {
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
                    path.to_path_buf()
                };
                let is_png = out_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("png"))
                    .unwrap_or(false);
                if is_png {
                    let png = render::png::svg_to_png(&svg_str, args.scale)
                        .with_context(|| format!("rasterising {:?}", out_path))?;
                    std::fs::write(&out_path, &png)
                        .with_context(|| format!("writing {:?}", out_path))?;
                } else {
                    std::fs::write(&out_path, svg_str.as_bytes())
                        .with_context(|| format!("writing {:?}", out_path))?;
                }
                if args.verbose {
                    eprintln!("wrote {:?}", out_path);
                }
            }
        }
    }

    Ok(includes)
}
