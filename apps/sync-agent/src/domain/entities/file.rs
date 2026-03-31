#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    id: String,
    name: String,
    size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileError {
    EmptyId,
    EmptyName,
}

impl File {
    pub fn new(id: String, name: String, size_bytes: u64) -> Result<Self, FileError> {
        if id.trim().is_empty() {
            return Err(FileError::EmptyId);
        }

        if name.trim().is_empty() {
            return Err(FileError::EmptyName);
        }

        Ok(Self {
            id,
            name,
            size_bytes,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::{File, FileError};

    #[test]
    fn creates_file_when_data_is_valid() {
        let file = File::new("file-1".to_string(), "report.pdf".to_string(), 1024);

        assert!(file.is_ok());

        let file = file.unwrap();
        assert_eq!(file.id(), "file-1");
        assert_eq!(file.name(), "report.pdf");
        assert_eq!(file.size_bytes(), 1024);
    }

    #[test]
    fn returns_error_when_id_is_empty() {
        let file = File::new("   ".to_string(), "report.pdf".to_string(), 1024);

        assert_eq!(file, Err(FileError::EmptyId));
    }

    #[test]
    fn returns_error_when_name_is_empty() {
        let file = File::new("file-1".to_string(), "   ".to_string(), 1024);

        assert_eq!(file, Err(FileError::EmptyName));
    }
}
