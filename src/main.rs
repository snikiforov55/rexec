/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

use log::{info, debug};
use register::create_register_ref;
use simplelog::*;
use std::{fs::File, sync::Arc};

mod error;
mod webapi;
pub(crate) mod util;
pub(crate) mod proc;
pub(crate) mod exec;
pub(crate) mod register;

use util::config::Config;


fn main() {
    let config = Config::from_env();

    let log_level = match config.verbosity_level.to_lowercase().as_str(){
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
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
        port: {}",
          &config.net.ip,
          &config.net.port);

    debug!("Full config:\n{:?}", &config);

    let register = create_register_ref();
    let conf_ref = Arc::new(config);
    let api = webapi::create_server(conf_ref.clone(),register.clone()).unwrap();

    let job = async{
        // let (res_broker, res_api) = futures::join!(broker.start(), api);
        // match res_broker{
        //     Ok(_)=> info!("Broker finished"),
        //     Err(e) => error!(target:"main","Broker finished with error {}", e.to_string()),
        // }
        // match res_api{
        //     Ok(_)=> info!("API finished"),
        //     Err(e) => error!(target:"main","API finished with error {}", e.to_string()),
        // }
        api.await.ok()
    };
    tokio::runtime::Runtime:: new()
        .expect("Failed to create Tokio runtime")
        .block_on(job);
}
