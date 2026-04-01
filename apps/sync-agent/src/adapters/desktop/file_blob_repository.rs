use crate::application::ports::file_blob_repository::{FileBlobMetadata, FileBlobRepository};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::time::UNIX_EPOCH;

/// Filesystem-backed [`FileBlobRepository`] for Windows, Linux, and macOS using `std::fs`.
#[derive(Debug, Default, Clone, Copy)]
pub struct DesktopFileBlobRepository;

impl DesktopFileBlobRepository {
    fn normalize_path(file_path: &str) -> Option<String> {
        let trimmed = file_path.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(trimmed.replace('\\', "/"))
    }

    fn metadata_for_path(path: &Path) -> Option<FileBlobMetadata> {
        let metadata = std::fs::metadata(path).ok()?;
        let modified_at = metadata
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();
        Some(FileBlobMetadata {
            size_bytes: metadata.len(),
            modified_at,
        })
    }

    fn hash_file(path: &Path) -> Option<String> {
        let mut file = File::open(path).ok()?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            };
            hasher.update(&buf[..n]);
        }
        Some(hex::encode(hasher.finalize()))
    }
}

impl FileBlobRepository for DesktopFileBlobRepository {
    fn get_file_blob_by_path(&self, file_path: &str) -> Option<Vec<u8>> {
        let normalized = Self::normalize_path(file_path)?;
        std::fs::read(Path::new(&normalized)).ok()
    }

    fn get_file_blob_metadata_by_path(&self, file_path: &str) -> Option<FileBlobMetadata> {
        let normalized = Self::normalize_path(file_path)?;
        Self::metadata_for_path(Path::new(&normalized))
    }

    fn get_file_blob_hash_by_path(&self, file_path: &str) -> Option<String> {
        let normalized = Self::normalize_path(file_path)?;
        Self::hash_file(Path::new(&normalized))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_metadata_hash_and_bytes_for_temp_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sample.txt");
        let contents = b"hello blob";
        std::fs::write(&path, contents).expect("write");

        let path_str = path.to_string_lossy().replace('\\', "/");
        let repo = DesktopFileBlobRepository;

        let meta = repo
            .get_file_blob_metadata_by_path(&path_str)
            .expect("metadata");
        assert_eq!(meta.size_bytes, contents.len() as u64);

        let bytes = repo
            .get_file_blob_by_path(&path_str)
            .expect("read bytes");
        assert_eq!(bytes, contents);

        let hash = repo
            .get_file_blob_hash_by_path(&path_str)
            .expect("hash");
        assert_eq!(
            hash,
            "e997afd18e5f6be004fc193aed2c90291e68ab2c7599a62538c935b7fca6ab0f"
        );
    }

    #[test]
    fn missing_path_returns_none() {
        let repo = DesktopFileBlobRepository;
        assert!(repo.get_file_blob_by_path("/nonexistent/path/xyz").is_none());
        assert!(repo
            .get_file_blob_metadata_by_path("/nonexistent/path/xyz")
            .is_none());
        assert!(repo
            .get_file_blob_hash_by_path("/nonexistent/path/xyz")
            .is_none());
    }

    #[test]
    fn empty_trimmed_path_returns_none() {
        let repo = DesktopFileBlobRepository;
        assert!(repo.get_file_blob_by_path("   ").is_none());
    }
}
