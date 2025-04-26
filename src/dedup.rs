use std::{collections::HashMap, fs::File, io::Read, path::Path};

use super::file_iter::{FileIter, FilterOptions};
use super::types::{DedupOptions, DuplicateFiles, DuplicateGroup, FileInfo};

pub fn find_duplicates(
    folder_path: &Path,
    options: &DedupOptions,
) -> Result<DuplicateFiles, String> {
    let filter_options = FilterOptions {
        filters: options.filters,
        case_sensitive: options.case_sensitive,
        exclude_empty: options.exclude_empty,
    };
    let duplicate_files = find_duplicate_files_by_size(folder_path, filter_options)?;
    if options.only_compare_file_size {
        return Ok(duplicate_files);
    }
    find_duplicate_files_by_hash(duplicate_files)
}

fn find_duplicate_files_by_hash(duplicate_files: DuplicateFiles) -> Result<DuplicateFiles, String> {
    let mut result_groups = Vec::new();

    // Process each group of files with the same size
    for group in duplicate_files.groups {
        // Skip groups with only one file
        if group.files.len() <= 1 {
            continue;
        }

        let mut hash_map = HashMap::new();

        for file_info in group.files {
            let hash = calculate_file_hash(&file_info.path)?;
            hash_map
                .entry(hash)
                .or_insert_with(Vec::new)
                .push(file_info);
        }

        // Filter out hash groups with only one file
        hash_map.retain(|_, files| files.len() > 1);

        for (_, files) in hash_map {
            result_groups.push(DuplicateGroup { files });
        }
    }

    Ok(DuplicateFiles {
        groups: result_groups,
    })
}

fn calculate_file_hash(path: &Path) -> Result<u64, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut hash: u64 = 0;

    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        if bytes_read == 0 {
            break;
        }

        for &byte in &buffer[..bytes_read] {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
    }

    Ok(hash)
}

fn find_duplicate_files_by_size(
    folder_path: &Path,
    filter_options: FilterOptions,
) -> Result<DuplicateFiles, String> {
    let file_iter = FileIter::new(folder_path, filter_options);
    let mut size_map = HashMap::new();
    for file_result in file_iter {
        let file_info = file_result?;
        let size = file_info.metadata.len();
        size_map
            .entry(size)
            .or_insert_with(Vec::new)
            .push(FileInfo {
                path: file_info.path,
                metadata: file_info.metadata,
            });
    }

    // Remove entries with only one file
    size_map.retain(|_, files| files.len() > 1);

    let groups = size_map
        .into_iter()
        .map(|(_, files)| DuplicateGroup { files })
        .collect();

    Ok(DuplicateFiles { groups })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_find_duplicates() {
        let temp_dir = tempdir().unwrap();

        // Create two files with the same size
        let content = b"Hello, World!";
        let file1_path = temp_dir.path().join("file1.txt");
        let file2_path = temp_dir.path().join("file2.txt");

        let mut file1 = File::create(&file1_path).unwrap();
        let mut file2 = File::create(&file2_path).unwrap();

        file1.write_all(content).unwrap();
        file2.write_all(content).unwrap();

        let options = DedupOptions {
            filters: &[],
            exclude_empty: false,
            case_sensitive: true,
            only_compare_file_size: false,
        };
        let duplicates = find_duplicates(temp_dir.path(), &options).unwrap();

        assert_eq!(duplicates.groups.len(), 1);
        assert_eq!(duplicates.groups[0].files.len(), 2);

        // Verify file paths
        let paths: Vec<_> = duplicates.groups[0]
            .files
            .iter()
            .map(|f| f.path.to_string_lossy().to_string())
            .collect();
        assert!(paths.contains(&file1_path.to_string_lossy().to_string()));
        assert!(paths.contains(&file2_path.to_string_lossy().to_string()));
    }

    #[test]
    fn test_empty_files_handling() {
        let temp_dir = tempdir().unwrap();

        // Create two empty files
        let file1_path = temp_dir.path().join("empty1.txt");
        let file2_path = temp_dir.path().join("empty2.txt");
        File::create(&file1_path).unwrap();
        File::create(&file2_path).unwrap();

        // Create a non-empty file
        let content = b"Not empty";
        let file3_path = temp_dir.path().join("non_empty.txt");
        let mut file3 = File::create(&file3_path).unwrap();
        file3.write_all(content).unwrap();

        // Test with exclude empty
        let options = DedupOptions {
            filters: &[],
            exclude_empty: true,
            case_sensitive: true,
            only_compare_file_size: false,
        };
        let duplicates = find_duplicates(temp_dir.path(), &options).unwrap();
        assert_eq!(duplicates.groups.len(), 0); // No duplicates found

        // Test with include empty
        let options = DedupOptions {
            filters: &[],
            exclude_empty: false,
            case_sensitive: true,
            only_compare_file_size: false,
        };
        let duplicates = find_duplicates(temp_dir.path(), &options).unwrap();
        assert_eq!(duplicates.groups.len(), 1); // Empty files are considered duplicates
        assert_eq!(duplicates.groups[0].files.len(), 2); // Two empty files

        // Verify file paths
        let paths: Vec<_> = duplicates.groups[0]
            .files
            .iter()
            .map(|f| f.path.to_string_lossy().to_string())
            .collect();
        assert!(paths.contains(&file1_path.to_string_lossy().to_string()));
        assert!(paths.contains(&file2_path.to_string_lossy().to_string()));
    }

    #[test]
    fn test_hash_based_detection() {
        let temp_dir = tempdir().unwrap();

        // Create two files with different content but same size
        let content1 = b"Hello, World !";
        let content2 = b"Hello, World ?";
        let file1_path = temp_dir.path().join("file1.txt");
        let file2_path = temp_dir.path().join("file2.txt");

        let mut file1 = File::create(&file1_path).unwrap();
        let mut file2 = File::create(&file2_path).unwrap();

        file1.write_all(content1).unwrap();
        file2.write_all(content2).unwrap();

        let options = DedupOptions {
            filters: &[],
            exclude_empty: false,
            case_sensitive: true,
            only_compare_file_size: false,
        };
        let duplicates = find_duplicates(temp_dir.path(), &options).unwrap();

        // Verify no duplicates found since content is different
        assert_eq!(duplicates.groups.len(), 0);
    }
}
