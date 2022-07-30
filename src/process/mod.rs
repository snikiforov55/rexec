/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

pub(crate) mod description;

use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt, AsyncBufRead, Lines};
use std::process::Stdio;
use futures::channel::mpsc::Sender;
use futures::channel::oneshot;
use futures::SinkExt;
use log::{error,debug,info,warn};

use crate::error::{RexecError, RexecErrorType};
use crate::process::description::ProcessDescription;

pub enum ProcessStatus {
    RUN,
    EXITED,
}
#[derive(Clone)]
pub enum StartConfirmation{
    Started,
    Error(String),
    AlreadyRunning,
}
pub struct ProcessStatusMessage{
    pub(crate) alias: String,
    pub(crate) status: ProcessStatus,
}
pub type StreamTx = Sender<String>;
pub type StatusTx = Sender<ProcessStatusMessage>;
pub type StartTx = oneshot::Sender<StartConfirmation>;

pub struct ProcessCreateMessage {
    pub desc: ProcessDescription,
    pub stdout_tx: StreamTx,
    pub start_tx: Option<StartTx>
}

pub struct Process{
}

impl Process{
    pub async fn run(mut create: ProcessCreateMessage,
                     status_tx: StatusTx
    ) ->  Result<(),RexecError>
    {
        let child_res = Command::new(&create.desc.cmd)
            .stdout(Stdio::piped())
            .args(&create.desc.args)
            .current_dir(&create.desc.cwd)
            .envs(&create.desc.envs)
            .spawn();

        let start_tx = match create.start_tx.take(){
            Some(tx) => tx,
            None => {
                error!("Empty start tx supplied \
                    when running {}", &create.desc.alias);
                return Err(RexecError::code(RexecErrorType::InvalidCreateProcessRequest));
            }
        };
        let alias = create.desc.alias.clone();

        match child_res.as_ref(){
            Ok(_) => start_tx.send(StartConfirmation::Started)
                .map_err(|_| {
                    debug!("FailedToSendStatus StartConfirmation::Started");
                    RexecError::code(RexecErrorType::FailedToSendStatus)
                })?,
            Err(e) =>{
                info!("FailedToExecuteProcess {}",&e.to_string());

                start_tx.send(StartConfirmation::Error(e.to_string()))
                    .map_err(|_| RexecError::code(RexecErrorType::FailedToSendStatus))?;
                Process::send_status(status_tx.clone(), alias).await?;

                return Err(RexecError::code(RexecErrorType::FailedToExecuteProcess))
            },
        }

        let mut child = child_res.unwrap();
        let stdout = child.stdout
            .take()
            .ok_or_else(||{
                warn!("FailedToExecuteProcess. Stdout not available.");
                RexecError::code_msg(
                RexecErrorType::FailedToExecuteProcess,
                "stdout not available".to_string())
            })?;

        let reader_out = BufReader::new(stdout).lines();
        Process::process_stdout(create, status_tx, reader_out).await.ok();

        child.kill().await.map_err(|e| {
            warn!("Failed to kill process {}", &e.to_string());
            RexecError::code_msg(
                RexecErrorType::FailedToKillProcess,
                e.to_string())
        })?;
        info!("Process: Finished {}", alias);
        Ok(())
    }
    async fn process_stdout<T: AsyncBufRead + Unpin>(
        create: ProcessCreateMessage,
        status_tx: StatusTx,
        mut reader_out: Lines<T>
    ) ->  Result<(),RexecError>{
        let mut stdout_tx = create.stdout_tx;
        let alias = create.desc.alias;
        let mut exit_result = Ok(());

        loop{
            tokio::select!{
            line = reader_out.next_line() => match line{
                Err(_) => {
                    debug!("Failed to read next_line from child's stdout buffer.");
                    break
                },
                Ok(Some(l)) => {
                    let res = stdout_tx.send(l).await;
                    match res{
                        Ok(_) => continue,
                        Err(_) => {
                            debug!("Premature close of receiving channel.");
                            exit_result = Err(RexecError::code_msg(
                                RexecErrorType::UnexpectedEof,
                                "Premature close of receiving channel".to_string()
                            ));
                            break
                        },
                    }
                },
                Ok(None) => {
                    debug!("Child's stdout closed. The child process finished.");
                    break
                },
            },
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) =>{
                if stdout_tx.is_closed() {
                    debug!("From a timeout. Child's stdout closed. The child process finished.");
                    break
                }
            },
            }
        }
        stdout_tx.close_channel();
        Process::send_status(status_tx, alias).await?;
        exit_result
    }

    async fn send_status(mut status_tx: StatusTx, alias: String) ->  Result<(),RexecError>{
        status_tx.send(ProcessStatusMessage { alias, status: ProcessStatus::EXITED })
            .await
            .map_err(|e|{
                debug!("FailedToSendStatus ProcessStatus::EXITED {}", &e.to_string());
                RexecError::code_msg(RexecErrorType::FailedToSendStatus,
                                     e.to_string())
            })
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
            let (mut stdout_rx, status_tx, mut status_rx, _start_rx, create, reader_out) = setup_test();
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
            let (mut stdout_rx, status_tx, mut status_rx, _start_rx, create, reader_out) = setup_test();
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
    //struct SlowLines;
    // #[test]
    // fn test_premature_receiver_close_for_quiet_stdout() {
    //     let job = async{
    //         let (mut stdout_rx, status_tx, mut status_rx, _start_rx, create, reader_out) = setup_test();
    //         let reader = Lines::try_from(SlowLines{});
    //         let alias = create.desc.alias.clone();
    //         let process = Process::process_stdout(create,status_tx,reader_out);
    //         let reader = async move{
    //             let mut line = stdout_rx.next().await.unwrap();
    //             println!("{}",line);
    //             line = stdout_rx.next().await.unwrap();
    //             println!("{}",line);
    //
    //             Ok::<_,RexecError>(())
    //         };
    //         let status = async move{
    //             let status = status_rx.next().await.unwrap();
    //             Ok::<_,RexecError>(status)
    //         };
    //         let (p, r, s) = futures::join!(process, reader,status);
    //         assert!(!p.is_ok());
    //         matches!(p.err().unwrap().code, RexecErrorType::UnexpectedEof);
    //         assert!(r.is_ok());
    //         let status_msg = s.unwrap();
    //         matches!(status_msg.status, ProcessStatus::EXITED);
    //         assert_eq!(status_msg.alias, alias);
    //     };
    //     tokio::runtime::Runtime:: new()
    //         .expect("Failed to create Tokio runtime")
    //         .block_on(job);
    //
    // }

    fn setup_test<'a>() -> (Receiver<String>,
                            Sender<ProcessStatusMessage>,
                            Receiver<ProcessStatusMessage>,
                            oneshot::Receiver<StartConfirmation>,
                            ProcessCreateMessage,
                            Lines<BufReader<Cursor<&'a str>>>) {
        let (stdout_tx, stdout_rx) = mpsc::channel::<String>(1);
        let (status_tx, status_rx) = mpsc::channel::<ProcessStatusMessage>(1);
        let (start_tx, start_rx) = oneshot::channel::<StartConfirmation>();

        let desc = ProcessDescription::simple(
            "test".to_string(),
            "program".to_string(),
            Vec::new(),
            "work_dir".to_string(),
            HashMap::new()
        );
        let create = ProcessCreateMessage { desc, stdout_tx, start_tx: Some(start_tx) };
        let buffer = Cursor::new("1\n2\n3\n4\n5\n6\n");
        let reader_out = BufReader::new(buffer).lines();
        (stdout_rx, status_tx, status_rx, start_rx, create, reader_out)
    }
}