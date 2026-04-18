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

    /// Force diagram type (sequence|class|activity|state)
    #[arg(short = 't', long, value_name = "TYPE")]
    pub r#type: Option<String>,

    /// Show parse and layout debug info
    #[arg(short, long)]
    pub verbose: bool,
}
