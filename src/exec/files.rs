use crate::error::{RexecError, RexecErrorType};
use crate::util::config::{LogTimeStamp, PathConfig};
use crate::util::time::{time_stamp_day, time_stamp_hour, time_stamp_min, time_stamp_sec};
use log::debug;
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::{ffi::OsString, path::PathBuf};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};

pub struct FileInfo {
    pub filename: OsString,
    fd: File,
}
impl FileInfo {
    pub async fn next_file(alias: &String, conf: &PathConfig) -> Result<FileInfo, RexecError> {
        let path = FileInfo::rotate_files(alias, &conf).await?;
        debug!("Log file: {:?}", path);
        let fd = File::options()
            .append(true)
            .create(true)
            .open(path.as_path())
            .await
            .map_err(|e| RexecError::code_msg(
                RexecErrorType::FailedFileCreate, 
                e.to_string())
            )?;
        Ok(FileInfo {
            filename: path.into_os_string(),
            fd,
        })
    }
    pub async fn write(&mut self, line: &String) -> Result<usize, RexecError> {
        self.fd
            .write(line.as_bytes())
            .await
            .map_err(|e| RexecError::code_msg(
                RexecErrorType::FailedFileWrite, 
                e.to_string())
            )
    }
    pub async fn sync_all(&mut self) -> Result<(), RexecError> {
        self.fd.sync_all().await.map_err(|e| RexecError {
            code: RexecErrorType::FailedFileWrite,
            message: e.to_string(),
        })
    }
    fn next_filename_min(alias: &String) -> String {
        return format!("{}-utc-{}.log", alias, time_stamp_min());
    }
    fn next_filename_hour(alias: &String) -> String {
        return format!("{}-utc-{}.log", alias, time_stamp_hour());
    }
    fn next_filename_sec(alias: &String) -> String {
        return format!("{}-utc-{}.log", alias, time_stamp_sec());
    }
    fn next_filename_day(alias: &String) -> String {
        return format!("{}-utc-{}.log", alias, time_stamp_day());
    }
    async fn rotate_files(alias: &String, conf: &PathConfig) -> Result<PathBuf, RexecError> {
        let mut path = conf.install_dir.clone();
        path.push(&conf.log_dir);
        path.push(alias);

        debug!("Log directory: {:?}", path);
        create_dir_all(&path)
            .await
            .map_err(|e| RexecError::code_msg(RexecErrorType::FailedDirCreate, e.to_string()))?;
        FileInfo::do_clean_files(
            path.clone(),
            alias.clone(),
            "log".to_string(),
            conf.max_log_files,
        )
        .await
        .ok();
        path.push(match conf.log_timestamp {
            LogTimeStamp::Hour => FileInfo::next_filename_hour(alias),
            LogTimeStamp::Min => FileInfo::next_filename_min(alias),
            LogTimeStamp::Sec => FileInfo::next_filename_sec(alias),
            LogTimeStamp::Day => FileInfo::next_filename_day(alias),
        });
        debug!("Log filename full: {:?}", &path);
        Ok(path)
    }
    /// Deletes old files. Keep latest N files. The N is configurable
    ///
    async fn do_clean_files(
        p: PathBuf,
        alias: String,
        ext: String,
        keep_files: usize,
    ) -> Result<(), RexecError> {
        tokio::task::spawn_blocking(move || {
            let res = std::fs::read_dir(p).map(|res| {
                let mut v = res
                    .flatten()
                    .filter(|de| de.path().is_file())
                    .filter(|de| {
                        de.path()
                            .file_name()
                            .and_then(OsStr::to_str)
                            .is_some_and(|f| f.starts_with(&alias) && f.ends_with(&ext))
                    })
                    .collect::<Vec<_>>();
                debug!("log files found: {:?}", &v);
                v.sort_by(|l, r| {
                    l.metadata()
                        .and_then(|lm| lm.created())
                        .and_then(|lt| {
                            r.metadata()
                                .and_then(|rm| rm.created())
                                .map(|rt| rt.cmp(&lt))
                        })
                        .unwrap_or(Ordering::Equal)
                });
                // This keep first four newest files and removes the oldest
                for f in &v[std::cmp::min(keep_files, v.len())..] {
                    debug!("Deleting old log: {:?}", f.path());
                    std::fs::remove_file(f.path()).ok();
                }
            });
            if let Err(e) = res {
                debug!("Failed to read directory {:?}", e);
            }
        })
        .await
        .map_err(|e| RexecError {
            code: RexecErrorType::FailedDirCreate,
            message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod files_tests {
    use futures_util::TryFutureExt;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    async fn fn1() -> Result<(), &'static str> {
        Ok::<(), &str>(())
    }
    async fn fn2() -> Result<(), &'static str> {
        Err::<(), &str>("")
    }
    #[test]
    fn test_futures() {
        let call = async {
            fn1().and_then(|_| fn2()).await;
        };

        tokio::runtime::Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(call);
    }
    #[test]
    fn test_slice() {
        let v = vec![1, 2, 3, 4];
        assert_eq!(*&v[0..2].len(), 2);
        assert_eq!(*&v[1..].len(), 3);
        assert_eq!(*&v[3..].len(), 1);
        assert_eq!(*&v[std::cmp::min(3, v.len())..].len(), 1);
        assert_eq!(*&v[std::cmp::min(4, v.len())..].len(), 0);

        let v = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(*&v[std::cmp::min(4, v.len())..].len(), 4);

        let v: Vec<i32> = Vec::new();
        assert_eq!(*&v[std::cmp::min(4, v.len())..].len(), 0);
    }
}
