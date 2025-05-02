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

pub(super) fn from_file_filesystem(
    file: &PathBuf,
    cfg: &mut super::Config,
) -> Result<(), RexecError> {
    let mut file = file.clone();
    file.push("global.json");
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

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_usize_from_string(){
        assert_eq!(4096, "4096".parse::<usize>().unwrap())
    }
}
