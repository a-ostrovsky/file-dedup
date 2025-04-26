use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fs::{self, Metadata, ReadDir};
use std::iter::Peekable;
use std::path::{Path, PathBuf};
use std::str::Chars;
use anyhow::{Context, Result};

pub struct FileInfo {
    pub path: PathBuf,
    pub metadata: Metadata,
}

#[derive(Debug, Clone)]
pub struct FilterOptions<'a> {
    pub filters: &'a [&'a str],
    pub case_sensitive: bool,
    pub exclude_empty: bool,
}

pub struct FileIter<'a> {
    queue: VecDeque<PathBuf>,
    read_dir: Option<ReadDir>,
    options: FilterOptions<'a>,
}

impl<'a> FileIter<'a> {
    pub fn new(dir: &Path, options: FilterOptions<'a>) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back(dir.to_path_buf());
        FileIter {
            queue,
            read_dir: None,
            options,
        }
    }
}

impl<'a> Iterator for FileIter<'a> {
    type Item = Result<FileInfo>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.queue.is_empty() || self.read_dir.is_some() {
            if let Some(read_dir) = &mut self.read_dir {
                for entry in read_dir {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(e) => return Some(Err(e).context("Failed to read directory entry")),
                    };
                    let path = entry.path();

                    if path.is_dir() {
                        self.queue.push_back(path);
                        continue;
                    }

                    if !path.is_file()
                        || !matches_filters(
                            &path,
                            &self.options.filters,
                            self.options.case_sensitive,
                        )
                    {
                        continue;
                    }

                    let metadata = match path.metadata() {
                        Ok(metadata) => metadata,
                        Err(e) => return Some(Err(e).context(format!("Failed to get metadata for {}", path.display()))),
                    };

                    if self.options.exclude_empty && metadata.len() == 0 {
                        continue;
                    }

                    return Some(Ok(FileInfo { path, metadata }));
                }
                self.read_dir = None; // Finished reading this directory
            }
            if let Some(next_dir) = self.queue.pop_front() {
                match fs::read_dir(&next_dir) {
                    Ok(dir) => self.read_dir = Some(dir),
                    Err(e) => return Some(Err(e).context(format!("Failed to read directory {}", next_dir.display()))),
                }
            } else {
                return None; // No more files to process
            }
        }
        None
    }
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

fn matches_filters(path: &Path, filters: &[&str], case_sensitive: bool) -> bool {
    if filters.is_empty() || filters.contains(&"*") {
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
            &["*test*"],
            true
        ));
        assert!(!matches_filters(
            Path::new("c:\\temp\\test.txt"),
            &["nonexistent"],
            true
        ));
        assert!(matches_filters(
            Path::new("/home/user/test.txt"),
            &["test*"],
            true
        ));
        assert!(matches_filters(
            Path::new("/home/user/test.txt"),
            &["*.txt"],
            true
        ));
    }

    #[test]
    fn test_file_iterator() {
        let temp_dir = tempdir().unwrap();

        // Create two files
        let file1_path = temp_dir.path().join("file1.txt");
        let file2_path = temp_dir.path().join("file2.txt");

        File::create(&file1_path).unwrap();
        File::create(&file2_path).unwrap();

        let options = FilterOptions {
            filters: &[],
            case_sensitive: true,
            exclude_empty: false,
        };

        let iterator = FileIter::new(temp_dir.path(), options);
        let files: Vec<_> = iterator.collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(files.len(), 2);

        let paths: Vec<_> = files
            .iter()
            .map(|f| f.path.to_string_lossy().to_string())
            .collect();
        assert!(paths.contains(&file1_path.to_string_lossy().to_string()));
        assert!(paths.contains(&file2_path.to_string_lossy().to_string()));
    }
}
