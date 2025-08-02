use crate::cli::OutputFormat;

pub fn lint(file: &str, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("Linting: {}", file);
    }
    
    println!("Linting {}", file);
    Ok(())
} 