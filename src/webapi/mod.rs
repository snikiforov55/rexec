/*
 * Copyright (c) 2020-2025. Stanislav Nikiforov
 */
mod http;

use std::sync::Arc;
use actix_web::web;
use actix_web::web::Data;
use actix_web::App;
use actix_web::HttpServer;

use crate::util::config::Config;
use crate::register::RegisterRef;

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
                    .route(web::post().to(http::try_create_process))
                    .route(web::delete().to(http::try_stop_process)),
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
