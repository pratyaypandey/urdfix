use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "urdfix",
    about = "🦾 A fast, Rust-powered CLI for linting, formatting, and fixing URDF robot descriptions.",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    Lint {
        #[arg(value_name = "FILE")]
        file: String,
    },
    Fix {
        #[arg(value_name = "FILE")]
        file: String,
    },
    Format {
        #[arg(value_name = "FILE")]
        file: String,
    },
    Analyze {
        #[arg(value_name = "FILE")]
        file: String,
    },
    Convert {
        #[arg(value_name = "FILE")]
        file: String,
    },
    Diff {
        #[arg(value_name = "FILE1")]
        file1: String,
        #[arg(value_name = "FILE2")]
        file2: String,
    },
} 