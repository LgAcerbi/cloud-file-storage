#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileBlobMetadata {
    pub size_bytes: u64,
    pub modified_at: u64,
}

pub trait FileBlobRepository {
    fn exists_file_blob_by_id(&self, file_id: &str) -> bool;
    fn get_file_blob_by_path(&self, file_path: &str) -> Option<Vec<u8>>;
    fn get_file_blob_metadata_by_path(&self, file_path: &str) -> Option<FileBlobMetadata>;
}
