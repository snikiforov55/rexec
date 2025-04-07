/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

use clap::{command, Arg};
#[derive(Clone)]
pub struct Config {
    pub ip: String,
    pub port: u16,
    pub verbosity_level: String,
    pub install_dir: Option<String>,
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

        Config {
            ip: matches
                .get_one::<String>("ip")
                .unwrap_or(&"0.0.0.0".to_string())
                .to_string(),
            port: *matches.get_one::<u16>("port").unwrap_or(&8910),
            verbosity_level: matches
                .get_one::<String>("log-level")
                .unwrap_or(&"debug".to_string())
                .to_string(),
            install_dir: matches
                .get_one::<String>("install-directory")
                .map(|s| s.to_string())
                .or_else(|| {
                    std::env::current_exe()
                        .ok()
                        .and_then(|p| p.to_str().map(|s| s.to_string()))
                }),
        }
    }
}
