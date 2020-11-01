/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

use std::collections::HashMap;
use std::option::Option::Some;

use futures::{StreamExt};
use futures::future::{FutureExt};
use futures::channel::oneshot;
use futures::channel::mpsc;

use crate::error::{RexecError};
use crate::process::{Process, ProcessStatus, ProcessCreateMessage, ProcessStatusMessage, StartConfirmation, StatusTx};
use futures::channel::mpsc::Receiver;
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
pub type ProcessResult = Result<ProcessStatus, RexecError>;
pub type CreateRx = Receiver<ProcessCreateMessage>;
pub type ShutdownRx = oneshot::Receiver<Shutdown>;


pub struct Broker{
    create_rx: CreateRx,
    shutdown_rx: ShutdownRx,
    config: Config,
}
struct BrokerState{
    children: Processes,
}

impl BrokerState{
    fn create_child_process(&mut self, desc: ProcessDescription) {
        println!("Starting {}", &desc.alias);
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
                println!("finished process {}", res.alias);
                }
            );
    }
    fn is_running(&self, alias : &String) -> bool{
        self.children
            .get(alias)
            .map(|p| match p.status{
                ProcessStatus::RUN => return true,
                _ => return false,
            });
        return false
    }
}

impl Broker {
    pub fn new(
        create_rx: CreateRx,
        shutdown_rx: ShutdownRx,
        config: Config
    ) -> Self {
        Broker { create_rx, shutdown_rx, config }
    }

    pub async fn start(self) -> Result<BrokerExitStatus, RexecError> {
        let Broker{
            mut create_rx,
            shutdown_rx,
            config
        } = self;
        let mut broker_state = BrokerState{
            children: HashMap::new(),
        };
        let mut shutdown = shutdown_rx.fuse();
        let (status_tx, mut status_rx) = mpsc::channel::<ProcessStatusMessage>(config.status_size);
        loop{
            futures::select! {
            // Application shutdown requested
            _ = shutdown => break,
            // Process create command
            msg = create_rx.next() => match msg {
                Some(create) => {
                    if broker_state.is_running(&create.desc.alias){
                        Broker::send_reply_already_running(create);
                    }else{
                        Broker::create_child(&mut broker_state, create, status_tx.clone());
                    }
                },
                None => return Ok(BrokerExitStatus::CreateChannelFailure),
            },
            status = status_rx.next() => match status{
                Some(s) => broker_state.set_status(s),
                None => return Ok(BrokerExitStatus::StatusChannelFailure),
            },
            complete => break,
            }
        }
        Ok(BrokerExitStatus::Shutdown)
    }
    fn send_reply_already_running(mut create : ProcessCreateMessage){
        create.start_tx.take().unwrap().send(StartConfirmation::AlreadyRunning).ok();
    }
    fn create_child(broker_state: &mut BrokerState,
                    create : ProcessCreateMessage,
                    status_tx: StatusTx
    ){
        println!("Started process {}", &create.desc.alias);
        broker_state.create_child_process(create.desc.clone());
        tokio::task::spawn(Process::run(create, status_tx));
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
        tokio::runtime::Runtime:: new()
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
}