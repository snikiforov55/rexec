
use std::collections::HashMap;
use std::option::Option::Some;

use futures::{StreamExt};
use futures::future::{FutureExt};
use futures::channel::oneshot;
use futures::channel::mpsc;

use crate::error::{RexecError};
use crate::process::{Process, ProcessStatus, ProcessCreateMessage, ProcessStatusMessage};
use futures::channel::mpsc::Receiver;
use crate::process::description::ProcessDescription;
use crate::config::Config;

pub enum Shutdown{
    Shutdown,
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
}

impl Broker {
    pub fn new(
        create_rx: CreateRx,
        shutdown_rx: ShutdownRx,
        config: Config
    ) -> Self {
        Broker { create_rx, shutdown_rx, config }
    }

    pub async fn start(self) -> Result<(), RexecError> {
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
                    println!("Started process {}", &create.desc.alias);
                    broker_state.create_child_process(create.desc.clone());
                    tokio::task::spawn(Process::run(create, status_tx.clone()));
                    ()
                },
                None => break,
            },
            status = status_rx.next() => match status{
                Some(s) => broker_state.set_status(s),
                None => break,
            },
            complete => break,
            }
        }
        Ok(())
    }
}


#[cfg(test)]
mod broker_tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_create() {

    }
}