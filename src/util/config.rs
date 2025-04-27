/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

use clap::{command, Arg};
use std::{collections::HashMap, path::PathBuf};
#[derive(Clone, Debug)]
pub struct IoConfig {
    pub stdin_capasity: usize,
    pub bcast_capasity: usize,
}
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
pub struct PathConfig {
    pub install_dir: PathBuf,
    pub log_dir: PathBuf,
    pub config_dir: PathBuf,
    pub max_log_files: usize,
    pub log_timestamp: LogTimeStamp,
}
pub type UrlPathMap = HashMap<String, PathBuf>;
#[derive(Clone, Debug)]
pub struct FsConfig {
    pub entries: UrlPathMap,
    pub chunk_size: usize,
    pub metadata_limit: usize,
    pub max_file: usize,
    pub max_single_chunk: usize,
}
#[derive(Clone, Debug)]
pub struct Config {
    pub verbosity_level: String,
    pub install_dir: PathBuf,
    pub path: PathConfig,
    pub net: NetConfig,
    pub io: IoConfig,
    pub fs: FsConfig,
}

impl Config {
    pub fn new() -> Self {
        let install_dir = std::env::current_exe()
        .ok()
        .map(|mut p| {// chop the executable name
                p.pop();
                p
        })
        .unwrap_or(PathBuf::from("."));

        Self {
            verbosity_level: "debug".to_string(),
            install_dir: install_dir.clone(),
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
                entries: HashMap::from([
                    ("foo".to_string(), {
                        let mut foo = install_dir.clone();
                        foo.push("var");
                        foo.push("fs");
                        foo.push("foo");
                        foo
                    }),
                    ("bar".to_string(), {
                        let mut bar = install_dir.clone();
                        bar.push("var");
                        bar.push("fs");
                        bar.push("bar");
                        bar
                    }),
                ]),
                chunk_size: 1000 * 1024,
                metadata_limit: 1024,
                max_file: 8_000_000*1024, // 8Gb by default
                max_single_chunk: 1000*1024, // All files smaller than this will be sent as one chunk and not streamed.
            },
        }
    }
    pub fn apply_commandline(mut self) -> Self {
        let matches = command!("rexec")
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
                    .long("verbose")
                    .help("Sets the level of verbosity")
                    .required(false),
            )
            .arg(
                Arg::new("install-directory")
                    .short('d')
                    .long("install-directory")
                    .help("Sets the installation directory. Default is the executable's directory.")
                    .required(false),
            )
            .get_matches();
        matches.get_one::<String>("install-directory").map(|s| self.install_dir = PathBuf::from(s));
        matches.get_one::<String>("log-level").map(|s| {self.verbosity_level = s.to_string();});
        matches.get_one::<String>("ip").map(|s| {self.net.ip = s.to_string();});
        matches.get_one::<u16>("port").map(|u| {self.net.port = *u;});
        self
    }
}
