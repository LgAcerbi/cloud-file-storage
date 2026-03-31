use crate::application::ports::file_metadata_repository::FileMetadataRepository;
use crate::application::ports::file_blob_repository::FileBlobRepository;
use crate::application::ports::file_remote_gateway::FileRemoteGateway;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileServiceError {
    FileBlobNotFound,
}

pub struct FileService<TFileMetadataRepository, TFileBlobRepository, TFileRemoteGateway>
where
    TFileMetadataRepository: FileMetadataRepository,
    TFileBlobRepository: FileBlobRepository,
    TFileRemoteGateway: FileRemoteGateway,
{
    file_metadata_repository: TFileMetadataRepository,
    file_blob_repository: TFileBlobRepository,
    file_remote_gateway: TFileRemoteGateway,
}

impl<TFileMetadataRepository, TFileBlobRepository, TFileRemoteGateway>
    FileService<TFileMetadataRepository, TFileBlobRepository, TFileRemoteGateway>
where
    TFileMetadataRepository: FileMetadataRepository,
    TFileBlobRepository: FileBlobRepository,
    TFileRemoteGateway: FileRemoteGateway,
{
    pub fn new(
        file_metadata_repository: TFileMetadataRepository,
        file_blob_repository: TFileBlobRepository,
        file_remote_gateway: TFileRemoteGateway,
    ) -> Self {
        Self {
            file_metadata_repository,
            file_blob_repository,
            file_remote_gateway,
        }
    }

    pub fn sync_local_changes_to_remote(&self, file_path: &str) -> Result<(), FileServiceError> {
        let file_blob = self
            .file_blob_repository
            .get_file_blob_by_path(file_path)
            .ok_or(FileServiceError::FileBlobNotFound)?;

        let file_metadata = self
            .file_metadata_repository
            .get_file_metadata_by_path(file_path);

        match file_metadata {
            Some(file_metadata) => self.file_remote_gateway.update_file(file_metadata.id(), file_blob),
            None => {
                let uploaded_file_metadata = self.file_remote_gateway.upload_file(file_blob);
                self.file_metadata_repository
                    .create_file_metadata(uploaded_file_metadata);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{FileService, FileServiceError};
    use crate::application::ports::file_blob_repository::FileBlobRepository;
    use crate::application::ports::file_metadata_repository::FileMetadataRepository;
    use crate::application::ports::file_remote_gateway::FileRemoteGateway;
    use crate::domain::entities::file_metadata::FileMetadata;
    use std::cell::RefCell;

    struct InMemoryFileMetadataRepository {
        file_metadata: Option<FileMetadata>,
        created_file_metadata: RefCell<Vec<FileMetadata>>,
    }

    impl FileMetadataRepository for InMemoryFileMetadataRepository {
        fn create_file_metadata(&self, file_metadata: FileMetadata) {
            self.created_file_metadata.borrow_mut().push(file_metadata);
        }

        fn get_file_metadata_by_id(&self, _id: &str) -> Option<FileMetadata> {
            self.file_metadata.clone()
        }

        fn get_file_metadata_by_path(&self, _file_path: &str) -> Option<FileMetadata> {
            self.file_metadata.clone()
        }
    }

    struct InMemoryFileBlobRepository {
        file_blob: Option<Vec<u8>>,
    }

    impl FileBlobRepository for InMemoryFileBlobRepository {
        fn exists_file_blob_by_id(&self, _file_id: &str) -> bool {
            self.file_blob.is_some()
        }

        fn get_file_blob_by_path(&self, _file_path: &str) -> Option<Vec<u8>> {
            self.file_blob.clone()
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum RemoteCall {
        Upload { file_blob: Vec<u8> },
        Update { id: String, file_blob: Vec<u8> },
    }

    struct InMemoryFileRemoteGateway {
        calls: RefCell<Vec<RemoteCall>>,
        upload_response: FileMetadata,
    }

    impl InMemoryFileRemoteGateway {
        fn new(upload_response: FileMetadata) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                upload_response,
            }
        }
    }

    impl FileRemoteGateway for InMemoryFileRemoteGateway {
        fn get_file_metadata_by_id(&self, _id: &str) -> Option<FileMetadata> {
            None
        }

        fn upload_file(&self, file_blob: Vec<u8>) -> FileMetadata {
            self.calls.borrow_mut().push(RemoteCall::Upload { file_blob });
            self.upload_response.clone()
        }

        fn update_file(&self, id: &str, file_blob: Vec<u8>) {
            self.calls.borrow_mut().push(RemoteCall::Update {
                id: id.to_string(),
                file_blob,
            });
        }
    }

    fn build_file_metadata() -> FileMetadata {
        FileMetadata::new(
            "file-1".to_string(),
            "report.pdf".to_string(),
            "/docs/report.pdf".to_string(),
            1024,
        )
        .unwrap()
    }

    #[test]
    fn returns_error_when_file_blob_is_not_found_by_path() {
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: Some(build_file_metadata()),
            created_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository { file_blob: None };
        let remote_gateway = InMemoryFileRemoteGateway::new(build_file_metadata());
        let service = FileService::new(metadata_repository, blob_repository, remote_gateway);

        let result = service.sync_local_changes_to_remote("/docs/missing.pdf");

        assert_eq!(result, Err(FileServiceError::FileBlobNotFound));
    }

    #[test]
    fn updates_remote_file_when_metadata_exists() {
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: Some(build_file_metadata()),
            created_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository {
            file_blob: Some(vec![1, 2, 3]),
        };
        let remote_gateway = InMemoryFileRemoteGateway::new(build_file_metadata());

        let service = FileService::new(metadata_repository, blob_repository, remote_gateway);
        let result = service.sync_local_changes_to_remote("/docs/report.pdf");

        assert_eq!(result, Ok(()));
        assert_eq!(
            service.file_remote_gateway.calls.into_inner(),
            vec![RemoteCall::Update {
                id: "file-1".to_string(),
                file_blob: vec![1, 2, 3],
            }]
        );
        assert!(service
            .file_metadata_repository
            .created_file_metadata
            .into_inner()
            .is_empty());
    }

    #[test]
    fn uploads_remote_file_when_metadata_does_not_exist() {
        let uploaded_metadata = FileMetadata::new(
            "file-2".to_string(),
            "new-file.pdf".to_string(),
            "/docs/new-file.pdf".to_string(),
            2048,
        )
        .unwrap();
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: None,
            created_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository {
            file_blob: Some(vec![4, 5, 6]),
        };
        let remote_gateway = InMemoryFileRemoteGateway::new(uploaded_metadata.clone());

        let service = FileService::new(metadata_repository, blob_repository, remote_gateway);

        let result = service.sync_local_changes_to_remote("/docs/new-file.pdf");

        assert_eq!(result, Ok(()));
        assert_eq!(
            service.file_remote_gateway.calls.into_inner(),
            vec![RemoteCall::Upload {
                file_blob: vec![4, 5, 6],
            }]
        );
        assert_eq!(
            service
                .file_metadata_repository
                .created_file_metadata
                .into_inner(),
            vec![uploaded_metadata]
        );
    }
}
