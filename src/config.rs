/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

use clap::{arg, command};
#[derive(Clone)]
pub struct Config{
    pub ip: String,
    pub port: u16,
    pub status_size: usize,
    pub stdout_size: usize,
    pub verbosity_level: String,
}

impl Config{
    pub fn for_addr(ip: String, port: u16)->Self {
        Config{ip, port, status_size: 10, stdout_size: 10, verbosity_level: "Info".to_string()}
    }
    pub fn from_env()->Self {
        let matches = command!("rexec")
            .version(clap::crate_version!())
            .author(clap::crate_authors!())
            .about("Allows one to run executables remotely")
            .arg(arg!(-i --ip <IPADDR> "Sets the IP address to bind to.")
                .default_value("0.0.0.0")
            ).arg(arg!(-p --port <PORT> "Sets the IP port to bind to.")
                .default_value("8910")
            ).arg(arg!(--status-size <STATUS_SIZE> "Sets the size of the status message channel")
                .default_value("8")
            ).arg(arg!(--stdout-size <STDOUT_SIZE> "Sets the size of the stdout channel")
                .default_value("8")
            ).arg(arg!(-v --verbose <VERBOSITY_LEVEL> "Sets the level of verbosity")
            ).get_matches();

        Config{
            ip: matches.get_one::<String>("ip").unwrap_or(&"0.0.0.0".to_string()).to_string(),
            port: *matches.get_one::<u16>("port").unwrap_or(&8910),
            status_size: *matches.get_one::<usize>("status-size").unwrap_or(&8),
            stdout_size: *matches.get_one::<usize>("stdout-size").unwrap_or(&8),
            verbosity_level: matches.get_one::<String>("verbose").unwrap_or(&"info".to_string()).to_string()
        }
    }
}
