# File Hashing Tool

A Python-based command-line tool for hashing files and verifying them later. This tool helps ensure file integrity by embedding a portion of the SHA256 hash directly into the filename.

## Features

- **Recursive Hashing**: Automatically processes all files in a directory and its subdirectories.
- **Filename Embedding**: Renames files to include their SHA256 hash (e.g., `document.pdf` becomes `document_SHA256_a1b2c3d4e5f6.pdf`).
- **Configurable Hash Length**: Specify how many characters of the hash to include in the filename (default is 12).
- **Verification Mode**: Easily verify the integrity of files by comparing their current hash against the one stored in their filename.
- **Smart Skipping**: Automatically skips files that already appear to have a hash in their name to avoid redundant processing.

## Installation

This tool uses only Python standard libraries, so no additional dependencies are required.

1. Ensure you have Python 3.x installed.
2. Clone or download this repository.

## Usage

Run the tool using `main.py`:

```bash
python main.py <path> [options]
```

### Arguments

- `path`: The path to a folder or a specific file to process.

### Options

- `--hash`: Calculate hashes and rename files.
- `--verify`: Verify the hashes of previously processed files.
- `--hash-length <int>`: (Optional) The number of characters of the hash to include in the filename (default: 12).
- `--hash-algorithm {sha256}`: (Optional) The hashing algorithm to use (currently only `sha256` is supported).

### Examples

**Hash all files in a directory:**
```bash
python main.py ./my_documents --hash
```

**Hash a single file with a specific hash length:**
```bash
python main.py ./image.png --hash --hash-length 8
```

**Verify files in a directory:**
```bash
python main.py ./my_documents --verify
```

## Project Structure

- `main.py`: The main entry point of the application.
- `src/`: Contains core logic modules.
  - `parse_args.py`: Handles command-line argument parsing.
  - `list_files.py`: Recursively lists files in a directory.
  - `list_folders.py`: Recursively lists subfolders.
