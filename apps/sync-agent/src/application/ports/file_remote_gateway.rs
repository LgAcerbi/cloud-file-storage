pub trait FileRemoteGateway {
    fn upload_file(&self, file_blob: Vec<u8>);
    fn update_file(&self, id: &str, file_blob: Vec<u8>);
}
