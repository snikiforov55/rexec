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
struct ProcessIo{

}
impl Process{
    pub fn from(desc: ProcessDescription) -> (){
        /*
            let fileinfo = FileInfo::next_file(&desc.alias, &conf.path).await?;
    debug!("filename {:?}", fileinfo.filename);

    let (stop_tx, stop_rx) = oneshot::channel::<StopMessage>();
    let (exit_tx, exit_rx) = oneshot::channel::<ExitMessage>();
    let (stdin_tx, stdin_rx) = mpsc::channel::<String>(conf.io.stdin_capasity);
    let (bcst_tx, bcst_rx) = broadcast::channel::<String>(conf.io.bcast_capasity);

    let proc = Process {
        desc: desc.clone(),
        status: ProcessStatusId::New,
        filename: fileinfo.filename.clone(),
        stop_tx: Some(stop_tx),
        exit_rx: Some(exit_rx),
        bcst_rx,
        stdin_tx,
    };
         */
    }
}
impl std::fmt::Display for Process{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Process\ndesc:{:#?}\nfilename:{:#?}",self.desc, self.filename))?;
        Ok(())
    }
}