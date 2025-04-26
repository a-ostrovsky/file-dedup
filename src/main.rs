use std::env;
use std::path::Path;

pub mod dedup;
use dedup::{find_duplicates, DedupOptions};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <folder> [--exclude-empty] [filter1] ... [filterN]", args[0]);
        eprintln!("Example: {} ./my_documents *.txt *.pdf", args[0]);
        eprintln!("This will scan the 'my_documents' folder for duplicate files with .txt or .pdf extensions");
        eprintln!("Options:");
        eprintln!("  --exclude-empty    Exclude files with zero size from duplicate search");
        std::process::exit(1);
    }

    let folder_path = &args[1];
    let mut filters: Vec<String> = Vec::new();
    let mut exclude_empty = false;
    
    // Parse arguments
    for arg in args[2..].iter() {
        if arg == "--exclude-empty" {
            exclude_empty = true;
        } else {
            filters.push(arg.clone());
        }
    }
    
    let options = DedupOptions {
        filters,
        exclude_empty,
        case_sensitive: true,
    };

    if let Err(e) = run(folder_path, &options) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(folder_path: &str, options: &DedupOptions) -> Result<(), String> {
    let path = Path::new(folder_path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("'{}' is not a valid directory", folder_path));
    }

    let duplicates = find_duplicates(path, options)?;
    
    if duplicates.groups.is_empty() {
        println!("No duplicate files found.");
        return Ok(());
    }

    println!("Found duplicate files (by size):");
    for (i, group) in duplicates.groups.iter().enumerate() {
        println!("\nGroup {}: {} files", i + 1, group.files.len());
        for file in &group.files {
            println!("  {}", file.path.display());
        }
    }

    Ok(())
}
