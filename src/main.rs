use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use regex::Regex;
use std::sync::OnceLock;

static HASH_REGEX: OnceLock<Regex> = OnceLock::new();

#[derive(Parser, Debug)]
#[command(author, version, about = "File Hashing Tool", long_about = None)]
struct Args {
    /// The folder- or filepath to perform operations in
    path: PathBuf,

    /// Calculate hashes and rename files
    #[arg(long, group = "action", required = true)]
    hash: bool,

    /// Verify the hashes of previously processed files
    #[arg(long, group = "action", required = true)]
    verify: bool,

    /// The number of characters of the hash to include in the filename
    #[arg(long, default_value_t = 12)]
    hash_length: usize,

    /// The hashing algorithm to use
    #[arg(long, value_enum, default_value_t = HashAlgorithm::Sha256)]
    hash_algorithm: HashAlgorithm,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum HashAlgorithm {
    Sha256,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.hash {
        run_hash(&args)?;
    } else if args.verify {
        run_verify(&args)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_extract_hash() {
        assert_eq!(extract_hash(Path::new("file_SHA256_A1B2C3D4E5F6.txt")), Some("A1B2C3D4E5F6".to_string()));
        assert_eq!(extract_hash(Path::new("file.txt")), None);
        assert_eq!(extract_hash(Path::new("file_SHA256_123.txt")), None); // too short
        assert_eq!(extract_hash(Path::new("file_SHA256_A1B2C3D4E5F6G7.txt")), None); // invalid hex
    }
}

fn get_files(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }

    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn calc_sha256(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

fn extract_hash(path: &Path) -> Option<String> {
    let filename = path.file_stem()?.to_str()?;
    // Pattern looking for _SHA256_ followed by hex characters at the end of file stem
    let re = HASH_REGEX.get_or_init(|| Regex::new(r"_SHA256_([a-fA-F0-9]{6,})$").unwrap());
    if let Some(caps) = re.captures(filename) {
        return Some(caps.get(1)?.as_str().to_string());
    }
    None
}

fn run_hash(args: &Args) -> Result<()> {
    println!("** HASH **");
    let files = get_files(&args.path);

    for file in files {
        if let Some(_existing_hash) = extract_hash(&file) {
            if args.verbose {
                println!("Skipping existing file: {}", file.display());
            }
            continue;
        }

        let file_hash = calc_sha256(&file)?.to_uppercase();
        let truncated_hash = &file_hash[..args.hash_length.min(file_hash.len())];

        let stem = file.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid filename: {}", file.display()))?;
        let extension = file.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();

        let new_filename = format!("{}_SHA256_{}{}", stem, truncated_hash, extension);
        let mut new_path = file.clone();
        new_path.set_file_name(new_filename);

        if args.verbose {
            println!("hash: {} - file: {}", file_hash, file.display());
        }

        fs::rename(&file, &new_path)
            .with_context(|| format!("Failed to rename {} to {}", file.display(), new_path.display()))?;
    }

    Ok(())
}

fn run_verify(args: &Args) -> Result<()> {
    println!("** VERIFY **");
    let files = get_files(&args.path);

    for file in files {
        if let Some(extracted_hash) = extract_hash(&file) {
            let actual_hash = calc_sha256(&file)?;
            let actual_truncated = &actual_hash[..extracted_hash.len().min(actual_hash.len())];

            if extracted_hash.to_lowercase() == actual_truncated.to_lowercase() {
                println!("all_good - file: {}", file.display());
            } else {
                println!(
                    "HASH MISMATCH!!! from name: {} - actual: {} (file: \"{}\")",
                    extracted_hash, actual_truncated, file.display()
                );
            }
        } else if args.verbose {
            println!("Skipping file without hash: {}", file.display());
        }
    }

    Ok(())
}
