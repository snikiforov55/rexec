use std::sync::Arc;
use actix_web::web::{self, Data};
use actix_web::HttpResponse;
use actix_web_lab::extract::Path;
use log::debug;
use tokio::time::Duration;

use crate::exec::execute::start;
use crate::proc::description::ProcessDescription;
use crate::proc::comm::StopMessage;
use crate::register::RegisterRef;
use crate::util::config::Config;

pub(super) async fn try_create_process(
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

pub(super) async fn try_stop_process(reg: Data<RegisterRef>, Path((alias,)): Path<(String,)>) -> HttpResponse {
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