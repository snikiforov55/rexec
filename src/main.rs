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

fn main() {
    let config = Config::for_addr("127.0.0.1".to_string(), 8910);

    let (create_tx, create_rx) = mpsc::channel::<ProcessCreateMessage>(10);
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<Shutdown>();
    let broker = Broker::new(create_rx, shutdown_rx, config.clone());
    let api = WebApi{create_tx, shutdown_tx, config: config.clone()};

    let job = async{
        let (res_broker, res_api) = futures::join!(broker.start(), api.start());
        match res_broker{
            Ok(_)=> println!("Broker finished"),
            Err(e) => println!("Broker finished with error {}", e.to_string()),
        }
        match res_api{
            Ok(_)=> println!("API finished"),
            Err(e) => println!("API finished with error {}", e.to_string()),
        }
    };
    tokio::runtime::Runtime:: new()
        .expect("Failed to create Tokio runtime")
        .block_on(job);
}
