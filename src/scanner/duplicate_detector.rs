//! Duplicate file detection using SHA-256 hashing

use anyhow::Result;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Information about a duplicate file
#[derive(Debug, Clone)]
pub struct DuplicateInfo {
    pub hash: String,
    pub paths: Vec<PathBuf>,
}

/// Compute SHA-256 hash of a file
///
/// # Arguments
/// * `path` - Path to the file
///
/// # Returns
/// Hex-encoded SHA-256 hash string
pub fn compute_file_hash(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Find duplicate files in a list of paths
///
/// # Arguments
/// * `paths` - List of file paths to check
///
/// # Returns
/// Vector of DuplicateInfo containing files with identical hashes
pub fn find_duplicates(paths: &[PathBuf]) -> Result<Vec<DuplicateInfo>> {
    let mut hash_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for path in paths {
        if let Ok(hash) = compute_file_hash(path) {
            hash_map.entry(hash).or_insert_with(Vec::new).push(path.clone());
        }
    }

    let duplicates: Vec<DuplicateInfo> = hash_map
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(hash, paths)| DuplicateInfo { hash, paths })
        .collect();

    Ok(duplicates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_file_hash() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();

        let hash = compute_file_hash(temp_file.path()).unwrap();
        // SHA-256 of "test content"
        assert_eq!(hash, "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72");
    }

    #[test]
    fn test_find_duplicates() {
        let mut file1 = NamedTempFile::new().unwrap();
        let mut file2 = NamedTempFile::new().unwrap();
        let mut file3 = NamedTempFile::new().unwrap();

        file1.write_all(b"same content").unwrap();
        file2.write_all(b"same content").unwrap();
        file3.write_all(b"different content").unwrap();

        let paths = vec![
            file1.path().to_path_buf(),
            file2.path().to_path_buf(),
            file3.path().to_path_buf(),
        ];

        let duplicates = find_duplicates(&paths).unwrap();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].paths.len(), 2);
    }
}
