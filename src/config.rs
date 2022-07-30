/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

use clap::{App, Arg};

#[derive(Clone)]
pub struct Config{
    pub ip: String,
    pub port: u16,
    pub status_size: usize,
    pub stdout_size: usize,
    pub verbosity_level: usize,
}

impl Config{
    pub fn for_addr(ip: String, port: u16)->Self {
        Config{ip, port, status_size: 10, stdout_size: 10, verbosity_level: 0 }
    }
    pub fn from_env()->Self {
        let matches = App::new("rexec")
            .version(clap::crate_version!())
            .author(clap::crate_authors!())
            .about("Allows one to run executables remotely")
            .arg(Arg::with_name("IP_ADDRESS")
                .short('i')
                .long("ip")
                .help("Sets the IP address to bind to.")
                .default_value("0.0.0.0")
                .takes_value(true)
            ).arg(Arg::with_name("IP_PORT")
                .short('p')
                .long("port")
                .help("Sets the IP port to bind to.")
                .default_value("8910")
                .takes_value(true)
            ).arg(Arg::with_name("STATUS_SIZE")
                .long("status-size")
                .help("Sets the size of the status message channel")
                .default_value("8")
                .takes_value(true)
            ).arg(Arg::with_name("STDOUT_SIZE")
                .long("stdout-size")
                .help("Sets the size of the stdout channel")
                .default_value("8")
                .takes_value(true)
            ).arg(Arg::with_name("v")
                .short('v')
                .multiple(true)
                .help("Sets the level of verbosity")
            ).get_matches();

        Config{
            ip: matches.value_of("IP_ADDRESS").unwrap().to_string(),
            port: matches.value_of("IP_PORT").unwrap().to_string().parse().unwrap_or(8910),
            status_size: matches.value_of("STATUS_SIZE").unwrap().parse().unwrap_or(8),
            stdout_size: matches.value_of("STDOUT_SIZE").unwrap().parse().unwrap_or(8),
            verbosity_level: matches.occurrences_of("v") as usize
        }
    }
}