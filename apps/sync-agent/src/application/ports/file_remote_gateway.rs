use crate::domain::entities::file_metadata::FileMetadata;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileRemoteGatewayError {
    Network,
    Timeout,
    Conflict,
    Unauthorized,
    Unknown(String),
}

pub trait FileRemoteGateway {
    fn get_file_metadata_by_id(&self, id: &str) -> Option<FileMetadata>;
    fn upload_file(&self, file_blob: Vec<u8>) -> Result<FileMetadata, FileRemoteGatewayError>;
    fn update_file(
        &self,
        id: &str,
        file_blob: Vec<u8>,
    ) -> Result<FileMetadata, FileRemoteGatewayError>;
}
