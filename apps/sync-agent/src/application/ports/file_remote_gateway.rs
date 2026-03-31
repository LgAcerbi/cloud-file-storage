use crate::domain::entities::file_metadata::FileMetadata;

pub trait FileRemoteGateway {
    fn get_file_metadata_by_id(&self, id: &str) -> Option<FileMetadata>;
}
