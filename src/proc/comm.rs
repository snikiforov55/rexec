use futures::{channel::oneshot};
use tokio::sync::broadcast;

use super::description::ProcessDescription;

#[derive(Clone)]
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
    Run,
    Exit,
    Failed,
}

pub struct Process{
    pub desc: ProcessDescription,
    pub filename: String,
    pub stop_tx: Option<StopTx>,
    pub exit_rx: ExitRx,
    pub bcst_rx: broadcast::Receiver<String>,
    pub status: ProcessStatusId,
}

impl std::fmt::Display for Process{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Process\ndesc:{:#?}\nfilename:{}",self.desc, self.filename));
        Ok(())
    }
}