use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "puml",
    about = "PlantUML-compatible diagram generator (native SVG, no Java)"
)]
#[command(version)]
pub struct Args {
    /// Input .puml file (reads from stdin if omitted)
    #[arg(value_name = "INPUT")]
    pub input: Option<std::path::PathBuf>,

    /// Output SVG file (writes to stdout if omitted)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<std::path::PathBuf>,

    /// Force diagram type (sequence|class|activity|state|usecase|timing|mindmap|gantt)
    #[arg(short = 't', long, value_name = "TYPE")]
    pub r#type: Option<String>,

    /// Built-in theme preset: `light` (default), `dark`, `auto` (adapts to the
    /// viewer's `prefers-color-scheme` via CSS media queries), or any other
    /// named preset the renderer knows about (`plain`, `amiga`).
    #[arg(long, value_name = "THEME")]
    pub theme: Option<String>,

    /// Show parse and layout debug info
    #[arg(short, long)]
    pub verbose: bool,
}
