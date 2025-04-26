use anyhow::Result;
use std::env;
use std::path::Path;

mod dedup;
mod file_iter;
mod types;
use dedup::find_duplicates;

use crate::types::DedupOptions;

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} bytes", size)
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} <folder> [--exclude-empty] [--size-only] [--case-insensitive] [filter1] ... [filterN]",
            args[0]
        );
        eprintln!("Example: {} ./my_documents *.txt *.pdf", args[0]);
        eprintln!(
            "This will scan the 'my_documents' folder for duplicate files with .txt or .pdf extensions"
        );
        eprintln!("Options:");
        eprintln!("  --exclude-empty      Exclude files with zero size from duplicate search");
        eprintln!("  --size-only          Compare files only by size, not content");
        eprintln!("  --case-insensitive   Use case-insensitive filter matching");
        std::process::exit(1);
    }

    let folder_path = &args[1];
    let mut filters: Vec<&str> = Vec::new();
    let mut exclude_empty = false;
    let mut size_only = false;
    let mut case_sensitive = true;

    // Parse arguments
    for arg in args[2..].iter() {
        match arg.as_str() {
            "--exclude-empty" => exclude_empty = true,
            "--size-only" => size_only = true,
            "--case-insensitive" => case_sensitive = false,
            _ => filters.push(arg),
        }
    }

    let options = DedupOptions {
        filters: &filters,
        exclude_empty,
        case_sensitive,
        only_compare_file_size: size_only,
    };

    run(folder_path, &options)
}

fn run(folder_path: &str, options: &DedupOptions) -> Result<()> {
    let path = Path::new(folder_path);
    if !path.exists() || !path.is_dir() {
        anyhow::bail!("'{}' is not a valid directory", folder_path);
    }

    let duplicates = find_duplicates(path, options)?;

    if duplicates.groups.is_empty() {
        println!("No duplicate files found.");
        return Ok(());
    }

    println!(
        "Found duplicate files{}:",
        if options.only_compare_file_size {
            " (by size only)"
        } else {
            ""
        }
    );
    for group in duplicates.groups {
        let size = group.files[0].metadata.len();
        println!(
            "\nGroup: {} files of size {}",
            group.files.len(),
            format_size(size)
        );
        for file in &group.files {
            println!("  {}", file.path.display());
        }
    }

    Ok(())
}
