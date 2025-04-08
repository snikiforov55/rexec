use std::ffi::OsString;

use tokio::sync::{broadcast, oneshot, mpsc};

use super::description::ProcessDescription;

#[derive(Clone, Debug)]
pub struct Message{
    //pub alias: String
}
pub type StopMessage = Message;
pub type ExitMessage = Message;

pub type StopTx = oneshot::Sender<StopMessage>;
pub type StopRx = oneshot::Receiver<StopMessage>;

pub type ExitRx = oneshot::Receiver<ExitMessage>;
pub type ExitTx = oneshot::Sender<ExitMessage>;

pub enum ProcessStatusId {
    New,
    Run,
    Failed,
}


pub struct Process{
    pub desc: ProcessDescription,
    pub status: ProcessStatusId,
    pub filename: OsString,
    pub stop_tx: Option<StopTx>,
    pub exit_rx: Option<ExitRx>,
    pub bcst_rx: broadcast::Receiver<String>,
    pub stdin_tx: mpsc::Sender<String>,
}

impl std::fmt::Display for Process{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Process\ndesc:{:#?}\nfilename:{:#?}",self.desc, self.filename))?;
        Ok(())
    }
}