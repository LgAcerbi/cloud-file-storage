#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    id: String,
    name: String,
    file_path: String,
    size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileMetadataError {
    EmptyId,
    EmptyName,
    EmptyFilePath,
}

impl FileMetadata {
    pub fn new(
        id: String,
        name: String,
        file_path: String,
        size_bytes: u64,
    ) -> Result<Self, FileMetadataError> {
        if id.trim().is_empty() {
            return Err(FileMetadataError::EmptyId);
        }

        if name.trim().is_empty() {
            return Err(FileMetadataError::EmptyName);
        }

        if file_path.trim().is_empty() {
            return Err(FileMetadataError::EmptyFilePath);
        }

        Ok(Self {
            id,
            name,
            file_path,
            size_bytes,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::{FileMetadata, FileMetadataError};

    #[test]
    fn creates_file_metadata_when_data_is_valid() {
        let file_metadata = FileMetadata::new(
            "file-1".to_string(),
            "report.pdf".to_string(),
            "/docs/report.pdf".to_string(),
            1024,
        );

        assert!(file_metadata.is_ok());

        let file_metadata = file_metadata.unwrap();
        assert_eq!(file_metadata.id(), "file-1");
        assert_eq!(file_metadata.name(), "report.pdf");
        assert_eq!(file_metadata.file_path(), "/docs/report.pdf");
        assert_eq!(file_metadata.size_bytes(), 1024);
    }

    #[test]
    fn returns_error_when_id_is_empty() {
        let file_metadata = FileMetadata::new(
            "   ".to_string(),
            "report.pdf".to_string(),
            "/docs/report.pdf".to_string(),
            1024,
        );

        assert_eq!(file_metadata, Err(FileMetadataError::EmptyId));
    }

    #[test]
    fn returns_error_when_name_is_empty() {
        let file_metadata = FileMetadata::new(
            "file-1".to_string(),
            "   ".to_string(),
            "/docs/report.pdf".to_string(),
            1024,
        );

        assert_eq!(file_metadata, Err(FileMetadataError::EmptyName));
    }

    #[test]
    fn returns_error_when_file_path_is_empty() {
        let file_metadata = FileMetadata::new(
            "file-1".to_string(),
            "report.pdf".to_string(),
            "   ".to_string(),
            1024,
        );

        assert_eq!(file_metadata, Err(FileMetadataError::EmptyFilePath));
    }
}
