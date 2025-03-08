/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

use clap::Parser;

#[derive(Parser,Clone,Debug)]
#[command(version, about, long_about=None)]
pub struct Config{
    ///IP address to bind to
    #[arg(short, long, default_value_t="127.0.0.1")]
    pub ip: String,
    ///IP port to bind to
    #[arg(short, long, default_value_t=8910)]
    pub port: u16,
    #[arg(short, long, default_value_t=8)]
    /// Size of the status message channel
    pub status_size: usize,
    /// Size of the stdout channel
    #[arg(short, long, default_value_t=8)]
    pub stdout_size: usize,
    /// Loggin verbosity level.
    #[arg(short, long, default_value_t="Info")]
    pub verbosity_level: String,
}

impl Config{
    pub fn for_addr(ip: String, port: u16)->Self {
        Config{ip, port, status_size: 10, stdout_size: 10, verbosity_level: "Info".to_string()}
    }
    pub fn from_env()->Self {
        return Config::parse();
        // let matches = App::new("rexec")
        //     .version(clap::crate_version!())
        //     .author(clap::crate_authors!())
        //     .about("Allows one to run executables remotely")
        //     .arg(Arg::with_name("IP_ADDRESS")
        //         .short('i')
        //         .long("ip")
        //         .help("Sets the IP address to bind to.")
        //         .default_value("0.0.0.0")
        //         .takes_value(true)
        //     ).arg(Arg::with_name("IP_PORT")
        //         .short('p')
        //         .long("port")
        //         .help("Sets the IP port to bind to.")
        //         .default_value("8910")
        //         .takes_value(true)
        //     ).arg(Arg::with_name("STATUS_SIZE")
        //         .long("status-size")
        //         .help("Sets the size of the status message channel")
        //         .default_value("8")
        //         .takes_value(true)
        //     ).arg(Arg::with_name("STDOUT_SIZE")
        //         .long("stdout-size")
        //         .help("Sets the size of the stdout channel")
        //         .default_value("8")
        //         .takes_value(true)
        //     ).arg(Arg::with_name("v")
        //         .short('v')
        //         .multiple(true)
        //         .help("Sets the level of verbosity")
        //     ).get_matches();

        // Config{
        //     ip: matches.value_of("IP_ADDRESS").unwrap().to_string(),
        //     port: matches.value_of("IP_PORT").unwrap().to_string().parse().unwrap_or(8910),
        //     status_size: matches.value_of("STATUS_SIZE").unwrap().parse().unwrap_or(8),
        //     stdout_size: matches.value_of("STDOUT_SIZE").unwrap().parse().unwrap_or(8),
        //     verbosity_level: matches.occurrences_of("v") as usize
        // }
    }
}