# file-dedup

[![Build](https://github.com/a-ostrovsky/file-dedup/actions/workflows/build.yml/badge.svg)](https://github.com/a-ostrovsky/file-dedup/actions/workflows/build.yml)

A command-line tool for finding duplicate files in your system.

## Features

- Find files with identical sizes and contents
- Filter by file extensions
- Option to exclude empty files

## Installation

```bash
# Clone the repository
git clone https://github.com/a-ostrovsky/file-dedup.git
cd file-dedup

# Build the project
cargo build --release
```

## Usage

```bash
# Basic usage
./target/release/file-dedup <folder>

# With filters for specific file types
./target/release/file-dedup <folder> *.txt *.pdf
```

## Example

```bash
./target/release/file-dedup ./my_documents *.txt *.pdf
```

This will scan the 'my_documents' folder for duplicate files with .txt or .pdf extensions.

## License

MIT 