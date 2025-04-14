/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

use std::path::PathBuf;

use clap::{command, Arg};
use serde_json::to_string;
#[derive(Clone)]
pub struct IoConfig{
    pub stdin_capasity: usize,
    pub bcast_capasity: usize,
}
#[derive(Clone)]
pub struct NetConfig{
    pub ip: String,
    pub port: u16,
    pub json_default_limit: usize,
}

#[derive(Clone)]
pub enum LogTimeStamp{
    Sec,
    Min,
    Hour,
    Day
}
#[derive(Clone)]
pub struct PathConfig{
    pub install_dir: PathBuf,
    pub log_dir: PathBuf,
    pub config_dir: PathBuf,
    pub max_log_files: usize,
    pub log_timestamp: LogTimeStamp,
}
#[derive(Clone)]
pub struct Config {
    pub verbosity_level: String,
    pub path: PathConfig,
    pub net: NetConfig,
    pub io: IoConfig,
}


impl Config {
    pub fn from_env() -> Self {
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
        let install_dir = matches
        .get_one::<String>("install-directory")
        .map(|s| PathBuf::from(s))
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .map(|mut p| {p.pop(); p})// chop the executable name
        })
        .unwrap_or(PathBuf::from("."));
        Config {
            verbosity_level: matches
                .get_one::<String>("log-level")
                .unwrap_or(&"debug".to_string())
                .to_string(),
            net:NetConfig{
                ip: matches
                    .get_one::<String>("ip")
                    .unwrap_or(&"0.0.0.0".to_string())
                    .to_string(),
                port: *matches.get_one::<u16>("port").unwrap_or(&8910),
                json_default_limit: 4096,
            },
            path: PathConfig {
                log_dir: {let mut p = install_dir.clone();
                    p.push("var");
                    p.push("log");
                    p
                }, 
                config_dir: {let mut p = install_dir.clone();
                    p.push("etc");
                    p
                }, 
                install_dir,
                max_log_files: 5, 
                log_timestamp: LogTimeStamp::Day 
            },
            io: IoConfig { 
                stdin_capasity: 32, 
                bcast_capasity: 128 }
        }
    }
}
