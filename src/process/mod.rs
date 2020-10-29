pub(crate) mod description;

use futures::{SinkExt};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Command};
use crate::process::description::ProcessDescription;
use std::process::Stdio;
use futures::channel::mpsc::Sender;
use crate::error::RexecError;

pub enum ProcessStatus {
    RUN,
    EXITED,
}
pub struct ProcessStatusMessage{
    pub(crate) alias: String,
    pub(crate) status: ProcessStatus,
}
pub type StreamTx = Sender<String>;
pub type StatusTx = Sender<ProcessStatusMessage>;

pub struct ProcessCreateMessage {
    pub desc: ProcessDescription,
    pub stdout_tx: StreamTx,
}

pub struct Process{
}

impl Process{
    pub async fn run(create: ProcessCreateMessage, mut status_tx: StatusTx) ->  Result<(),RexecError>
    {
        Command::new(&create.desc.command)
            .args(&create.desc.arguments)
            .current_dir(&create.desc.work_dir)
            .stdout(Stdio::piped())
            .spawn()
            .map_or(None, |child| {
                println!("Process: Child created");
                child.stdout
                }
            )
            .map(|stdout|{
                let mut stdout = stdout;
                let mut stdout_tx = create.stdout_tx;
                let alias = create.desc.alias;
                async move{
                    let mut reader_out = BufReader::new(stdout).lines();
                    while let Ok(Some(line)) = reader_out.next_line().await {
                        let res = stdout_tx.send(line).await;
                        match res{
                            Ok(_) => continue,
                            Err(_) => break,
                        }
                    }
                    stdout_tx.disconnect();
                    status_tx.send(ProcessStatusMessage{alias: alias.clone(), status: ProcessStatus::EXITED }).await;
                    println!("Process: Finished {}", alias);
                    Ok(())
                }
            }
            ).unwrap().await
    }
}

#[cfg(test)]
mod process_tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_start() {
        let job = async{

        };
        tokio::runtime::Runtime:: new()
            .expect("Failed to create Tokio runtime")
            .block_on(job);

    }
}