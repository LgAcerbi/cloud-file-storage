pub trait FileBlobRepository {
    fn exists_file_blob_by_id(&self, file_id: &str) -> bool;
    fn get_file_blob_by_id(&self, file_id: &str) -> Option<Vec<u8>>;
}
