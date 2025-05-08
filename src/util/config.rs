/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

mod files;
use pico_args::Arguments;
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

#[derive(Clone, Debug)]
pub struct NetConfig {
    pub ip: String,
    pub port: u16,
    pub json_default_limit: usize,
    pub allowed_cors_domains: String,
    pub response_timeout: u64,

}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub enum LogTimeStamp {
    Sec,
    Min,
    Hour,
    Day,
}
#[derive(Clone, Debug, Deserialize)]
pub struct ProcConfig {
    #[serde(default = "default_stdin_capacity")]
    pub stdin_capacity: usize,
    #[serde(default = "default_bcast_capacity")]
    pub bcast_capacity: usize,
    #[serde(default = "default_max_log_files")]
    pub max_log_files: usize,
    #[serde(default = "default_log_timestamp")]
    pub log_timestamp: LogTimeStamp,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}
fn default_stdin_capacity()->usize{
    1024
}
fn default_bcast_capacity()->usize{
    128
}
fn default_max_log_files()->usize{
    5
}
fn default_log_timestamp()->LogTimeStamp{
    LogTimeStamp::Day
}
fn default_timeout() -> u64{
    5
}
#[derive(Clone, Debug)]
pub struct PathConfig {
    pub install_dir: PathBuf,
    pub log_dir: PathBuf,
    pub config_dir: PathBuf,
    
}
pub type UrlPathMap = HashMap<String, String>;
#[derive(Clone, Debug, Deserialize)]
pub struct FsConfig {
    #[serde(default)]
    pub entries: UrlPathMap,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    #[serde(default = "default_metadata_limit")]    
    pub metadata_limit: usize,
    #[serde(default = "default_max_file")]
    pub max_file: usize,
    #[serde(default = "default_max_single_chunk")]
    pub max_single_chunk: usize,
}
fn default_chunk_size()->usize{
    1000 * 1024
}
fn default_metadata_limit()->usize{
    1024
}
fn default_max_file()->usize{
    8_000_000*1024 // 8Gb by default
}
fn default_max_single_chunk()->usize{
    1000*1024 // All files smaller than this will be sent as one chunk and not streamed.
}
#[derive(Clone, Debug)]
pub struct Config {
    pub verbosity_level: String,
    pub path: PathConfig,
    pub net: NetConfig,
    pub proc: ProcConfig,
    pub fs: FsConfig,
}

impl Config {
    pub fn new() -> Self{
        let mut pargs = pico_args::Arguments::from_env();

        let install_dir: Option<PathBuf> = pargs.opt_value_from_str("--install-dir")
            .unwrap_or(None);
        let cmd = Config::default(install_dir);
        cmd
        .apply_file()
        .apply_commandline(pargs)
    }
    fn default(install_dir : Option<PathBuf>) -> Self {
        let install_dir = 
            install_dir.unwrap_or(std::env::current_exe()
            .ok()
            .map(|mut p| {// chop the executable name
                    p.pop();
                    p
            })
            .unwrap_or(PathBuf::from("."))
        );

        Self {
            verbosity_level: "info".to_string(),
            net: NetConfig {
                ip: "0.0.0.0".to_string(),
                port: 8910,
                json_default_limit: 4096,
                allowed_cors_domains: "*".to_string(),
                response_timeout: 25,
            },
            path: PathConfig {
                log_dir: {
                    let mut p = install_dir.clone();
                    p.push("var");
                    p.push("log");
                    p
                },
                config_dir: {
                    let mut p = install_dir.clone();
                    p.push("etc");
                    p
                },
                install_dir: install_dir.clone(),
            },
            proc: ProcConfig {
                stdin_capacity: default_stdin_capacity(),
                bcast_capacity: default_bcast_capacity(),
                max_log_files: default_max_log_files(),
                log_timestamp: default_log_timestamp(),
                timeout: default_timeout(),
            },
            fs: FsConfig {
                entries: HashMap::new(),
                chunk_size: default_chunk_size(),
                metadata_limit: default_metadata_limit(),
                max_file: default_max_file(),
                max_single_chunk: default_max_single_chunk(), 
            },
        }
    }    
    fn apply_commandline(mut self, mut pargs: Arguments) -> Self {
        pargs.opt_value_from_str("--log-level").unwrap_or(None).map(|s: String| self.verbosity_level = s);
        pargs.opt_value_from_str("--ip").unwrap_or(None).map(|s: String| self.net.ip = s);
        pargs.opt_value_from_str("--port").unwrap_or(None).map(|p: u16| self.net.port = p);

        self
    }
    fn apply_file(mut self)->Self{
        let mut file = self.path.config_dir.clone();
        file.push("init.d");
        if let Err(e) = files::from_file_global(&file, &mut self)
        {
            println!("Failed to read global config from dir {:?}/confg.json {e}", &file);
        }
        if let Err(e) = files::from_file_filesystem(&file, &mut self)
        {
            println!("Failed to read global config from dir {:?}/filesystem.json {e}", &file);
        }
        if let Err(e) = files::from_file_process(&file, &mut self)
        {
            println!("Failed to read global config from dir {:?}/process.json {e}", &file);
        }
        self
    }
}
