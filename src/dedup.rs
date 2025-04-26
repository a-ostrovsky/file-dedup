use std::collections::{HashMap, VecDeque};
use std::ffi::OsStr;
use std::fs;
use std::iter::Peekable;
use std::path::{Path, PathBuf};
use std::str::Chars;

pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct DedupOptions {
    pub filters: Vec<String>,
    pub exclude_empty: bool,
    pub case_sensitive: bool,
}

pub struct DuplicateGroup {
    pub files: Vec<FileInfo>,
}

pub struct DuplicateFiles {
    pub groups: Vec<DuplicateGroup>,
}

pub fn find_duplicates(
    folder_path: &Path,
    options: &DedupOptions,
) -> Result<DuplicateFiles, String> {
    let mut size_map = scan_directory(folder_path, options)?;

    // Remove entries with only one file (no duplicates)
    size_map.retain(|_, files| files.len() > 1);

    let groups = size_map
        .into_iter()
        .map(|(_, files)| DuplicateGroup { files })
        .collect();

    Ok(DuplicateFiles { groups })
}

fn scan_directory(
    dir: &Path,
    options: &DedupOptions,
) -> Result<HashMap<u64, Vec<FileInfo>>, String> {
    let mut size_map: HashMap<u64, Vec<FileInfo>> = HashMap::new();

    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(dir.to_path_buf());

    while let Some(current_dir) = queue.pop_front() {
        let entries = match fs::read_dir(&current_dir) {
            Ok(entries) => entries,
            Err(e) => return Err(format!("Failed to read directory: {}", e)),
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => return Err(format!("Failed to read directory entry: {}", e)),
            };

            let path = entry.path();

            if path.is_dir() {
                queue.push_back(path);
                continue;
            }

            if !path.is_file() || !matches_filters(&path, &options.filters, options.case_sensitive)
            {
                continue;
            }

            let metadata = match path.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue, // Skip files we can't get metadata for
            };

            let size = metadata.len();

            if size == 0 && options.exclude_empty {
                continue;
            }

            let file_info = FileInfo {
                path: path.clone(),
                size,
            };
            size_map
                .entry(size)
                .or_insert_with(Vec::new)
                .push(file_info);
        }
    }

    Ok(size_map)
}

// Verifies that the file matches the filter which may contain wildcards.
// E.g. "*.txt" will match "file.txt" and "file2.txt" but not "file.docx".
// *.a?b will match a.acb or a.aab but not a.a_something_b
fn matches_filter(path: &Path, filter: &str, case_sensitive: bool) -> bool {
    if filter.is_empty() || filter == "*" {
        return true;
    }

    let chars_eq = |a: &char, b: &char| -> bool {
        if case_sensitive {
            a == b
        } else {
            a.eq_ignore_ascii_case(&b)
        }
    };

    let file_name = path.file_name().unwrap_or(OsStr::new("")).to_string_lossy();

    let mut filter_iter = filter.chars().peekable();
    let mut file_name_iter = file_name.chars().peekable();

    let mut star_filter_iter: Option<Peekable<Chars>> = None;
    let mut star_file_name_iter: Peekable<Chars> = file_name_iter.clone();

    while let Some(file_name_char) = file_name_iter.peek() {
        let filter_char = filter_iter.peek();
        if filter_char
            .is_some_and(|filter_char| filter_char == &'?' || chars_eq(filter_char, file_name_char))
        {
            filter_iter.next();
            file_name_iter.next();
        } else if filter_char.is_some_and(|filter_char| filter_char == &'*') {
            star_filter_iter = Some(filter_iter.clone());
            star_file_name_iter = file_name_iter.clone();
            filter_iter.next();
        } else if let Some(star_filter_iter) = star_filter_iter.clone() {
            filter_iter = star_filter_iter;
            star_file_name_iter.next();
            file_name_iter = star_file_name_iter.clone();
        } else {
            return false;
        }
    }

    let remaining_all_stars = filter_iter.all(|f| return f == '*');
    remaining_all_stars
}

fn matches_filters(path: &Path, filters: &[String], case_sensitive: bool) -> bool {
    if filters.is_empty() || filters.contains(&"*".to_string()) {
        return true;
    }

    return filters
        .iter()
        .any(|filter| matches_filter(path, filter, case_sensitive));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_matches_filter() {
        assert!(matches_filter(Path::new("a.txt"), "*", true));
        assert!(matches_filter(Path::new("a.txt"), "?.???", true));
        assert!(!matches_filter(Path::new("a.txt"), "?.??", false));
        assert!(matches_filter(Path::new("a.txt"), "*.*?", true));
        assert!(!matches_filter(Path::new("a"), "aa", false));
        assert!(!matches_filter(Path::new("A"), "a", true));
        assert!(matches_filter(Path::new("A"), "***********", true));
    }

    #[test]
    fn test_matches_filters() {
        assert!(matches_filters(Path::new("c:\\temp\\test.txt"), &[], true));
        assert!(matches_filters(
            Path::new("c:\\temp\\test.txt"),
            &["*test*".to_string()],
            true
        ));
        assert!(!matches_filters(
            Path::new("c:\\temp\\test.txt"),
            &["nonexistent".to_string()],
            true
        ));
        assert!(matches_filters(
            Path::new("/home/user/test.txt"),
            &["test*".to_string()],
            true
        ));
        assert!(matches_filters(
            Path::new("/home/user/test.txt"),
            &["*.txt".to_string()],
            true
        ));
    }

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
            filters: Vec::new(),
            exclude_empty: false,
            case_sensitive: true,
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
            filters: Vec::new(),
            exclude_empty: true,
            case_sensitive: true,
        };
        let duplicates = find_duplicates(temp_dir.path(), &options).unwrap();
        assert_eq!(duplicates.groups.len(), 0); // No duplicates found

        // Test with include empty
        let options = DedupOptions {
            filters: Vec::new(),
            exclude_empty: false,
            case_sensitive: true,
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
}
