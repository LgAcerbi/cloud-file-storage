use crate::domain::entities::file_metadata::FileMetadata;

pub trait FileMetadataRepository {
    fn get_file_metadata_by_id(&self, id: &str) -> Option<FileMetadata>;
    fn get_file_metadata_by_path(&self, file_path: &str) -> Option<FileMetadata>;
}
