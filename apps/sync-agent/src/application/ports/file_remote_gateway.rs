use crate::domain::entities::file_metadata::FileMetadata;

pub trait FileRemoteGateway {
    fn get_file_metadata_by_id(&self, id: &str) -> Option<FileMetadata>;
    fn upload_file(&self, file_blob: Vec<u8>) -> FileMetadata;
    fn update_file(&self, id: &str, file_blob: Vec<u8>);
}
