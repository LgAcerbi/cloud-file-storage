use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::application::ports::file_snapshot_repository::FileSnapshotRepository;
use crate::domain::entities::file_metadata::FileMetadata;

pub struct SqliteFileSnapshotRepository {
    connection: Mutex<Connection>,
}

impl SqliteFileSnapshotRepository {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        Self::init_schema(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn from_connection(connection: Connection) -> rusqlite::Result<Self> {
        Self::init_schema(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn init_schema(connection: &Connection) -> rusqlite::Result<()> {
        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS file_snapshots (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                file_path TEXT NOT NULL UNIQUE,
                size_bytes INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                file_hash TEXT NOT NULL,
                etag TEXT NOT NULL,
                last_checked_at INTEGER NOT NULL
            );
            ",
        )?;

        Ok(())
    }

    fn row_to_metadata(row: &Row<'_>) -> rusqlite::Result<FileMetadata> {
        let id: String = row.get("id")?;
        let name: String = row.get("name")?;
        let file_path: String = row.get("file_path")?;
        let size_bytes_i64: i64 = row.get("size_bytes")?;
        let modified_at_i64: i64 = row.get("modified_at")?;
        let file_hash: String = row.get("file_hash")?;
        let etag: String = row.get("etag")?;
        let last_checked_at_i64: i64 = row.get("last_checked_at")?;

        let size_bytes = u64::try_from(size_bytes_i64)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, size_bytes_i64))?;
        let modified_at = u64::try_from(modified_at_i64)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, modified_at_i64))?;
        let last_checked_at = u64::try_from(last_checked_at_i64)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, last_checked_at_i64))?;

        let metadata = FileMetadata::new(
            id, name, file_path, size_bytes, modified_at, file_hash, etag,
        )
        .map_err(|_| rusqlite::Error::InvalidQuery)?;

        Ok(metadata.with_last_checked_at(last_checked_at))
    }
}

impl FileSnapshotRepository for SqliteFileSnapshotRepository {
    fn create_file_metadata(&self, file_metadata: FileMetadata) {
        let connection = self
            .connection
            .lock()
            .expect("SQLite mutex poisoned while creating file snapshot metadata");

        connection
            .execute(
                "
                INSERT INTO file_snapshots (
                    id, name, file_path, size_bytes, modified_at, file_hash, etag, last_checked_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    file_path = excluded.file_path,
                    size_bytes = excluded.size_bytes,
                    modified_at = excluded.modified_at,
                    file_hash = excluded.file_hash,
                    etag = excluded.etag,
                    last_checked_at = excluded.last_checked_at
                ",
                params![
                    file_metadata.id(),
                    file_metadata.name(),
                    file_metadata.file_path(),
                    file_metadata.size_bytes() as i64,
                    file_metadata.modified_at() as i64,
                    file_metadata.file_hash(),
                    file_metadata.etag(),
                    file_metadata.last_checked_at() as i64
                ],
            )
            .expect("Failed to create file snapshot metadata in SQLite");
    }

    fn update_file_metadata(&self, file_metadata: FileMetadata) {
        let connection = self
            .connection
            .lock()
            .expect("SQLite mutex poisoned while updating file snapshot metadata");

        connection
            .execute(
                "
                UPDATE file_snapshots
                SET
                    name = ?2,
                    file_path = ?3,
                    size_bytes = ?4,
                    modified_at = ?5,
                    file_hash = ?6,
                    etag = ?7,
                    last_checked_at = ?8
                WHERE id = ?1
                ",
                params![
                    file_metadata.id(),
                    file_metadata.name(),
                    file_metadata.file_path(),
                    file_metadata.size_bytes() as i64,
                    file_metadata.modified_at() as i64,
                    file_metadata.file_hash(),
                    file_metadata.etag(),
                    file_metadata.last_checked_at() as i64
                ],
            )
            .expect("Failed to update file snapshot metadata in SQLite");
    }

    fn get_file_metadata_by_id(&self, id: &str) -> Option<FileMetadata> {
        let connection = self
            .connection
            .lock()
            .expect("SQLite mutex poisoned while getting file snapshot metadata by id");

        connection
            .query_row(
                "
                SELECT
                    id, name, file_path, size_bytes, modified_at, file_hash, etag, last_checked_at
                FROM file_snapshots
                WHERE id = ?1
                ",
                params![id],
                Self::row_to_metadata,
            )
            .optional()
            .expect("Failed to query file snapshot metadata by id from SQLite")
    }

    fn get_file_metadata_by_path(&self, file_path: &str) -> Option<FileMetadata> {
        let connection = self
            .connection
            .lock()
            .expect("SQLite mutex poisoned while getting file snapshot metadata by path");

        connection
            .query_row(
                "
                SELECT
                    id, name, file_path, size_bytes, modified_at, file_hash, etag, last_checked_at
                FROM file_snapshots
                WHERE file_path = ?1
                ",
                params![file_path],
                Self::row_to_metadata,
            )
            .optional()
            .expect("Failed to query file snapshot metadata by path from SQLite")
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteFileSnapshotRepository;
    use crate::application::ports::file_snapshot_repository::FileSnapshotRepository;
    use crate::domain::entities::file_metadata::FileMetadata;
    use rusqlite::Connection;

    fn build_metadata() -> FileMetadata {
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
    fn creates_and_gets_snapshot_by_id_and_path() {
        let connection = Connection::open_in_memory().unwrap();
        let repository = SqliteFileSnapshotRepository::from_connection(connection).unwrap();
        let metadata = build_metadata();

        repository.create_file_metadata(metadata.clone());

        let by_id = repository.get_file_metadata_by_id("file-1");
        let by_path = repository.get_file_metadata_by_path("/docs/report.pdf");

        assert_eq!(by_id, Some(metadata.clone()));
        assert_eq!(by_path, Some(metadata));
    }

    #[test]
    fn updates_snapshot_and_persists_last_checked_at() {
        let connection = Connection::open_in_memory().unwrap();
        let repository = SqliteFileSnapshotRepository::from_connection(connection).unwrap();
        let metadata = build_metadata();

        repository.create_file_metadata(metadata.clone());
        let updated = FileMetadata::new(
            "file-1".to_string(),
            "report.pdf".to_string(),
            "/docs/report.pdf".to_string(),
            2048,
            1_720_000_000,
            "hash-2".to_string(),
            "etag-2".to_string(),
        )
        .unwrap()
        .with_last_checked_at(1_730_000_000);

        repository.update_file_metadata(updated.clone());

        let from_db = repository.get_file_metadata_by_id("file-1");
        assert_eq!(from_db, Some(updated));
    }

    #[test]
    fn returns_none_for_missing_snapshot() {
        let connection = Connection::open_in_memory().unwrap();
        let repository = SqliteFileSnapshotRepository::from_connection(connection).unwrap();

        let by_id = repository.get_file_metadata_by_id("missing");
        let by_path = repository.get_file_metadata_by_path("/missing/path.txt");

        assert_eq!(by_id, None);
        assert_eq!(by_path, None);
    }
}