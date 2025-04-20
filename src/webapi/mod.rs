/*
 * Copyright (c) 2020-2025. Stanislav Nikiforov
 */
mod http;
mod files;

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
            .app_data(web::JsonConfig::default().limit(json_default_limit)) //limit size of the payload (global configuration)
            .configure(http::configure_http)
            .configure(files::configure_files)
            // .service(Files::new(
            //     "/process/{alias}/log/list", 
            //     &cfg.path.log_dir).show_files_listing())
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

