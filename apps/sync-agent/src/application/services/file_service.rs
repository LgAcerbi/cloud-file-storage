use crate::application::ports::file_blob_repository::{FileBlobMetadata, FileBlobRepository};
use crate::application::ports::file_metadata_repository::FileMetadataRepository;
use crate::application::ports::file_remote_gateway::{FileRemoteGateway, FileRemoteGatewayError};
use crate::domain::entities::file_metadata::{FileMetadata, FileMetadataError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileServiceError {
    FileBlobNotFound,
    FileBlobMetadataNotFound,
    FileBlobHashNotFound,
    RemoteGateway(FileRemoteGatewayError),
    InvalidFileMetadata(FileMetadataError),
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

    fn load_local_blob(&self, file_path: &str) -> Result<Vec<u8>, FileServiceError> {
        self.file_blob_repository
            .get_file_blob_by_path(file_path)
            .ok_or(FileServiceError::FileBlobNotFound)
    }

    fn load_local_snapshot(
        &self,
        file_path: &str,
    ) -> Result<(FileBlobMetadata, Option<FileMetadata>), FileServiceError> {
        let file_blob_metadata = self
            .file_blob_repository
            .get_file_blob_metadata_by_path(file_path)
            .ok_or(FileServiceError::FileBlobMetadataNotFound)?;
        let file_metadata = self.file_metadata_repository.get_file_metadata_by_path(file_path);

        Ok((file_blob_metadata, file_metadata))
    }

    fn should_update_by_metadata(
        &self,
        file_metadata: &FileMetadata,
        file_blob_metadata: &FileBlobMetadata,
    ) -> bool {
        file_metadata.size_bytes() != file_blob_metadata.size_bytes
            || file_metadata.modified_at() != file_blob_metadata.modified_at
    }

    fn should_update_by_hash(
        &self,
        local_file_hash: &str,
        file_metadata: &FileMetadata,
    ) -> bool {
        local_file_hash != file_metadata.file_hash()
    }

    fn sync_existing_remote(
        &self,
        file_path: &str,
        file_blob: Vec<u8>,
        file_blob_metadata: &FileBlobMetadata,
        file_metadata: &FileMetadata,
    ) -> Result<(), FileServiceError> {
        let is_changed = self.should_update_by_metadata(file_metadata, file_blob_metadata);

        if is_changed {
            let local_file_hash = self
                .file_blob_repository
                .get_file_blob_hash_by_path(file_path)
                .ok_or(FileServiceError::FileBlobHashNotFound)?;

            if !self.should_update_by_hash(&local_file_hash, file_metadata) {
                return Ok(());
            }

            let updated_file_metadata = self
                .file_remote_gateway
                .update_file(file_metadata.id(), file_metadata.etag(), file_blob)
                .map_err(FileServiceError::RemoteGateway)?;
            let refreshed_file_metadata = FileMetadata::new(
                file_metadata.id().to_string(),
                file_metadata.name().to_string(),
                file_metadata.file_path().to_string(),
                file_blob_metadata.size_bytes,
                file_blob_metadata.modified_at,
                local_file_hash,
                updated_file_metadata.etag().to_string(),
            )
            .map_err(FileServiceError::InvalidFileMetadata)?;
            self.file_metadata_repository
                .update_file_metadata(refreshed_file_metadata);
        }

        Ok(())
    }

    fn sync_new_remote(&self, file_blob: Vec<u8>) -> Result<(), FileServiceError> {
        let uploaded_file_metadata = self
            .file_remote_gateway
            .upload_file(file_blob)
            .map_err(FileServiceError::RemoteGateway)?;
        self.file_metadata_repository
            .create_file_metadata(uploaded_file_metadata);
        Ok(())
    }

    pub fn sync_local_changes_to_remote(&self, file_path: &str) -> Result<(), FileServiceError> {
        let file_blob = self.load_local_blob(file_path)?;
        let (file_blob_metadata, file_metadata) = self.load_local_snapshot(file_path)?;

        match file_metadata {
            Some(file_metadata) => {
                self.sync_existing_remote(file_path, file_blob, &file_blob_metadata, &file_metadata)?
            }
            None => self.sync_new_remote(file_blob)?,
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{FileService, FileServiceError};
    use crate::application::ports::file_blob_repository::{FileBlobMetadata, FileBlobRepository};
    use crate::application::ports::file_metadata_repository::FileMetadataRepository;
    use crate::application::ports::file_remote_gateway::{FileRemoteGateway, FileRemoteGatewayError};
    use crate::domain::entities::file_metadata::FileMetadata;
    use std::cell::RefCell;

    struct InMemoryFileMetadataRepository {
        file_metadata: Option<FileMetadata>,
        created_file_metadata: RefCell<Vec<FileMetadata>>,
        updated_file_metadata: RefCell<Vec<FileMetadata>>,
    }

    impl FileMetadataRepository for InMemoryFileMetadataRepository {
        fn create_file_metadata(&self, file_metadata: FileMetadata) {
            self.created_file_metadata.borrow_mut().push(file_metadata);
        }

        fn update_file_metadata(&self, file_metadata: FileMetadata) {
            self.updated_file_metadata.borrow_mut().push(file_metadata);
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
        file_blob_metadata: Option<FileBlobMetadata>,
        file_blob_hash: Option<String>,
    }

    impl FileBlobRepository for InMemoryFileBlobRepository {
        fn exists_file_blob_by_id(&self, _file_id: &str) -> bool {
            self.file_blob.is_some()
        }

        fn get_file_blob_by_path(&self, _file_path: &str) -> Option<Vec<u8>> {
            self.file_blob.clone()
        }

        fn get_file_blob_metadata_by_path(&self, _file_path: &str) -> Option<FileBlobMetadata> {
            self.file_blob_metadata.clone()
        }

        fn get_file_blob_hash_by_path(&self, _file_path: &str) -> Option<String> {
            self.file_blob_hash.clone()
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum RemoteCall {
        Upload { file_blob: Vec<u8> },
        Update {
            id: String,
            expected_etag: String,
            file_blob: Vec<u8>,
        },
    }

    struct InMemoryFileRemoteGateway {
        calls: RefCell<Vec<RemoteCall>>,
        upload_response: FileMetadata,
        upload_error: Option<FileRemoteGatewayError>,
        update_error: Option<FileRemoteGatewayError>,
    }

    impl InMemoryFileRemoteGateway {
        fn new(upload_response: FileMetadata) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                upload_response,
                upload_error: None,
                update_error: None,
            }
        }
    }

    impl FileRemoteGateway for InMemoryFileRemoteGateway {
        fn get_file_metadata_by_id(&self, _id: &str) -> Option<FileMetadata> {
            None
        }

        fn upload_file(&self, file_blob: Vec<u8>) -> Result<FileMetadata, FileRemoteGatewayError> {
            if let Some(err) = &self.upload_error {
                return Err(err.clone());
            }
            self.calls.borrow_mut().push(RemoteCall::Upload { file_blob });
            Ok(self.upload_response.clone())
        }

        fn update_file(
            &self,
            id: &str,
            expected_etag: &str,
            file_blob: Vec<u8>,
        ) -> Result<FileMetadata, FileRemoteGatewayError> {
            if let Some(err) = &self.update_error {
                return Err(err.clone());
            }
            self.calls.borrow_mut().push(RemoteCall::Update {
                id: id.to_string(),
                expected_etag: expected_etag.to_string(),
                file_blob,
            });
            Ok(self.upload_response.clone())
        }
    }

    fn build_file_metadata() -> FileMetadata {
        FileMetadata::new(
            "file-1".to_string(),
            "report.pdf".to_string(),
            "/docs/report.pdf".to_string(),
            1024,
            1_710_000_000,
            "hash-1".to_string(),
            "etag-1".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn returns_error_when_file_blob_is_not_found_by_path() {
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: Some(build_file_metadata()),
            created_file_metadata: RefCell::new(Vec::new()),
            updated_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository {
            file_blob: None,
            file_blob_metadata: None,
            file_blob_hash: None,
        };
        let remote_gateway = InMemoryFileRemoteGateway::new(build_file_metadata());
        let service = FileService::new(metadata_repository, blob_repository, remote_gateway);

        let result = service.sync_local_changes_to_remote("/docs/missing.pdf");

        assert_eq!(result, Err(FileServiceError::FileBlobNotFound));
    }

    #[test]
    fn updates_remote_file_when_metadata_exists() {
        let remote_updated_metadata = FileMetadata::new(
            "file-1".to_string(),
            "report.pdf".to_string(),
            "/docs/report.pdf".to_string(),
            2048,
            1_720_000_000,
            "hash-2".to_string(),
            "etag-2".to_string(),
        )
        .unwrap();
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: Some(build_file_metadata()),
            created_file_metadata: RefCell::new(Vec::new()),
            updated_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository {
            file_blob: Some(vec![1, 2, 3]),
            file_blob_metadata: Some(FileBlobMetadata {
                size_bytes: 2048,
                modified_at: 1_720_000_000,
            }),
            file_blob_hash: Some("hash-2".to_string()),
        };
        let remote_gateway = InMemoryFileRemoteGateway::new(remote_updated_metadata);

        let service = FileService::new(metadata_repository, blob_repository, remote_gateway);
        let result = service.sync_local_changes_to_remote("/docs/report.pdf");

        assert_eq!(result, Ok(()));
        assert_eq!(
            service.file_remote_gateway.calls.into_inner(),
            vec![RemoteCall::Update {
                id: "file-1".to_string(),
                expected_etag: "etag-1".to_string(),
                file_blob: vec![1, 2, 3],
            }]
        );
        assert!(service
            .file_metadata_repository
            .created_file_metadata
            .into_inner()
            .is_empty());
        assert_eq!(
            service
                .file_metadata_repository
                .updated_file_metadata
                .into_inner(),
            vec![FileMetadata::new(
                "file-1".to_string(),
                "report.pdf".to_string(),
                "/docs/report.pdf".to_string(),
                2048,
                1_720_000_000,
                "hash-2".to_string(),
                "etag-2".to_string(),
            )
            .unwrap()]
        );
    }

    #[test]
    fn uploads_remote_file_when_metadata_does_not_exist() {
        let uploaded_metadata = FileMetadata::new(
            "file-2".to_string(),
            "new-file.pdf".to_string(),
            "/docs/new-file.pdf".to_string(),
            2048,
            1_730_000_000,
            "hash-uploaded".to_string(),
            "etag-uploaded".to_string(),
        )
        .unwrap();
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: None,
            created_file_metadata: RefCell::new(Vec::new()),
            updated_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository {
            file_blob: Some(vec![4, 5, 6]),
            file_blob_metadata: Some(FileBlobMetadata {
                size_bytes: 2048,
                modified_at: 1_730_000_000,
            }),
            file_blob_hash: Some("hash-uploaded".to_string()),
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
        assert!(service
            .file_metadata_repository
            .updated_file_metadata
            .into_inner()
            .is_empty());
    }

    #[test]
    fn does_not_update_remote_file_when_metadata_matches_blob_metadata() {
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: Some(build_file_metadata()),
            created_file_metadata: RefCell::new(Vec::new()),
            updated_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository {
            file_blob: Some(vec![1, 2, 3]),
            file_blob_metadata: Some(FileBlobMetadata {
                size_bytes: 1024,
                modified_at: 1_710_000_000,
            }),
            file_blob_hash: Some("hash-1".to_string()),
        };
        let remote_gateway = InMemoryFileRemoteGateway::new(build_file_metadata());

        let service = FileService::new(metadata_repository, blob_repository, remote_gateway);
        let result = service.sync_local_changes_to_remote("/docs/report.pdf");

        assert_eq!(result, Ok(()));
        assert!(service.file_remote_gateway.calls.into_inner().is_empty());
        assert!(service
            .file_metadata_repository
            .updated_file_metadata
            .into_inner()
            .is_empty());
    }

    #[test]
    fn does_not_update_remote_file_when_hash_matches_after_metadata_change_detection() {
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: Some(build_file_metadata()),
            created_file_metadata: RefCell::new(Vec::new()),
            updated_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository {
            file_blob: Some(vec![1, 2, 3]),
            file_blob_metadata: Some(FileBlobMetadata {
                size_bytes: 2048,
                modified_at: 1_720_000_000,
            }),
            file_blob_hash: Some("hash-1".to_string()),
        };
        let remote_gateway = InMemoryFileRemoteGateway::new(build_file_metadata());

        let service = FileService::new(metadata_repository, blob_repository, remote_gateway);
        let result = service.sync_local_changes_to_remote("/docs/report.pdf");

        assert_eq!(result, Ok(()));
        assert!(service.file_remote_gateway.calls.into_inner().is_empty());
        assert!(service
            .file_metadata_repository
            .updated_file_metadata
            .into_inner()
            .is_empty());
    }

    #[test]
    fn returns_error_when_hash_is_needed_but_not_found() {
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: Some(build_file_metadata()),
            created_file_metadata: RefCell::new(Vec::new()),
            updated_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository {
            file_blob: Some(vec![1, 2, 3]),
            file_blob_metadata: Some(FileBlobMetadata {
                size_bytes: 2048,
                modified_at: 1_720_000_000,
            }),
            file_blob_hash: None,
        };
        let remote_gateway = InMemoryFileRemoteGateway::new(build_file_metadata());

        let service = FileService::new(metadata_repository, blob_repository, remote_gateway);
        let result = service.sync_local_changes_to_remote("/docs/report.pdf");

        assert_eq!(result, Err(FileServiceError::FileBlobHashNotFound));
    }

    #[test]
    fn returns_error_when_remote_upload_fails_and_does_not_persist_metadata() {
        let uploaded_metadata = FileMetadata::new(
            "file-2".to_string(),
            "new-file.pdf".to_string(),
            "/docs/new-file.pdf".to_string(),
            2048,
            1_730_000_000,
            "hash-uploaded".to_string(),
            "etag-uploaded".to_string(),
        )
        .unwrap();
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: None,
            created_file_metadata: RefCell::new(Vec::new()),
            updated_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository {
            file_blob: Some(vec![4, 5, 6]),
            file_blob_metadata: Some(FileBlobMetadata {
                size_bytes: 2048,
                modified_at: 1_730_000_000,
            }),
            file_blob_hash: Some("hash-uploaded".to_string()),
        };
        let mut remote_gateway = InMemoryFileRemoteGateway::new(uploaded_metadata);
        remote_gateway.upload_error = Some(FileRemoteGatewayError::Timeout);

        let service = FileService::new(metadata_repository, blob_repository, remote_gateway);
        let result = service.sync_local_changes_to_remote("/docs/new-file.pdf");

        assert_eq!(
            result,
            Err(FileServiceError::RemoteGateway(
                FileRemoteGatewayError::Timeout
            ))
        );
        assert!(service.file_remote_gateway.calls.into_inner().is_empty());
        assert!(service
            .file_metadata_repository
            .created_file_metadata
            .into_inner()
            .is_empty());
        assert!(service
            .file_metadata_repository
            .updated_file_metadata
            .into_inner()
            .is_empty());
    }

    #[test]
    fn returns_error_when_remote_update_fails_and_does_not_persist_metadata() {
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: Some(build_file_metadata()),
            created_file_metadata: RefCell::new(Vec::new()),
            updated_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository {
            file_blob: Some(vec![1, 2, 3]),
            file_blob_metadata: Some(FileBlobMetadata {
                size_bytes: 2048,
                modified_at: 1_720_000_000,
            }),
            file_blob_hash: Some("hash-2".to_string()),
        };
        let mut remote_gateway = InMemoryFileRemoteGateway::new(build_file_metadata());
        remote_gateway.update_error = Some(FileRemoteGatewayError::Network);

        let service = FileService::new(metadata_repository, blob_repository, remote_gateway);
        let result = service.sync_local_changes_to_remote("/docs/report.pdf");

        assert_eq!(
            result,
            Err(FileServiceError::RemoteGateway(
                FileRemoteGatewayError::Network
            ))
        );
        assert!(service.file_remote_gateway.calls.into_inner().is_empty());
        assert!(service
            .file_metadata_repository
            .created_file_metadata
            .into_inner()
            .is_empty());
        assert!(service
            .file_metadata_repository
            .updated_file_metadata
            .into_inner()
            .is_empty());
    }

    #[test]
    fn returns_conflict_when_remote_etag_precondition_fails() {
        let metadata_repository = InMemoryFileMetadataRepository {
            file_metadata: Some(build_file_metadata()),
            created_file_metadata: RefCell::new(Vec::new()),
            updated_file_metadata: RefCell::new(Vec::new()),
        };
        let blob_repository = InMemoryFileBlobRepository {
            file_blob: Some(vec![1, 2, 3]),
            file_blob_metadata: Some(FileBlobMetadata {
                size_bytes: 2048,
                modified_at: 1_720_000_000,
            }),
            file_blob_hash: Some("hash-2".to_string()),
        };
        let mut remote_gateway = InMemoryFileRemoteGateway::new(build_file_metadata());
        remote_gateway.update_error = Some(FileRemoteGatewayError::Conflict);

        let service = FileService::new(metadata_repository, blob_repository, remote_gateway);
        let result = service.sync_local_changes_to_remote("/docs/report.pdf");

        assert_eq!(
            result,
            Err(FileServiceError::RemoteGateway(
                FileRemoteGatewayError::Conflict
            ))
        );
        assert!(service
            .file_metadata_repository
            .updated_file_metadata
            .into_inner()
            .is_empty());
    }
}
