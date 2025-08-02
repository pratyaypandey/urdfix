pub fn convert(file: &str, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("Converting: {}", file);
    }
    
    println!("Converting {}", file);
    Ok(())
} 