/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

mod files;
use clap::{command, Arg, ArgMatches};
use log::error;
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

#[derive(Clone, Debug)]
pub struct NetConfig {
    pub ip: String,
    pub port: u16,
    pub json_default_limit: usize,
}
#[derive(Clone, Debug)]
pub enum LogTimeStamp {
    Sec,
    Min,
    Hour,
    Day,
}
#[derive(Clone, Debug)]
pub struct IoConfig {
    pub stdin_capasity: usize,
    pub bcast_capasity: usize,
}
#[derive(Clone, Debug)]
pub struct PathConfig {
    pub install_dir: PathBuf,
    pub log_dir: PathBuf,
    pub config_dir: PathBuf,
    pub max_log_files: usize,
    pub log_timestamp: LogTimeStamp,
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
    pub io: IoConfig,
    pub fs: FsConfig,
}

impl Config {
    pub fn new() -> Self{
        let cmd_line = Config::command_line_args();
        let install_dir = cmd_line.get_one::<String>("install-dir").map(|s| PathBuf::from(s));
        let cmd = Config::default(install_dir);

        cmd
        .apply_file()
        .apply_commandline(&cmd_line)
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
            verbosity_level: "debug".to_string(),
            net: NetConfig {
                ip: "0.0.0.0".to_string(),
                port: 8910,
                json_default_limit: 4096,
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
                max_log_files: 5,
                log_timestamp: LogTimeStamp::Day,
            },
            io: IoConfig {
                stdin_capasity: 32,
                bcast_capasity: 128,
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
    
    fn command_line_args()->ArgMatches{
        command!("rexec")
            .version(clap::crate_version!())
            .author(clap::crate_authors!())
            .about("Allows one to run executables remotely")
            .arg(
                Arg::new("ip")
                    .short('i')
                    .long("ip")
                    .help("Sets the IP address to bind to.")
                    .required(false),
            )
            .arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .help("Sets the IP port to bind to.")
                    .required(false),
            )
            .arg(
                Arg::new("log-level")
                    .short('v')
                    .long("log-level")
                    .help("Sets the level of verbosity (info, debug, trace)")
                    .required(false),
            )
            .arg(
                Arg::new("install-dir")
                    .short('d')
                    .long("install-dir")
                    .help("Sets the installation directory. Default is the executable's directory.")
                    .required(false),
            )
            .get_matches()
    }
    
    fn apply_commandline(mut self, matches: &ArgMatches) -> Self {
        matches.get_one::<String>("log-level").map(|s| {self.verbosity_level = s.to_string();});
        matches.get_one::<String>("ip").map(|s| {self.net.ip = s.to_string();});
        matches.get_one::<u16>("port").map(|u| {self.net.port = *u;});
        self
    }
    fn apply_file(mut self)->Self{
        let mut file = self.path.config_dir.clone();
        file.push("init.d");
        if let Err(e) = files::from_file_global(&file, &mut self)
        {
            println!("Failed to read global config from dir {:?} {e}", &file);
        }
        if let Err(e) = files::from_file_filesystem(&file, &mut self)
        {
            println!("Failed to read global config from dir {:?} {e}", &file);
        }
        self
    }
}
