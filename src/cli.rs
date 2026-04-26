use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "puml",
    about = "PlantUML-compatible diagram generator (native SVG, no Java)"
)]
#[command(version)]
pub struct Args {
    /// Subcommand. When omitted, `puml` renders the input file as before.
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Input .puml file (reads from stdin if omitted)
    #[arg(value_name = "INPUT")]
    pub input: Option<std::path::PathBuf>,

    /// Output file. Format inferred from extension: `.png` rasterises via
    /// resvg, anything else writes the SVG. With no `--output`, SVG goes to
    /// stdout.
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<std::path::PathBuf>,

    /// PNG scale factor — applied only when the output extension is `.png`.
    /// Defaults to 2 so the rasterised PNG looks sharp on a retina display
    /// without the user thinking about DPI; pass 1 for a 1:1 pixel output.
    #[arg(long, value_name = "N", default_value_t = 2.0)]
    pub scale: f32,

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

    /// Re-render whenever the input file (or any file it `!include`s) changes.
    /// Requires `--output` so the result has somewhere to land. Press Ctrl+C
    /// to stop.
    #[arg(short, long)]
    pub watch: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Drop AI-author instruction templates into a project so downstream AI
    /// tools (Codex, Cursor, Copilot, Claude, Windsurf, …) all pick up
    /// consistent guidance about how to author `.puml` diagrams here.
    InitGenai(InitGenaiArgs),
}

#[derive(clap::Args, Debug)]
pub struct InitGenaiArgs {
    /// Target directory (default: current directory).
    #[arg(value_name = "DIR")]
    pub dir: Option<std::path::PathBuf>,

    /// Overwrite files that already exist. Without this flag, existing
    /// files are left untouched and reported as skipped.
    #[arg(short, long)]
    pub force: bool,
}
