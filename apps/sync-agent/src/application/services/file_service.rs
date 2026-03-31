use crate::application::ports::file_metadata_repository::FileMetadataRepository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileServiceError {
    FileMetadataNotFound,
}

pub struct FileService<TFileMetadataRepository>
where
    TFileMetadataRepository: FileMetadataRepository,
{
    file_metadata_repository: TFileMetadataRepository,
}

impl<TFileMetadataRepository> FileService<TFileMetadataRepository>
where
    TFileMetadataRepository: FileMetadataRepository,
{
    pub fn new(file_metadata_repository: TFileMetadataRepository) -> Self {
        Self {
            file_metadata_repository,
        }
    }

    pub fn sync_local_changes_to_remote(&self, file_path: &str) -> Result<(), FileServiceError> {
        let _file_metadata = self
            .file_metadata_repository
            .get_file_metadata_by_path(file_path)
            .ok_or(FileServiceError::FileMetadataNotFound)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{FileService, FileServiceError};
    use crate::application::ports::file_metadata_repository::FileMetadataRepository;
    use crate::domain::entities::file_metadata::FileMetadata;

    struct InMemoryFileMetadataRepository {
        file_metadata: Option<FileMetadata>,
    }

    impl FileMetadataRepository for InMemoryFileMetadataRepository {
        fn get_file_metadata_by_id(&self, _id: &str) -> Option<FileMetadata> {
            self.file_metadata.clone()
        }

        fn get_file_metadata_by_path(&self, _file_path: &str) -> Option<FileMetadata> {
            self.file_metadata.clone()
        }
    }

    #[test]
    fn returns_error_when_file_metadata_is_not_found_by_path() {
        let repository = InMemoryFileMetadataRepository {
            file_metadata: None,
        };
        let service = FileService::new(repository);

        let result = service.sync_local_changes_to_remote("/docs/missing.pdf");

        assert_eq!(result, Err(FileServiceError::FileMetadataNotFound));
    }
}
