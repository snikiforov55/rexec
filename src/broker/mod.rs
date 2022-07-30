/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

use std::collections::HashMap;
use std::option::Option::Some;
use futures::{StreamExt};
use futures::future::{FutureExt};
use futures::channel::oneshot;
use futures::channel::mpsc;
use futures::channel::mpsc::Receiver;
use futures_util::future::BoxFuture;
use log::{info,debug};

use crate::error::{RexecError};
use crate::process::{Process, ProcessStatus, ProcessCreateMessage, ProcessStatusMessage, StartConfirmation, StatusTx};
use crate::process::description::ProcessDescription;
use crate::config::Config;

pub enum Shutdown{
    Shutdown,
}
pub enum BrokerExitStatus{
    Shutdown,
    CreateChannelFailure,
    StatusChannelFailure,
}
pub struct ProcessEndpoint{
    pub desc: ProcessDescription,
    pub status: ProcessStatus,
}


type Processes = HashMap<String, ProcessEndpoint>;
pub type CreateRx = Receiver<ProcessCreateMessage>;
pub type ShutdownRx = oneshot::Receiver<Shutdown>;


pub struct Broker{
    create_rx: CreateRx,
    shutdown_rx: ShutdownRx,
    config: Config,
    run_future: fn(ProcessCreateMessage,StatusTx)->BoxFuture<'static, Result<(),RexecError>>
}
struct BrokerState{
    children: Processes,
}

impl BrokerState{
    fn create_child_process(&mut self, desc: ProcessDescription) {
        debug!("Created process record {}. Children size {}",
               &desc.alias, self.children.len());

        self.children.insert(
            desc.alias.clone(),
            ProcessEndpoint { desc, status: ProcessStatus::RUN }
        );
    }
    fn set_status(&mut self, res : ProcessStatusMessage){
        self.children
            .get_mut(&res.alias)
            .map(|s| {
                s.status = res.status;
                info!("Finished process {}", res.alias);
                }
            );
    }
    fn is_running(&self, alias : &String) -> bool{
        self.children
            .get(alias)
            .map_or(false, |p| match p.status {
                ProcessStatus::RUN => true,
                _ => false,
            })
    }
}

impl Broker {
    pub fn new(
        create_rx: CreateRx,
        shutdown_rx: ShutdownRx,
        config: Config
    ) -> Self {
        Broker {
            create_rx,
            shutdown_rx,
            config,
            run_future: |a,b|{Process::run(a,b).boxed()}
        }
    }
    pub fn for_test(
        create_rx: CreateRx,
        shutdown_rx: ShutdownRx,
        config: Config,
        run_future: fn(ProcessCreateMessage,StatusTx)->BoxFuture<'static, Result<(),RexecError>>
    ) -> Self {
        Broker { create_rx, shutdown_rx, config, run_future }
    }

    pub async fn start(self) -> Result<BrokerExitStatus, RexecError> {
        let Broker{mut create_rx,shutdown_rx,config,run_future} = self;
        let mut broker_state = BrokerState{
            children: HashMap::new(),
        };
        let mut shutdown = shutdown_rx.fuse();
        let (status_tx, mut status_rx) =
            mpsc::channel::<ProcessStatusMessage>(config.status_size);
        loop{
            futures::select! {
            // Application shutdown requested
            _ = shutdown => {
                debug!("Exiting broker loop because of Shutdown command");
                break
            },
            // Process create command
            msg = create_rx.next() => match msg {
                Some(create) => {
                    if broker_state.is_running(&create.desc.alias){
                        debug!("process already running {}", &create.desc.alias);
                        Broker::send_reply_already_running(create);
                    }else{
                        info!(target:"broker", "Started process {}", &create.desc.alias);
                        broker_state.create_child_process(create.desc.clone());
                        tokio::task::spawn((run_future)(create, status_tx.clone()));
                    }
                },
                None => {
                    debug!("Exiting broker loop because of CreateChannelFailure");
                    return Ok(BrokerExitStatus::CreateChannelFailure)
                },
            },
            status = status_rx.next() => match status{
                Some(s) => broker_state.set_status(s),
                None => {
                    debug!("Exiting broker loop because of StatusChannelFailure");
                    return Ok(BrokerExitStatus::StatusChannelFailure)
                },
            },
            complete => {
                   debug!("Exiting broker loop because all futures have finished");
                   break
                },
            }
        }
        Ok(BrokerExitStatus::Shutdown)
    }
    fn send_reply_already_running(mut create : ProcessCreateMessage){
        create.start_tx.take().map(|tx| tx.send(StartConfirmation::AlreadyRunning));
    }
}


