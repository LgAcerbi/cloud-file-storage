#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePart {
    id: String,
    index: u32,
    size_bytes: u64,
    file_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePartError {
    EmptyId,
    EmptyFileId,
}

impl FilePart {
    pub fn new(id: String, index: u32, size_bytes: u64, file_id: String) -> Result<Self, FilePartError> {
        if id.trim().is_empty() {
            return Err(FilePartError::EmptyId);
        }

        if file_id.trim().is_empty() {
            return Err(FilePartError::EmptyFileId);
        }

        Ok(Self {
            id,
            index,
            size_bytes,
            file_id,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub fn file_id(&self) -> &str {
        &self.file_id
    }
}

#[cfg(test)]
mod tests {
    use super::{FilePart, FilePartError};

    #[test]
    fn creates_file_part_when_data_is_valid() {
        let file_part = FilePart::new(
            "part-1".to_string(),
            0,
            5_242_880,
            "file-1".to_string(),
        );

        assert!(file_part.is_ok());

        let file_part = file_part.unwrap();
        assert_eq!(file_part.id(), "part-1");
        assert_eq!(file_part.index(), 0);
        assert_eq!(file_part.size_bytes(), 5_242_880);
        assert_eq!(file_part.file_id(), "file-1");
    }

    #[test]
    fn returns_error_when_id_is_empty() {
        let file_part = FilePart::new("   ".to_string(), 0, 1024, "file-1".to_string());

        assert_eq!(file_part, Err(FilePartError::EmptyId));
    }

    #[test]
    fn returns_error_when_file_id_is_empty() {
        let file_part = FilePart::new("part-1".to_string(), 0, 1024, "   ".to_string());

        assert_eq!(file_part, Err(FilePartError::EmptyFileId));
    }
}
