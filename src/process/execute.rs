use std::{io::Write, process::{ ExitStatus, Stdio}};


use futures::{channel::oneshot};
use log::{debug, info};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader}, 
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command}
};

use crate::error::{RexecError, RexecErrorType};

use super::description::ProcessDescription;

pub enum ProcessStatusId {
    Run,
    Exit,
    AlreadyRunning,
}
#[derive(Clone)]
struct Message{
    alias: String
}
pub type StopMessage = Message;
pub type ExitMessage = Message;

pub type StopTx = oneshot::Sender<StopMessage>;
pub type StopRx = oneshot::Receiver<StopMessage>;

pub type ExitRx = oneshot::Receiver<ExitMessage>;
pub type ExitTx = oneshot::Sender<ExitMessage>;

pub struct Process{
    pub desc: ProcessDescription,
    pub filename: String,
    pub stop_tx: StopTx,
    pub exit_rx: ExitRx,
}

struct ChildProc{
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
    stop_rx: StopRx,
    exit_tx: ExitTx,
}
pub async fn start(create: &ProcessDescription) -> Result<Process, RexecError> {
    let child_res = Command::new(&create.cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .args(&create.args)
        .current_dir(&create.cwd)
        .envs(&create.envs)
        .spawn();

    let desc = create.clone();
    let alias = create.alias.clone();

    match child_res {
        Ok(mut child) => {
            let (stop_tx, stop_rx) = oneshot::channel::<StopMessage>();
            let (exit_tx, exit_rx) = oneshot::channel::<ExitMessage>();

            child.stdin.take()
                .and_then(|i|{
                    child.stdout.take().and_then(|o|{
                        child.stderr.take().map(|e| (i,o,e))
                    })
            })
            .map(|(stdin,stdout,stderr)|{  
                debug!("All stdin, stdout, or stderr are OK for {}",alias);

                tokio::task::spawn(async move {
                    run_child(ChildProc{child,stdin, stdout, stderr, stop_rx, exit_tx}).await
                });
                Process{
                    desc: desc,
                    filename: "".to_string(),
                    stop_tx: stop_tx,
                    exit_rx: exit_rx,
                }
            }).ok_or_else(||{
                debug!("Failed to start the process {} due to failing stdin, stdout, or stderr",alias);
                RexecError::code(RexecErrorType::FailedToExecuteProcess)
            })
        },
        Err(e)=>{
            info!("FailedToExecuteProcess {}", &e.to_string());
            Err(RexecError::code(RexecErrorType::FailedToExecuteProcess))
        }
    }
}
async fn signal_exit(tx : ExitTx, alias: String, err: ExitStatus){
    debug!("Process {} exited with error code {}",alias,err);
    tx.send(ExitMessage{alias: alias.clone()}).ok();
}
async fn write_log<T: AsyncRead+Unpin>(lines: &mut tokio::io::Lines<BufReader<T>>)-> Result<(),RexecError>{
    match lines.next_line().await{
        Err(e) => Err(RexecError::code_msg(RexecErrorType::UnexpectedEof, e.to_string())),
        Ok(Some(line)) => {
            println!("{line}");
            Ok(())
        },
        Ok(None) => Err(RexecError::code(RexecErrorType::UnexpectedEof)),
    }
}
async fn run_child(mut child_proc: ChildProc){
    let mut stdout = BufReader::new(child_proc.stdout).lines();
    let mut stderr = BufReader::new(child_proc.stderr).lines();
    loop{
        tokio::select! {
        Ok(()) = write_log(&mut stdout) => (),
        Ok(()) = write_log(&mut stderr) => (),
        else => break
        }
    }
    debug!("run_child loop finished. Waiting for the process to finish.");
    // child_proc.exit_tx.cancellation().await;
    // child_proc.stdin.drop();
    child_proc.child.wait().await.ok();
    debug!("run_child completed.");


}
// async fn monitor_process<T: AsyncBufRead + Unpin>(
//     create: ProcessCreateMessage,
//     status_tx: StatusTx,info
//     mut reader_out: Lines<T>
//     mut reader_out: Lines<T>
// ) ->  Result<(),RexecError>{
//     let mut stdout_tx = create.stdout_tx;
//     let alias = create.desc.alias;
//     let mut exit_result = Ok(());

//     loop{
//         tokio::select!{
//         line = reader_out.next_line() => match line{
//             Err(_) => {
//                 debug!("Failed to read next_line from child's stdout buffer.");
//                 break
//             },
//             Ok(Some(l)) => {
//                 let res = stdout_tx.send(l).await;
//                 match res{
//                     Ok(_) => continue,
//                     Err(_) => {
//                         debug!("Premature close of receiving channel.");
//                         exit_result = Err(RexecError::code_msg(
//                             RexecErrorType::UnexpectedEof,
//                             "Premature close of receiving channel".to_string()
//                         ));
//                         break
//                     },
//                 }
//             },
//             Ok(None) => {
//                 debug!("Child's stdout closed. The child process finished.");
//                 break
//             },
//         },
//         _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) =>{
//             if stdout_tx.is_closed() {
//                 debug!("From a timeout. Child's stdout closed. The child process finished.");
//                 break
//             }
//         },
//         }
//     }
//     stdout_tx.close_channel();
//     Process::send_status(status_tx, alias).await?;
//     exit_result
// }

// async fn send_status(mut status_tx: StatusTx, alias: String) ->  Result<(),RexecError>{
//     status_tx.send(ProcessStatusMessage { alias, status: ProcessStatus::EXITED })
//         .await
//         .map_err(|e|{
//             debug!("FailedToSendStatus ProcessStatus::EXITED {}", &e.to_string());
//             RexecError::code_msg(RexecErrorType::FailedToSendStatus,
//                                  e.to_string())
//         })
// }
