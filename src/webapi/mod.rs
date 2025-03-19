/*
 * Copyright (c) 2020-2025. Stanislav Nikiforov
 */

use actix_web_lab::extract::Path;
use actix_web::App;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use actix_web::web;

use futures::SinkExt;

use std::str::FromStr;
use futures::channel::mpsc;
use futures::channel::oneshot;
use std::sync::Arc;
use log::{info,error,debug};

use crate::broker::Shutdown;
use crate::config;
use crate::process::{ProcessCreateMessage, StartConfirmation};
use crate::process::description::ProcessDescription;
use crate::error::{RexecError, RexecErrorType};
use crate::config::Config;

type CreateTx=mpsc::Sender<ProcessCreateMessage>;
type ShutdownTx = oneshot::Sender<Shutdown>;


pub struct WebApi{
    pub(crate) create_tx: CreateTx,
    pub(crate) shutdown_tx: ShutdownTx,
    pub(crate) config: Config,
}

async fn try_create_process(item: web::Json<ProcessDescription>, Path((alias,)): Path<(String,)>) ->HttpResponse{
    println!("alias {alias} for \n{:?}", item);
    HttpResponse::Ok().body(())
}

pub fn create_server(config: &Config)->std::io::Result<actix_web::dev::Server>{
    let out = HttpServer::new(|| {
        App::new()
        // enable loggerstart
        //.wrap(middleware::Logger::default())
        .app_data(web::JsonConfig::default().limit(4096)) // <- limit size of the payload (global configuration)
        .service(web::resource("/process/{alias}").route(web::post().to(try_create_process)))
        // .service(
        //     web::resource("/extractor2")
        //         .app_data(web::JsonConfig::default().limit(1024)) // <- limit size of the payload (resource level)
        //         .route(web::post().to(extract_item)),
        // )
        //.service(web::resource("/manual").route(web::post().to(index_manual)))
        //.service(web::resource("/").route(web::post().to(index)))
    })
    .bind((config.ip.clone(), config.port))?
    .run();
    Ok(out)
}

impl WebApi{
    

    fn parse_body<R>( bytes: &mut R) -> Result<ProcessDescription, RexecError> 
        where R: std::io::Read{
        //debug!("Received body: {}",body.);
        let desc : ProcessDescription = serde_json::from_reader(&mut * bytes)
            .map_err(|e| {
                let mut body = String::new();
                bytes.read_to_string(&mut body);
                info!("Failed to parse JSON from a request body {} from string. Reason {}",
                    body,
                    &e.to_string());
                RexecError::code(RexecErrorType::InvalidCreateProcessRequest)
            })?;
        Ok(desc)
    }
    // pub async fn start<>(self) ->Result<(), RexecError>{
    //     let ip = IpAddr::from_str(self.config.ip.as_str())
    //         .map_err(|e| {
    //             error!("FailedToCreateSocketAddress from {} reason {}",
    //                    self.config.ip, e.to_string());
    //             RexecError::code(RexecErrorType::FailedToCreateSocketAddress)
    //         })?;
    //     let address = SocketAddr::new(ip, self.config.port);

    //     let the_arc = Arc::new(self);
        
    //     info!("Starting service on {}", address.to_string());
    //     let listener = TcpListener::bind(&address)
    //         .await
    //         .map_err(|e| {
    //             log::error!("FailedToStartWebServer {}", &e.to_string());
    //             RexecError::code_msg(
    //                 RexecErrorType::FailedToStartWebServer,
    //                 e.to_string())
    //         })?;

    //     loop{
    //         let (stream, _) = listener
    //             .accept()
    //             .await            
    //             .map_err(|e| {
    //                 log::error!("FailedToStartWebServer {}", &e.to_string());
    //                 RexecError::code_msg(
    //                     RexecErrorType::FailedToStartWebServer,
    //                     e.to_string())
    //             })?;
    //         let io = TokioIo::new(stream);
    //         let api = the_arc.clone();
    //         tokio::task::spawn(async move {
    //             let service = 
    //                 service_fn(| req: Request<IncomingBody>| {
    //                     let api2 = api.clone();
    //                     async move {
    //                         WebApi::router(api2, req).await
    //                     }
    //                 });
    //             if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
    //                 println!("Failed to serve connection: {:?}", err);
                    
    //             }
    //         });
    //     }   
    // }
    //     Ok(desc)
    // }
    // pub async fn start<>(self) ->Result<(), RexecError>{
    //     let ip = IpAddr::from_str(self.config.ip.as_str())
    //         .map_err(|e| {
    //             error!("FailedToCreateSocketAddress from {} reason {}",
    //                    self.config.ip, e.to_string());
    //             RexecError::code(RexecErrorType::FailedToCreateSocketAddress)
    //         })?;
    //     let address = SocketAddr::new(ip, self.config.port);

    //     let the_arc = Arc::new(self);
        
    //     info!("Starting service on {}", address.to_string());
    //     let listener = TcpListener::bind(&address)
    //         .await
    //         .map_err(|e| {
    //             log::error!("FailedToStartWebServer {}", &e.to_string());
    //             RexecError::code_msg(
    //                 RexecErrorType::FailedToStartWebServer,
    //                 e.to_string())
    //         })?;

    //     loop{
    //         let (stream, _) = listener
    //             .accept()
    //             .await            
    //             .map_err(|e| {
    //                 log::error!("FailedToStartWebServer {}", &e.to_string());
    //                 RexecError::code_msg(
    //                     RexecErrorType::FailedToStartWebServer,
    //                     e.to_string())
    //             })?;
    //         let io = TokioIo::new(stream);
    //         let api = the_arc.clone();
    //         tokio::task::spawn(async move {
    //             let service = 
    //                 service_fn(| req: Request<IncomingBody>| {
    //                     let api2 = api.clone();
    //                     async move {
    //                         WebApi::router(api2, req).await
    //                     }
    //                 });
    //             if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
    //                 println!("Failed to serve connection: {:?}", err);
                    
