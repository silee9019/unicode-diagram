use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "unid", about = "Unicode box-drawing diagram renderer")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List objects in a diagram (stdin)
    List,

    /// Lint DSL input for errors and warnings (stdin)
    Lint,

    /// Show comprehensive usage guide with examples
    Guide,
}
