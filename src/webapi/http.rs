use actix_web::{web, web::Data};
use actix_web::HttpResponse;
use log::debug;
use std::sync::Arc;
use tokio::time::Duration;

use crate::exec::execute::start;
use crate::proc::comm::StopMessage;
use crate::proc::description::ProcessDescription;
use crate::register::RegisterRef;
use crate::util::config::Config;
use super::files;

pub(super) fn configure_http(cfg: &mut web::ServiceConfig){
    cfg.service(
web::scope("/process")
            .service(
        web::resource("")
                    .route(web::get().to(try_get_status_all)),
            )
            .service(
        web::scope("/{alias}")
                .service(web::resource("")
                    .route(web::post().to(try_create_process))
                    .route(web::delete().to(try_stop_process))
                    .route(web::get().to(try_get_status)),
                )
                .service(
                web::scope("/log")
                    .service(web::resource("")
                        .route(web::get().to(files::send::nope)),
                    )
                    .service(web::resource("/last")
                         .route(web::get().to(files::send::nope)),
                    )
                    .service(web::resource("/{id}")
                        .route(web::get().to(files::send::list_log_files)),
                    )
                ),
            ),
    );
}
pub(super) async fn try_create_process(
    conf: Data<Arc<Config>>,
    reg: Data<RegisterRef>,
    item: web::Json<ProcessDescription>,
    alias: web::Path<String>,
) -> HttpResponse {
    debug!("POST for alias {alias} for \n{:?}", item);
    let mut i = item.into_inner();
    i.alias = alias.to_string();
    if i.alias.is_empty() {
        HttpResponse::InternalServerError().body("Invalid alias provided.")
    } else {
        match start(conf.get_ref(), reg.get_ref(), &i).await {
            Ok(_) => HttpResponse::Ok().body(()),
            Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
        }
    }
}

pub(super) async fn try_stop_process(
    reg: Data<RegisterRef>,
    alias: web::Path<String>,
) -> HttpResponse {
    debug!("DELETE for alias {alias}");

    //Lock the Register for a very short time, only to get the channels.
    //After this operation the channels will be consumed.
    //The next DELETE request will no do anything but returning the NotFound response.
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

pub(super) async fn try_get_status(
    reg: Data<RegisterRef>,
    alias: web::Path<String>,
) -> HttpResponse {
    debug!("GET for alias {alias}");

    let info = match reg.get_ref().read().await.get(&alias) {
        Some(p) => Some(p.desc.clone()),
        None => None,
    };
    match info {
        Some(i) => {
            debug!("GET is responding with the Description {:?}", i);
            HttpResponse::Ok().json(i)
        }
        None => {
            debug!("GET Process {alias} not found");
            HttpResponse::NotFound().body(())
        }
    }
}

pub(super) async fn try_get_status_all(reg: Data<RegisterRef>) -> HttpResponse {
    debug!("GET for all");

    let all = reg.get_ref().read().await.get_all_desc();
    debug!("GET all processes {:?}", all);
    HttpResponse::Ok().json(all)
}
