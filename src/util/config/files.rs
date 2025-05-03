use std::path::PathBuf;

use crate::error::{RexecError, RexecErrorType};

fn usize_from_string(src: &String) -> Option<usize>{
    None
}

pub(super) fn from_file_global(file: &PathBuf, cfg: &mut super::Config) -> Result<(), RexecError> {
    let mut file = file.clone();
    file.push("config.json");
    let content = std::fs::read_to_string(file).map_err(|e| RexecError {
        code: RexecErrorType::FailedFileRead,
        message: e.to_string(),
    })?;

    let config: serde_json::Value =
        serde_json::from_str(content.as_str()).map_err(|e| RexecError {
            code: RexecErrorType::FailedFileRead,
            message: e.to_string(),
        })?;

    config
        .get("verbosity_level")
        .and_then(|v| v.as_str())
        .map(|v| cfg.verbosity_level = v.to_string());

    config["path"]["log_dir"].as_str().map(|s| {
        if let Some(new_log) = cfg
            .path
            .install_dir
            .as_os_str()
            .to_str()
            .map(|i| s.replace("${install}", i))
        {
            cfg.path.log_dir = PathBuf::from(new_log)
        } else {
            cfg.path.log_dir = PathBuf::from(s)
        }
    });
    config["path"]["config_dir"].as_str().map(|s| {
        if let Some(config_dir) = cfg
            .path
            .install_dir
            .as_os_str()
            .to_str()
            .map(|i| s.replace("${install}", i))
        {
            cfg.path.config_dir = PathBuf::from(config_dir)
        } else {
            cfg.path.config_dir = PathBuf::from(s)
        }
    });
    config["net"]["ip"].as_str().map(|ip| cfg.net.ip = ip.to_string());
    config["net"]["port"].as_u64().map(|port| cfg.net.port  = port as u16 );
    config["net"]["json_default_limit"]
        .as_str()
        .and_then(|s| s.parse::<usize>().ok())
        .map(|dl| cfg.net.json_default_limit = dl);

    Ok(())
}

fn fs_from_jason_string(content: &str, cfg: &super::Config) -> Result<super::FsConfig, RexecError>{
    let mut fs: super::FsConfig =
    serde_json::from_str(content).map_err(|e| RexecError {
        code: RexecErrorType::FailedFileRead,
        message: e.to_string(),
    })?;
    let install_dir = cfg.path.install_dir.as_os_str().to_str().unwrap_or("./");
    for v in fs.entries.values_mut(){
        *v = v.replace("${install}", install_dir);
    }
    Ok(fs)
}

pub(super) fn from_file_filesystem(
    file: &PathBuf,
    cfg: &mut super::Config,
) -> Result<(), RexecError> {
    let mut file = file.clone();
    file.push("filesystem.json");
    let content = std::fs::read_to_string(file).map_err(|e| RexecError {
        code: RexecErrorType::FailedFileRead,
        message: e.to_string(),
    })?;
    cfg.fs = fs_from_jason_string(content.as_str(), cfg)?;
    Ok(())
}

pub(super) fn from_file_process(
    file: &PathBuf,
    cfg: &mut super::Config,
) -> Result<(), RexecError> {
    let mut file = file.clone();
    file.push("process.json");
    let content = std::fs::read_to_string(file).map_err(|e| RexecError {
        code: RexecErrorType::FailedFileRead,
        message: e.to_string(),
    })?;
    cfg.proc = serde_json::from_str(&content)
    .map_err(|e| RexecError{code: RexecErrorType::FailedFileRead, message: e.to_string()})?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::util::config::{default_chunk_size, default_max_file, default_max_single_chunk, default_metadata_limit, Config, LogTimeStamp, ProcConfig};

    #[test]
    fn test_fs_config(){
        let cfg = Config::default(Some(PathBuf::from("/opt/rexec")));

        let fs_content = r#"
     {
        "entries":{
            "foo": "${install}/var/fs/foo",
            "bar": "${install}/var/fs/baar",
            "boo": "/tmp/rexec/fs/boo"
        }
    }"#;
        let fs = fs_from_jason_string(fs_content,&cfg).unwrap();
        assert!(fs.entries.is_empty() == false);
        assert_eq!(fs.entries.get("foo").unwrap(),"/opt/rexec/var/fs/foo");
        assert_eq!(fs.entries.get("bar").unwrap(),"/opt/rexec/var/fs/baar");
        assert_eq!(fs.entries.get("boo").unwrap(),"/tmp/rexec/fs/boo");
        assert_eq!(fs.chunk_size, default_chunk_size());
        assert_eq!(fs.max_file, default_max_file());
        assert_eq!(fs.max_single_chunk, default_max_single_chunk());
        assert_eq!(fs.metadata_limit,default_metadata_limit());
    }
    #[test]
    fn test_fs_config_modified(){
        let cfg = Config::default(Some(PathBuf::from("/opt/rexec")));

        let fs_content = r#"
    {
        "entries":{
            "foo": "${install}/var/fs/foo",
            "bar": "${install}/var/fs/baar",
            "boo": "/tmp/rexec/fs/boo"
        },
        "chunk_size": 1122,
        "max_file": 90,
        "max_single_chunk": 2048,
        "metadata_limit": 800        
    }"#;
    let fs = fs_from_jason_string(fs_content,&cfg).unwrap();
        assert!(fs.entries.is_empty() == false);
        assert_eq!(fs.entries.get("foo").unwrap(),"/opt/rexec/var/fs/foo");
        assert_eq!(fs.entries.get("bar").unwrap(),"/opt/rexec/var/fs/baar");
        assert_eq!(fs.entries.get("boo").unwrap(),"/tmp/rexec/fs/boo");
        assert_eq!(fs.chunk_size, 1122);
        assert_eq!(fs.max_file, 90);
        assert_eq!(fs.max_single_chunk, 2048);
        assert_eq!(fs.metadata_limit,800);
    }
    #[test]
    fn test_file_proc_config(){
        let proc_str = r#"
        {
          "max_log_files": 3,
          "log_timestamp": "Min",
          "stdin_capacity": 32,
          "bcast_capacity": 12
        }
        "#;
        let proc: ProcConfig  = serde_json::from_str(&proc_str).unwrap();
        assert_eq!(proc.bcast_capacity, 12);
        assert_eq!(proc.log_timestamp, LogTimeStamp::Min);
        assert_eq!(proc.max_log_files, 3);
        assert_eq!(proc.stdin_capacity, 32);
    }
}
