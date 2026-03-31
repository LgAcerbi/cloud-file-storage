use crate::domain::entities::file::File;

pub trait FileMetadataRepository {
    fn get_file_by_id(&self, id: &str) -> Option<File>;
}
