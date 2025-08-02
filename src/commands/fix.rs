pub fn fix(file: &str, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("Fixing: {}", file);
    }
    
    println!("Fixing {}", file);
    Ok(())
} 