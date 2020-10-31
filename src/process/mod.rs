pub(crate) mod description;

use futures::{SinkExt};
use tokio::io::{AsyncBufReadExt, BufReader, Lines, AsyncBufRead};
use tokio::process::{Command};
use crate::process::description::ProcessDescription;
use std::process::Stdio;
use futures::channel::mpsc::Sender;
use crate::error::{RexecError, RexecErrorType};
use std::collections::HashMap;

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
        let child_res = Command::new(&create.desc.cmd)
            .stdout(Stdio::piped())
            .args(&create.desc.args)
            .current_dir(&create.desc.cwd)
            .envs(&create.desc.envs)
            .spawn();

        match child_res{
            Ok(_) => (),
            Err(e) =>{
                create.stdout_tx.clone().send(e.to_string()).await;
                return Err(RexecError::code_msg(
                    RexecErrorType::FailedToExecuteProcess,
                    e.to_string()))
            },
        }
        let mut child = child_res.unwrap();
        let stdout = child.stdout
            .take()
            .ok_or(RexecError::code_msg(
                RexecErrorType::FailedToExecuteProcess,
                "stdout not available".to_string()
            ))?;

        let alias = create.desc.alias.clone();
        let reader_out = BufReader::new(stdout).lines();

        Process::process_stdout(create, status_tx, reader_out).await.ok();

        child.kill().map_err(|e| RexecError::code_msg(
            RexecErrorType::FailedToKillProcess,
            e.to_string()))?;
        println!("Process: Finished {}", alias);
        Ok(())
    }
    async fn process_stdout<T: AsyncBufRead + Unpin>(
        create: ProcessCreateMessage,
        mut status_tx: StatusTx,
        mut reader_out: Lines<T>
    ) ->  Result<(),RexecError>{
        let mut stdout_tx = create.stdout_tx;
        let alias = create.desc.alias;
        let mut exit_result = Ok(());
        while let Ok(Some(line)) = reader_out.next_line().await {
            let res = stdout_tx.send(line).await;
            match res{
                Ok(_) => continue,
                Err(_) => {
                    exit_result = Err(RexecError::code_msg(
                        RexecErrorType::UnexpectedEof,
                        "Premature close of receiving channel".to_string()
                    ));
                    break
                },
            }
        }
        stdout_tx.disconnect();
        status_tx.send(ProcessStatusMessage{
            alias: alias.clone(),
            status: ProcessStatus::EXITED
        }).await.map_err(|e| RexecError::code_msg(
            RexecErrorType::UnexpectedEof,
            e.to_string()
        ))?;
        exit_result
    }
}

#[cfg(test)]
mod process_tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use futures::channel::mpsc;
    use std::collections::HashMap;
    use std::io::Cursor;
    use futures::StreamExt;
    use futures::channel::mpsc::Receiver;

    #[test]
    fn test_process_stdout_ok() {
        let job = async{
            let (mut stdout_rx, status_tx, mut status_rx, create, reader_out) = setup_test();
            let alias = create.desc.alias.clone();
            let process = Process::process_stdout(create,status_tx,reader_out);
            let reader = async move{
                while let Some(line) = stdout_rx.next().await{
                    println!("{}",line);
                }
                Ok::<_,RexecError>(())
            };
            let status = async move{
                let status = status_rx.next().await.unwrap();
                Ok::<_,RexecError>(status)
            };
            let (p, r, s) = futures::join!(process, reader,status);
            assert!(p.is_ok());
            assert!(r.is_ok());
            let status_msg = s.unwrap();
            matches!(status_msg.status, ProcessStatus::EXITED);
            assert_eq!(status_msg.alias, alias);
        };
        tokio::runtime::Runtime:: new()
            .expect("Failed to create Tokio runtime")
            .block_on(job);

    }
    #[test]
    fn test_premature_receiver_close() {
        let job = async{
            let (mut stdout_rx, status_tx, mut status_rx, create, reader_out) = setup_test();
            let alias = create.desc.alias.clone();
            let process = Process::process_stdout(create,status_tx,reader_out);
            let reader = async move{
                let mut line = stdout_rx.next().await.unwrap();
                println!("{}",line);
                line = stdout_rx.next().await.unwrap();
                println!("{}",line);

                Ok::<_,RexecError>(())
            };
            let status = async move{
                let status = status_rx.next().await.unwrap();
                Ok::<_,RexecError>(status)
            };
            let (p, r, s) = futures::join!(process, reader,status);
            assert!(!p.is_ok());
            matches!(p.err().unwrap().code, RexecErrorType::UnexpectedEof);
            assert!(r.is_ok());
            let status_msg = s.unwrap();
            matches!(status_msg.status, ProcessStatus::EXITED);
            assert_eq!(status_msg.alias, alias);
        };
        tokio::runtime::Runtime:: new()
            .expect("Failed to create Tokio runtime")
            .block_on(job);

    }

    fn setup_test<'a>() -> (Receiver<String>,
                            Sender<ProcessStatusMessage>,
                            Receiver<ProcessStatusMessage>,
                            ProcessCreateMessage,
                            Lines<BufReader<Cursor<&'a str>>>) {
        let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(1);
        let (status_tx, status_rx) = mpsc::channel::<ProcessStatusMessage>(1);

        let desc = ProcessDescription::simple(
            "test".to_string(),
            "program".to_string(),
            Vec::new(),
            "work_dir".to_string(),
            HashMap::new()
        );
        let create = ProcessCreateMessage { desc, stdout_tx };
        let buffer = Cursor::new("1\n2\n3\n4\n5\n6\n");
        let mut reader_out = BufReader::new(buffer).lines();
        (stdout_rx, status_tx, status_rx, create, reader_out)
    }
}