    //             }
    //         });
    //     }   
}


#[cfg(test)]
mod web_api_tests{
    use futures::StreamExt;

    use super::*;

    #[test]
    fn test_parse_body_full(){
        let body = r#"{
            "alias" : "test",
            "cmd": "shell",yper::body::Bytes::from(
            "args": [
                "ls",
                "arg1",
                "arg2",
                "arg3"
            ],
            "cwd": "here",
            "envs": {
                "PATH": "/bin",
                "SECRET_KEY": "QWE_YUI_345_GHJ_789"
            }
        }"#.to_string();
        let desc = WebApi::parse_body(&mut body.as_bytes()).unwrap();
        assert_eq!(desc.alias, "test".to_string());
        assert_eq!(desc.cmd, "shell".to_string());
        assert_eq!(desc.cwd, "here".to_string());
        assert_eq!(desc.args.len(), 4);
        assert_eq!(desc.envs.len(), 2);
    }
    #[test]
    fn test_parse_body_minimal(){
        let body = r#"{
            "alias" : "test",
            "cmd": "shell"
        }"#.to_string();
        let desc = WebApi::parse_body(&mut body.as_bytes()).unwrap();
        assert_eq!(desc.alias, "test".to_string());
        assert_eq!(desc.cmd, "shell".to_string());
        assert_eq!(desc.cwd, ".".to_string());
        assert_eq!(desc.args.len(), 0);
        assert_eq!(desc.envs.len(), 0);
    }
    #[test]
    fn test_parse_body_failing(){
        let body = r#"{
            "alias" : "test"
        }"#.to_string();
        let desc = WebApi::parse_body(&mut body.as_bytes());
        assert!(!desc.is_ok());
        matches!(desc.err().unwrap().code, RexecErrorType::InvalidCreateProcessRequest);
    }
    // #[test]
    // fn test_router_non_api(){
    //     let config = Config::for_addr("localhost".to_string(), 5566);
    //     let (create_tx, _create_rx) = mpsc::channel::<ProcessCreateMessage>(10);
    //     let (shutdown_tx, _shutdown_rx) = oneshot::channel::<Shutdown>();
    //     let api = WebApi{create_tx, shutdown_tx, config: config.clone()};
    //     let api_ref = Arc::new(api);

    //     let job = async{
    //         let req: Request<Full<Bytes>> = Request::get("http://localhost:5566/")
    //             .body(Full::from("".as_bytes()))
    //             .unwrap();
    //         let res = WebApi::router(api_ref.clone(),req).await.unwrap();
    //         matches!(res.status(), StatusCode::NOT_IMPLEMENTED);
    //         let req: Request<Full<Bytes>> = Request::builder()
    //             .uri("http://localhost:5566/process/1234")
    //             .body(Full::from("".as_bytes()))
    //             .unwrap();
    //         let res = WebApi::router(api_ref.clone(),req).await.unwrap();
    //         matches!(res.status(), StatusCode::NOT_IMPLEMENTED);

    //         let req: Request<Full<Bytes>> = Request::builder()
    //             .uri("http://localhost:5566/process/1234")
    //             .method("POST")
    //             .body(Full::from("".as_bytes()))
    //             .unwrap();
    //         let res = WebApi::router(api_ref.clone(),req).await.unwrap();
    //         matches!(res.status(), StatusCode::NOT_IMPLEMENTED);

    //         let req: Request<Full<Bytes>> = Request::builder()
    //             .uri("http://localhost:5566/process")
    //             .method("POST")
    //             .body(Full::from("".as_bytes()))
    //             .unwrap();
    //         let res = WebApi::router(api_ref.clone(),req).await.unwrap();
    //         matches!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

    //         Ok::<_,RexecError>(())
    //     };
    //     tokio::runtime::Runtime:: new()
    //         .expect("Failed to create Tokio runtime")
    //         .block_on(job).ok();
    // }
    // #[test]
    // fn test_router_api(){
    //     let config = Config::for_addr("localhost".to_string(), 5566);
    //     let (create_tx, mut create_rx) = mpsc::channel::<ProcessCreateMessage>(10);
    //     let (shutdown_tx, _shutdown_rx) = oneshot::channel::<Shutdown>();
    //     let api = WebApi{create_tx, shutdown_tx, config: config.clone()};
    //     let api_ref = Arc::new(api);

    //     let dummy_broker = async{
    //         if let Some(mut msg) = create_rx.next().await{
    //             msg.start_tx.take()
    //                 .unwrap()
    //                 .send(StartConfirmation::Started)
    //                 .unwrap_or(());
    //             msg.stdout_tx.disconnect();
    //         }else{ () }

    //     };
    //     let job = async{
    //         let req = Request::builder()
    //             .uri("http://localhost:5566/process")
    //             .method("POST")
    //             .body(full(r#"{"cmd":"ls","alias":"ls"}"#))
    //             .unwrap();
    //         let router = WebApi::router(api_ref.clone(),req);
    //         let (res, _dummy) = futures::join!(router,dummy_broker);

    //         matches!(res.unwrap().status(), StatusCode::OK);
    //     };
    //     tokio::runtime::Runtime:: new()
    //         .expect("Failed to create Tokio runtime")
    //         .block_on(job);
    // }
}