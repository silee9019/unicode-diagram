use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "unid", about = "Unicode box-drawing diagram renderer")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Collision mode (overrides DSL declaration)
    #[arg(long, value_enum)]
    pub collision: Option<CollisionMode>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List objects in a diagram (stdin)
    List,

    /// Show comprehensive usage guide with examples
    Guide,

    /// Lint DSL input for errors and warnings (stdin)
    Lint,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CollisionMode {
    On,
    Off,
}
