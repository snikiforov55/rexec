use crate::error::{RexecError, RexecErrorType};
use crate::util::time::time_stamp_min;
use futures_util::TryFutureExt;
use log::debug;
use std::cmp::Ordering;
use std::{ffi::OsString, path::PathBuf};
use tokio::{
    fs::{self, create_dir_all, File},
    io::AsyncWriteExt,
};

pub struct FileInfo {
    pub filename: OsString,
    fd: File,
}
impl FileInfo {
    pub async fn next_file(alias: &String, dir: &String) -> Result<FileInfo, RexecError> {
        let path = FileInfo::rotate_files(alias, dir).await?;
        debug!("Log file: {:?}", path);
        let fd = File::options()
            .append(true)
            .create(true)
            .open(path.as_path())
            .await
            .map_err(|e| RexecError::code_msg(RexecErrorType::FailedFileCreate, e.to_string()))?;
        Ok(FileInfo {
            filename: path.into_os_string(),
            fd,
        })
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
    async fn rotate_files(alias: &String, dir: &String) -> Result<PathBuf, RexecError> {
        let mut path = [dir, alias].iter().collect::<PathBuf>();

        debug!("Log directory: {:?}", path);
        create_dir_all(&path)
            .await
            .map_err(|e| RexecError::code_msg(RexecErrorType::FailedDirCreate, e.to_string()))?;

        path.push(FileInfo::next_filename(alias));
        debug!("Log filename full: {:?}", &path);
        let pp = path.clone();
        let p = fs::try_exists(path)
            .and_then(|exists| async move {
                if exists {
                    Ok(pp)
                } else {
                    let ppp = pp.clone();
                    tokio::task::spawn_blocking(move || {
                        let r = std::fs::read_dir(pp)
                            .and_then(|res| {
                                res.map(|dirs| dirs.map(|e| 
                                    match e.path().is_file(){
                                        true => Some(e),
                                        _ => None
                                    }))
                                    .collect::<Result<Vec<_>, std::io::Error>>()
                                    
                            })
                            .map(|v| {
                                if v.len() < 5 {None}
                                else{
                                    v.into_iter()
                                        .flatten()
                                        .min_by(|l, r| {
                                        let cmp =
                                            l.metadata().and_then(|lm| lm.created()).and_then(|lt| {
                                                r.metadata()
                                                    .and_then(|rm| rm.created())
                                                    .map(|rt| lt.cmp(&rt))
                                            });
                                        match cmp {
                                            Ok(c) => c,
                                            _ => Ordering::Equal,
                                        }
                                    })
                                }                                
                            })
                            .unwrap_or(None);
                        // This removes the oldest file if there are 5 files in the folder
                        if let Some(de) = r {std::fs::remove_file(de.path()).ok();}
                    })
                    .await
                    .ok();
                    Ok(ppp)
                }
            })
            .await
            .map_err(|e| RexecError {
                code: RexecErrorType::FailedFileCreate,
                message: e.to_string(),
            });
        p
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
}
