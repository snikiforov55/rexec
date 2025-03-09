/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

use futures::SinkExt;
use hyper::{body::Incoming as IncomingBody, Request, Response, Method, StatusCode};
use hyper::body::{Buf, Bytes};
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use hyper::server::conn::http1;
use http_body_util::{BodyExt, Full, Empty};

use std::net::{SocketAddr, IpAddr};
use tokio::net::TcpListener;

use std::str::FromStr;
use futures::channel::mpsc;
use futures::channel::oneshot;
use std::sync::Arc;
use log::{info,error,debug};

use crate::broker::Shutdown;
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
type BoxBody = http_body_util::combinators::BoxBody<Bytes, RexecError>;
type RouterResponse = Result<Response<BoxBody>,RexecError>;

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

impl WebApi{
    async fn create_new_and_run<B>(api:  Arc<WebApi>, req: Request<B>) ->RouterResponse
        where B: BodyExt
        {
            //type Error = dyn BodyExt::Error + std::fmt::Display ;

        let (stdout_tx, stdout_rx) = mpsc::channel::<String>(api.config.stdout_size);

        //let bytes = hyper::body::to_bytes(req.into_body()).await?;
        let bytes = req.collect().await.map_err(|e  | {
            debug!("FailedToSendStartCommand {:#?}", &e);

            RexecError::code_msg(
                RexecErrorType::FailedToSendStartCommand,
                e.to_string())
        })?.aggregate();

        let res = async move{
            let desc = WebApi::parse_body(&mut bytes.reader())?;
            debug!("Sending start command for {}", &desc.alias);

            let mut create_tx = api.create_tx.clone();
            let (start_tx, start_rx) = oneshot::channel::<StartConfirmation>();

            create_tx.send(ProcessCreateMessage{
                desc,
                stdout_tx,
                start_tx: Some(start_tx)
            }).await.map_err(|e| {
                debug!("FailedToSendStartCommand {}", &e.to_string());

                RexecError::code_msg(
                    RexecErrorType::FailedToSendStartCommand,
                    e.to_string())
            })?;

            let start_status = start_rx.await.map_err(|e| {
                debug!("UnexpectedEof {}", &e.to_string());
                RexecError::code_msg(
                RexecErrorType::UnexpectedEof,
                e.to_string())
            })?;

            match start_status{
                StartConfirmation::Started => Ok(()),
                StartConfirmation::Error(e) => {
                    debug!("FailedToExecuteProcess {}", &e.to_string());
                    Err(RexecError::code_msg(
                        RexecErrorType::FailedToExecuteProcess,
                        e.to_string()))
                },
                StartConfirmation::AlreadyRunning => {
                    debug!("AlreadyRunning");
                    Err(RexecError::code(
                        RexecErrorType::AlreadyRunning))
                },
            }
        }.await;

        match res{
            Ok(_) => Ok(
                Response::new(BoxBody::wrap_stream(
                    stdout_rx.map(|s| {
                        debug!("{}",&s);
                        Ok::<_, hyper::Error>(format!("{}\n",s))
                    })))
            ),
            Err(e) => {
                let status = match e.code{
                    RexecErrorType::FailedToExecuteProcess => StatusCode::NOT_FOUND,
                    RexecErrorType::AlreadyRunning => StatusCode::CONFLICT,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };
                debug!("Sending HTTP status {}", &status);
                Ok(
                    hyper::Response::builder()
                        .status(status)
                        .body(full(e.to_string()))
                        .unwrap()
                )
            },
        }
    }

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
    async fn root<B>(req: Request<B>)->RouterResponse{
        debug!("Requested URL is not processed {}", req.uri());
        Ok(hyper::Response::builder()
            .status(StatusCode::NOT_IMPLEMENTED)
            .body(full("Invalid path."))
            .unwrap()
        )
    }
    async fn router<'a, B: hyper::body::Body>(
        api : Arc<WebApi>,
        req: Request<B>
    )->RouterResponse{
        match(req.method(), req.uri().path()){
            (&Method::POST, "/process") => WebApi::create_new_and_run(api, req).await,
            _ => WebApi::root(req).await,
        }
    }
    pub async fn start<>(self) ->Result<(), RexecError>{
        let ip = IpAddr::from_str(self.config.ip.as_str())
            .map_err(|e| {
                error!("FailedToCreateSocketAddress from {} reason {}",
                       self.config.ip, e.to_string());
                RexecError::code(RexecErrorType::FailedToCreateSocketAddress)
            })?;
        let address = SocketAddr::new(ip, self.config.port);

        let the_arc = Arc::new(self);
        
        // let service   =  service_fn(| req: Request<BoxBody>|
        //     async move {
        //         WebApi::router(api, req)
        //     });
        info!("Starting service on {}", address.to_string());
        let listener = TcpListener::bind(&address)
            .await
            .map_err(|e| {
                log::error!("FailedToStartWebServer {}", &e.to_string());
                RexecError::code_msg(
                    RexecErrorType::FailedToStartWebServer,
                    e.to_string())
            })?;

        loop{
            let (stream, _) = listener
                .accept()
                .await            
                .map_err(|e| {
                    log::error!("FailedToStartWebServer {}", &e.to_string());
                    RexecError::code_msg(
                        RexecErrorType::FailedToStartWebServer,
                        e.to_string())
                })?;
            let io = TokioIo::new(stream);
            let api = the_arc.clone();
            tokio::task::spawn(async move {
                let service = 
                    service_fn(| req: Request<IncomingBody>| {
                        let api2 = api.clone();
                        async move {
                            WebApi::router(api2, req).await
                        }
                    });
                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    println!("Failed to serve connection: {:?}", err);
                    
                }
            });
        }   
        // Server::bind(&address)
        //     .serve(service)
        //     .await
        //     .map_err(|e| {
        //         log::error!("FailedToStartWebServer {}", &e.to_string());
        //         RexecError::code_msg(
        //             RexecErrorType::FailedToStartWebServer,
        //             e.to_string())
        //     })?;
    }
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
    #[test]
    fn test_router_non_api(){
        let config = Config::for_addr("localhost".to_string(), 5566);
        let (create_tx, _create_rx) = mpsc::channel::<ProcessCreateMessage>(10);
        let (shutdown_tx, _shutdown_rx) = oneshot::channel::<Shutdown>();
        let api = WebApi{create_tx, shutdown_tx, config: config.clone()};
        let api_ref = Arc::new(api);

        let job = async{
            let req: Request<Full<Bytes>> = Request::get("http://localhost:5566/")
                .body(Full::from("".as_bytes()))
                .unwrap();
            let res = WebApi::router(api_ref.clone(),req).await.unwrap();
            matches!(res.status(), StatusCode::NOT_IMPLEMENTED);
            let req: Request<Full<Bytes>> = Request::builder()
                .uri("http://localhost:5566/process/1234")
                .body(Full::from("".as_bytes()))
                .unwrap();
            let res = WebApi::router(api_ref.clone(),req).await.unwrap();
            matches!(res.status(), StatusCode::NOT_IMPLEMENTED);

            let req: Request<Full<Bytes>> = Request::builder()
                .uri("http://localhost:5566/process/1234")
                .method("POST")
                .body(Full::from("".as_bytes()))
                .unwrap();
            let res = WebApi::router(api_ref.clone(),req).await.unwrap();
            matches!(res.status(), StatusCode::NOT_IMPLEMENTED);

            let req: Request<Full<Bytes>> = Request::builder()
                .uri("http://localhost:5566/process")
                .method("POST")
                .body(Full::from("".as_bytes()))
                .unwrap();
            let res = WebApi::router(api_ref.clone(),req).await.unwrap();
            matches!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

            Ok::<_,RexecError>(())
        };
        tokio::runtime::Runtime:: new()
            .expect("Failed to create Tokio runtime")
            .block_on(job).ok();
    }
    #[test]
    fn test_router_api(){
        let config = Config::for_addr("localhost".to_string(), 5566);
        let (create_tx, mut create_rx) = mpsc::channel::<ProcessCreateMessage>(10);
        let (shutdown_tx, _shutdown_rx) = oneshot::channel::<Shutdown>();
        let api = WebApi{create_tx, shutdown_tx, config: config.clone()};
        let api_ref = Arc::new(api);

        let dummy_broker = async{
            if let Some(mut msg) = create_rx.next().await{
                msg.start_tx.take()
                    .unwrap()
                    .send(StartConfirmation::Started)
                    .unwrap_or(());
                msg.stdout_tx.disconnect();
            }else{ () }

        };
        let job = async{
            let req = Request::builder()
                .uri("http://localhost:5566/process")
                .method("POST")
                .body(full(r#"{"cmd":"ls","alias":"ls"}"#))
                .unwrap();
            let router = WebApi::router(api_ref.clone(),req);
            let (res, _dummy) = futures::join!(router,dummy_broker);

            matches!(res.unwrap().status(), StatusCode::OK);
        };
        tokio::runtime::Runtime:: new()
            .expect("Failed to create Tokio runtime")
            .block_on(job);
    }
}