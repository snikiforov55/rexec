/*
 * Copyright (c) 2020-2025. Stanislav Nikiforov
 */

use std::sync::Arc;

use actix_web::web;
use actix_web::web::Data;
use actix_web::App;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use actix_web_lab::extract::Path;

use tokio::{
    sync::mpsc,
    time::Duration,
};

use log::{debug, error, info};

use crate::util::config::Config;
use crate::exec::execute::start;
use crate::proc::comm::StopMessage;
use crate::proc::description::ProcessDescription;
use crate::register::RegisterRef;


async fn try_create_process(
    conf: Data<Arc<Config>>,
    reg: Data<RegisterRef>,
    item: web::Json<ProcessDescription>,
    Path((alias,)): Path<(String,)>,
) -> HttpResponse {
    debug!("POST for alias {alias} for \n{:?}", item);
    match start(conf.get_ref(), reg.get_ref(), &item.into_inner()).await {
        Ok(_) => HttpResponse::Ok().body(()),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
async fn try_stop_process(reg: Data<RegisterRef>, Path((alias,)): Path<(String,)>) -> HttpResponse {
    debug!("DELETE for alias {alias}");

    //Lock the Registed for a very short time, only to get the channels.
    //After this operation the channels will be consumed.
    //The next DELETE requiest will no do anything but returning the NotFound response.
    //The Process will be removed from the Registed in the execution context.
    let (stop_tx, exit_rx) = match reg.get_ref().write().await.get_mut(&alias) {
        Some(p) => (p.stop_tx.take(), p.exit_rx.take()),
        None => (None, None),
    };
    stop_tx.map(|tx| tx.send(StopMessage {}).ok());
    match exit_rx {
        Some(rx) => {
            tokio::select! {
                _ = rx => {
                    debug!("Confirmed process exit via the exit channel");
                    HttpResponse::Ok().body(())
                },
                _ = tokio::time::sleep(Duration::from_secs(25)) => {
                    debug!("Timeout waiting for the process to exit");
                    HttpResponse::RequestTimeout().body(())
                }
            }
        }
        _ => {
            debug!("Process {alias} not found");
            HttpResponse::NotFound().body(())
        }
    }
}

pub fn create_server(config: Arc<Config>, reg: RegisterRef) -> std::io::Result<actix_web::dev::Server> {
    let json_default_limit = config.net.json_default_limit;
    let cfg = config.clone();
    let out = HttpServer::new(move || {
        App::new()
            // enable loggerstart
            //.wrap(middleware::Logger::default())
            .app_data(Data::new(cfg.clone()))
            .app_data(Data::new(reg.clone()))
            .app_data(web::JsonConfig::default().limit(json_default_limit)) //todo. Read from the confgiuration. <- limit size of the payload (global configuration)
            .service(
                web::resource("/process/{alias}")
                    .route(web::post().to(try_create_process))
                    .route(web::delete().to(try_stop_process)),
            )
        // .service(
        //     web::resource("/extractor2")
        //         .app_data(web::JsonConfig::default().limit(1024)) // <- limit size of the payload (resource level)
        //         .route(web::post().to(extract_item)),
        // )
        //.service(web::resource("/manual").route(web::post().to(index_manual)))
        //.service(web::resource("/").route(web::post().to(index)))
    })
    .bind((config.net.ip.clone(), config.net.port))?
    .run();
    Ok(out)
}
#[cfg(test)]
mod web_api_tests {
    #[test]
    fn test_parse_body_full() {
    }
}
