pub fn diff(file1: &str, file2: &str, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("Comparing: {} and {}", file1, file2);
    }
    
    println!("Comparing {} and {}", file1, file2);
    Ok(())
} 