#[cfg(test)]
mod broker_tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use crate::error::RexecErrorType;
    use futures::SinkExt;

    #[test]
    fn test_shutdown() {
        let config = Config::for_addr("127.0.0.1".to_string(), 8910);
        let (_create_tx, create_rx) = mpsc::channel::<ProcessCreateMessage>(10);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<Shutdown>();
        let broker = Broker::new(create_rx, shutdown_rx, config.clone());

        let job = async move{
            let test_cycle = async move{
                shutdown_tx.send(Shutdown::Shutdown)
                    .map_err(|_| RexecError::code(RexecErrorType::UnexpectedEof))?;
                Ok::<_,RexecError>(())
            };
            let (broker_res, _) = futures::join!(broker.start(), test_cycle);
            matches!(broker_res.ok().unwrap(), BrokerExitStatus::Shutdown)
        };
        tokio::runtime::Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(job);
    }
    #[test]
    fn test_create_channel_error() {
        let config = Config::for_addr("127.0.0.1".to_string(), 8910);
        let (mut create_tx, create_rx) = mpsc::channel::<ProcessCreateMessage>(10);
        let (_shutdown_tx, shutdown_rx) = oneshot::channel::<Shutdown>();
        let broker = Broker::new(create_rx, shutdown_rx, config.clone());

        let job = async move{
            let test_cycle = async move{
                create_tx.close().await
                    .map_err(|_| RexecError::code(RexecErrorType::UnexpectedEof))?;
                Ok::<_,RexecError>(())
            };
            let (broker_res, _) = futures::join!(broker.start(), test_cycle);
            matches!(broker_res.ok().unwrap(), BrokerExitStatus::CreateChannelFailure)
        };
        tokio::runtime::Runtime:: new()
            .expect("Failed to create Tokio runtime")
            .block_on(job);
    }
    #[test]
    fn test_already_running() {
        let config = Config::for_addr("127.0.0.1".to_string(), 8910);
        let ( create_tx, create_rx) = mpsc::channel::<ProcessCreateMessage>(10);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<Shutdown>();

        let broker = Broker::for_test(
            create_rx,
            shutdown_rx,
            config.clone(),
            |mut create, _status_tx|{
                async move{
                    create.start_tx.unwrap()
                        .send(StartConfirmation::Started).ok();
                    create.stdout_tx.send("line".to_string()).await.ok();
                    Ok::<_,RexecError>(())
                }.boxed()
            }
        );
        let mut tx = create_tx.clone();
        let test_cycle = ||{async move {
            let (start_tx, start_rx) = oneshot::channel::<StartConfirmation>();
            let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(1);

            tx.send(ProcessCreateMessage {
                desc : ProcessDescription::simple(
                    "ls".to_string(),
                    "ls".to_string(),
                    Vec::new(),
                    ".".to_string(),
                    HashMap::new()
                ),
                stdout_tx,
                start_tx: Some(start_tx)
            }).await.map_err(|e| RexecError::code_msg(
                RexecErrorType::FailedToSendStartCommand,
                e.to_string())
            )?;
            match start_rx.await.unwrap(){
                StartConfirmation::Started => {
                    stdout_rx.next().await;
                    println!("Started normally");
                    Ok::<_, RexecError>(StartConfirmation::Started)
                },
                StartConfirmation::AlreadyRunning => {
                    println!("Already running");
                    Err(RexecError::code_msg(RexecErrorType::FailedToExecuteProcess,
                                             "Already running".to_string()))
                },
                StartConfirmation::Error(s) => {
                    println!("Error {}", &s);
                    Err(RexecError::code_msg(RexecErrorType::FailedToExecuteProcess,s))
                },
            }
            }
        };
        let job = async move{
            let first_test = (test_cycle.clone())();
            let second_test = async {
                let res = (test_cycle.clone())().await;
                shutdown_tx.send(Shutdown::Shutdown).ok();
                res
            };
            let (_b, f, s) = futures::join!(broker.start(),first_test, second_test);

            assert!(f.is_ok());
            assert!(!s.is_ok());
        };
        tokio::runtime::Runtime:: new()
            .expect("Failed to create Tokio runtime")
            .block_on(job);
    }
}