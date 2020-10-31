
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::net::SocketAddr;
use std::str::FromStr;
use crate::error::{RexecError, RexecErrorType};
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::{SinkExt, FutureExt, StreamExt};
use futures::future::BoxFuture;
use std::sync::Arc;
use crate::broker::Shutdown;
use crate::process::ProcessCreateMessage;
use crate::process::description::ProcessDescription;
use std::collections::HashMap;
use hyper::body::Bytes;

type CreateTx=mpsc::Sender<ProcessCreateMessage>;
type ShutdownTx = oneshot::Sender<Shutdown>;

pub struct WebApi{
    pub(crate) create_tx: CreateTx,
    pub(crate) shutdown_tx: ShutdownTx,
}
type RouterResponse = Result<Response<Body>,hyper::Error>;

impl WebApi{
    async fn create_new_and_run(api:  Arc<WebApi>, req: Request<Body>) ->RouterResponse{
        let (stdout_tx, stdout_rx) = mpsc::channel::<String>(8);

        let bytes = hyper::body::to_bytes(req.into_body()).await?;
        let res = async move{
            let desc = WebApi::parse_body(bytes)?;
            println!("Sending start command for {}", &desc.alias);
            let mut create_tx = api.create_tx.clone();
            create_tx.send(ProcessCreateMessage{desc,stdout_tx,})
                .await
                .map_err(|e| RexecError::code_msg(
                    RexecErrorType::UnexpectedEof,
                    e.to_string())
            )
        }.await;

        match res{
            Ok(_) => Ok(
                Response::new(Body::wrap_stream(
                    stdout_rx.map(|s| {
                        println!("HTTP: {}",s);
                        Ok::<_, hyper::Error>(format!("{}\n",s))
                    })))
            ),
            Err(e) => Ok(
                hyper::Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(e.to_string()))
                    .unwrap()
            ),
        }
    }

    fn parse_body(bytes: Bytes) -> Result<ProcessDescription, RexecError> {
        let body = String::from_utf8(bytes.to_vec())
            .map_err(|e| RexecError::code_msg(
                RexecErrorType::InvalidCreateProcessRequest,
                e.to_string()
            ))?;
        let desc : ProcessDescription = serde_json::from_str(&body)
            .map_err(|e| RexecError::code(RexecErrorType::InvalidCreateProcessRequest))?;
        Ok(desc)
    }
    async fn root(_req: Request<Body>)->RouterResponse{
        Ok(Response::new(Body::from("root".to_string())))
    }
    fn router<'a>(api : Arc<WebApi>, req: Request<Body>)->BoxFuture<'a,Result<Response<Body>,hyper::Error>>{
        match(req.method(), req.uri().path()){
            (&Method::POST, "/process") => WebApi::create_new_and_run(api, req).boxed(),
            _ => WebApi::root(req).boxed(),
        }
    }
    pub async fn start<>(self) ->Result<(), RexecError>{
        let the_arc = Arc::new(self);
        let service   = make_service_fn(move |_| {
            let api = the_arc.clone();
            async move {
                Ok::<_, hyper::Error>(
                    service_fn(move | req: Request<Body>| {
                        WebApi::router(api.clone(), req)
                    }))
            }
        });
        let address = SocketAddr::from_str("127.0.0.1:8910")
            .map_err(|_| RexecError::code(RexecErrorType::FailedToCreateSocketAddress))?;
        println!("Socket address {}", address.to_string());
        Server::bind(&address)
            .serve(service)
            .await
            .map_err(|e| RexecError::code_msg(
                RexecErrorType::FailedToStartWebServer,
                e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod web_api_tests{
    use super::*;

    #[test]
    fn test_parse_body_full(){
        let body = r#"{
            "alias" : "test",
            "cmd": "shell",
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
        let desc = WebApi::parse_body(hyper::body::Bytes::from(body)).unwrap();
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
        let desc = WebApi::parse_body(hyper::body::Bytes::from(body)).unwrap();
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
        let desc = WebApi::parse_body(hyper::body::Bytes::from(body));
        assert!(!desc.is_ok());
        matches!(desc.err().unwrap().code, RexecErrorType::InvalidCreateProcessRequest);
    }
}