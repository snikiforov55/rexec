use log::debug;
use std::path::PathBuf;
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};

use crate::util::time::time_stamp_min;
use crate::error::{RexecError, RexecErrorType};

pub struct FileInfo {
    pub filename: String,
    fd: File,
}
impl FileInfo {
    pub async fn next_file(alias: &String, dir: &String) -> Result<FileInfo, RexecError> {
        let filename = FileInfo::next_filename(alias);
        let path_dir = [dir, alias].iter().collect::<PathBuf>();
        debug!("Log directory: {:?}", path_dir);
        create_dir_all(path_dir)
            .await
            .map_err(|e| RexecError::code_msg(RexecErrorType::FailedDirCreate, e.to_string()))?;

        let path: PathBuf = [dir, alias, &filename].iter().collect();
        debug!("Log file: {:?}", path);
        let fd = File::options()
            .append(true)
            .create(true)
            .open(path.as_path())
            .await
            .map_err(|e| RexecError::code_msg(RexecErrorType::FailedFileCreate, e.to_string()))?;
        Ok(FileInfo { filename, fd })
    }
    pub async fn write(&mut self, line: &String) -> Result<usize, RexecError> {
        self.fd
            .write(line.as_bytes())
            .await
            .map_err(|e| RexecError::code_msg(RexecErrorType::FailedFileWrite, e.to_string()))
    }
    pub async fn sync_all(&mut self) -> Result<(), RexecError> {
        self.fd.sync_all().await.map_err(|e| RexecError {
            code: RexecErrorType::FailedFileWrite,
            message: e.to_string(),
        })
    }
    fn next_filename(alias: &String) -> String {
        return format!("{}-utc-{}.log", alias, time_stamp_min());
    }

}
