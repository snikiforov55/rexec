use std::{process::{ ExitStatus, Stdio}};


use chrono::Utc;

use futures::channel::oneshot;
use log::{debug, info};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader}, 
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command}
};

use crate::{error::{RexecError, RexecErrorType}, register::RegisterRef};

use crate::proc::description::ProcessDescription;
use crate::proc::comm::{Process, StopRx, ExitTx,StopMessage,ExitMessage};


struct ChildProc{
    alias: String,
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
    stop_rx: StopRx,
    exit_tx: ExitTx,
    reg: RegisterRef,
}

async fn do_start(reg: &RegisterRef,desc: &ProcessDescription) -> Result<(), RexecError>{
    let child_res = Command::new(&desc.cmd)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .stdin(Stdio::piped())
    .args(&desc.args)
    .current_dir(&desc.cwd)
    .envs(&desc.envs)
    .spawn();

    let pd = desc.clone();
    let alias = desc.alias.clone();
    let reg_ref = reg.clone();

    let proc = match child_res {
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
                let date = Utc::now().format("%Y%M%d-%H%M%S");
                let filename = format!("{}-utc-{date}.log", pd.alias); 
                let a = alias.clone();
                debug!("filename {filename}");
                tokio::task::spawn(async move {
                    run_child(ChildProc{alias:a,child,stdin, stdout, stderr, stop_rx, exit_tx, reg: reg_ref}).await
                });
                Process{
                    desc: pd,
                    filename: filename.to_string(),
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
    };
    match proc {
        Ok(p ) => {
            reg.write().await.add(p); 
            Ok(())
        },
        Err(e) => Err(e)
    }
}

pub async fn start(reg: &RegisterRef,desc: &ProcessDescription) -> Result<(), RexecError> {
    // Some more advanced request might be required.
    if reg.read().await.get(&desc.alias).is_some(){return Err(RexecError::code(RexecErrorType::AlreadyRunning))}
    do_start(reg, desc).await
}
async fn signal_exit(tx : ExitTx, alias: String, err: ExitStatus){
    debug!("Process {} exited with error code {}",alias,err);
    tx.send(ExitMessage{alias: alias.clone()}).ok();
}
async fn write_log<T: AsyncRead+Unpin>(lines: &mut tokio::io::Lines<BufReader<T>>, label: &str)-> Result<(),RexecError>{
    match lines.next_line().await{
        Err(e) => Err(RexecError::code_msg(RexecErrorType::UnexpectedEof, e.to_string())),
        Ok(Some(line)) => {
            println!("[{label}] {line}");
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
        Ok(()) = write_log(&mut stdout,"OUT") => (),
        Ok(()) = write_log(&mut stderr,"ERR") => (),
        else => break
        }
    }
    debug!("run_child loop finished. Waiting for the process to finish.");
    // child_proc.exit_tx.cancellation().await;
    // child_proc.stdin.drop();
    child_proc.child.wait().await.ok();
    debug!("run_child completed.");
    child_proc.reg.write().await.remove(&child_proc.alias);

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


#[cfg(test)]
mod process_tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    #[test]
    fn test_process_stdout_ok() {
        // let job = async{
        //     let (mut stdout_rx, status_tx, mut status_rx, _start_rx, create, reader_out) = setup_test();
        //     let alias = create.desc.alias.clone();
        //     let process = Process::process_stdout(create,status_tx,reader_out);
        //     let reader = async move{
        //         while let Some(line) = stdout_rx.next().await{
        //             println!("{}",line);
        //         }
        //         Ok::<_,RexecError>(())
        //     };
        //     let status = async move{
        //         let status = status_rx.next().await.unwrap();
        //         Ok::<_,RexecError>(status)
        //     };
        //     let (p, r, s) = futures::join!(process, reader,status);
        //     assert!(p.is_ok());
        //     assert!(r.is_ok());
        //     let status_msg = s.unwrap();
        //     matches!(status_msg.status, ProcessStatus::EXITED);
        //     assert_eq!(status_msg.alias, alias);
        // };
        // tokio::runtime::Runtime:: new()
        //     .expect("Failed to create Tokio runtime")
        //     .block_on(job);

    }
    #[test]
    fn test_premature_receiver_close() {
        // let job = async{
        //     let (mut stdout_rx, status_tx, mut status_rx, _start_rx, create, reader_out) = setup_test();
        //     let alias = create.desc.alias.clone();
        //     let process = Process::process_stdout(create,status_tx,reader_out);
        //     let reader = async move{
        //         let mut line = stdout_rx.next().await.unwrap();
        //         println!("{}",line);
        //         line = stdout_rx.next().await.unwrap();
        //         println!("{}",line);

        //         Ok::<_,RexecError>(())
        //     };
        //     let status = async move{
        //         let status = status_rx.next().await.unwrap();
        //         Ok::<_,RexecError>(status)
        //     };
        //     let (p, r, s) = futures::join!(process, reader,status);
        //     assert!(!p.is_ok());
        //     matches!(p.err().unwrap().code, RexecErrorType::UnexpectedEof);
        //     assert!(r.is_ok());
        //     let status_msg = s.unwrap();
        //     matches!(status_msg.status, ProcessStatus::EXITED);
        //     assert_eq!(status_msg.alias, alias);
        // };
        // tokio::runtime::Runtime:: new()
        //     .expect("Failed to create Tokio runtime")
        //     .block_on(job);

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

    // fn setup_test<'a>() -> (Receiver<String>,
    //                         Sender<ProcessStatusMessage>,
    //                         Receiver<ProcessStatusMessage>,
    //                         oneshot::Receiver<StartConfirmation>,
    //                         ProcessCreateMessage,
    //                         Lines<BufReader<Cursor<&'a str>>>) {
    //     let (stdout_tx, stdout_rx) = mpsc::channel::<String>(1);
    //     let (status_tx, status_rx) = mpsc::channel::<ProcessStatusMessage>(1);
    //     let (start_tx, start_rx) = oneshot::channel::<StartConfirmation>();

    //     let desc = ProcessDescription::simple(
    //         "test".to_string(),
    //         "program".to_string(),
    //         Vec::new(),
    //         "work_dir".to_string(),
    //         HashMap::new()
    //     );
    //     let create = ProcessCreateMessage { desc, stdout_tx, start_tx: Some(start_tx) };
    //     let buffer = Cursor::new("1\n2\n3\n4\n5\n6\n");
    //     let reader_out = BufReader::new(buffer).lines();
    //     (stdout_rx, status_tx, status_rx, start_rx, create, reader_out)
    // }
}