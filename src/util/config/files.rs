use std::path::PathBuf;

use crate::error::{RexecError, RexecErrorType};

pub(super) fn from_file_global(file: &PathBuf, cfg: &mut super::Config) -> Result<(), RexecError>{
    let mut file = file.clone();
    file.push("config.json");
    let content = std::fs::read_to_string(file)
    .map_err(|e| RexecError { code: RexecErrorType::FailedFileRead, message: e.to_string()})?;

    let config: serde_json::Value = serde_json::from_str(content.as_str())
    .map_err(|e| RexecError { code: RexecErrorType::FailedFileRead, message: e.to_string()})?;

    config.get("verbosity_level")
        .and_then(|v| v.as_str())
        .map(|v| cfg.verbosity_level = v.to_string());

    

    config["path"]["log_dir"].as_str()
        .map(|s| 
            if let Some(new_log) = cfg.path.install_dir.as_os_str().to_str()
            .map(|i| s.replace("${install}", i)){
                cfg.path.log_dir = PathBuf::from(new_log)
            }
            else { cfg.path.log_dir = PathBuf::from(s) }
        );
        // "path":{
        //     "log_dir": "${install}/var/log",
        //     "config_dir": "${install}/etc"
        // },
        // "net": {
        //     "ip": "localhost",
        //     "port": 8081,
        //     "json_default_limit": "4k"
        // }

    Ok(())
}

pub(super) fn from_file_filesystem(file: &PathBuf, cfg: &mut super::Config) -> Result<(), RexecError>{
    let mut file = file.clone();
    file.push("global.json");
    let content = std::fs::read_to_string(file)
    .map_err(|e| RexecError { code: RexecErrorType::FailedFileRead, message: e.to_string()})?;

    let config: serde_json::Value = serde_json::from_str(content.as_str())
    .map_err(|e| RexecError { code: RexecErrorType::FailedFileRead, message: e.to_string()})?;

    config.get("verbosity_level")
        .and_then(|v| v.as_str())
        .map(|v| cfg.verbosity_level = v.to_string());

    Ok(())
}