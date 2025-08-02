pub fn format(file: &str, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("Formatting: {}", file);
    }
    
    println!("Formatting {}", file);
    Ok(())
} 