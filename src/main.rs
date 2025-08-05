use std::process;

mod cli;
mod commands;
mod utils;

use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match &cli.command {
        Some(Commands::Lint { file }) => commands::lint(file, cli.verbose),
        Some(Commands::Fix { file }) => commands::fix(file, cli.verbose),
        Some(Commands::Format { file }) => commands::format(file, cli.verbose),
        Some(Commands::Analyze { file }) => commands::analyze(file, cli.verbose),
        Some(Commands::Convert { file }) => commands::convert(file, cli.verbose),
        Some(Commands::Diff { file1, file2 }) => commands::diff(file1, file2, cli.verbose),
        None => {
            println!("No command specified. Use --help for usage information.");
            println!("\nExamples:");
            println!("  urdfix lint robot.urdf");
            println!("  urdfix fix robot.urdf");
            println!("  urdfix format robot.urdf");
            println!("  urdfix analyze robot.urdf");
            println!("  urdfix convert robot.urdf");
            println!("  urdfix diff robot1.urdf robot2.urdf");
            Ok(())
        }
    }
} 