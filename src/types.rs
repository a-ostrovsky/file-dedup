use std::path::PathBuf;
use std::fs::Metadata;
pub struct FileInfo {
    pub path: PathBuf,
    pub metadata: Metadata,
}

pub struct DuplicateGroup {
    pub files: Vec<FileInfo>,
}

pub struct DuplicateFiles {
    pub groups: Vec<DuplicateGroup>,
} 

#[derive(Debug, Clone)]
pub struct DedupOptions<'a> {
    pub filters: &'a [&'a str],
    pub exclude_empty: bool,
    pub case_sensitive: bool,
    pub only_compare_file_size: bool,
}