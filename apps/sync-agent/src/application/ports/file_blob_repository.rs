pub trait FileBlobRepository {
    fn exists_by_file_id(&self, file_id: &str) -> bool;
}
