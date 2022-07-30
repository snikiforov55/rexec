/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */
#![recursion_limit="256"]

use crate::broker::Broker;
use futures::channel::{mpsc, oneshot};
use crate::process::ProcessCreateMessage;
use crate::broker::Shutdown;
use crate::webapi::WebApi;
use crate::config::Config;

mod error;
mod webapi;
pub(crate) mod process;
mod broker;
mod config;
use log::{info, error};
use simplelog::*;
use std::fs::File;

fn main() {
    let config = Config::from_env();

    let log_level = match config.verbosity_level{
        2 => LevelFilter::Trace,
        1 => LevelFilter::Debug,
        0 => LevelFilter::Info,
        _ => LevelFilter::Trace,
    };

    CombinedLogger::init(vec![
        TermLogger::new(
            log_level,
            simplelog::Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto
        ),
        WriteLogger::new(
            log_level,
            simplelog::Config::default(),
            File::create("rexec.log").unwrap(),
        ),
    ]).unwrap();

    info!("Version: {}",env!("CARGO_PKG_VERSION"));
    info!("Starting with configuration: \
        ip: {}, \
        port: {}, \
        status_size: {}, \
        stdout_size: {}",
          &config.ip,
          &config.port,
          &config.status_size,
          &config.stdout_size);

    let (create_tx, create_rx) = mpsc::channel::<ProcessCreateMessage>(10);
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<Shutdown>();
    let broker = Broker::new(create_rx, shutdown_rx, config.clone());
    let api = WebApi{create_tx, shutdown_tx, config: config.clone()};

    let job = async{
        let (res_broker, res_api) = futures::join!(broker.start(), api.start());
        match res_broker{
            Ok(_)=> info!("Broker finished"),
            Err(e) => error!(target:"main","Broker finished with error {}", e.to_string()),
        }
        match res_api{
            Ok(_)=> info!("API finished"),
            Err(e) => error!(target:"main","API finished with error {}", e.to_string()),
        }
    };
    tokio::runtime::Runtime:: new()
        .expect("Failed to create Tokio runtime")
        .block_on(job);
}
