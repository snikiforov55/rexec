use std::process::Stdio;

use chrono::Utc;

use log::{debug, error, info};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader, BufWriter},
    process::{Child, Command},
    sync::{broadcast, mpsc, oneshot},
};

use crate::{
    error::{RexecError, RexecErrorType},
    proc::comm::ProcessStatusId,
    register::RegisterRef,
};

use crate::proc::comm::{ExitMessage, ExitTx, Process, StopMessage, StopRx};
use crate::proc::description::ProcessDescription;

pub async fn start(reg: &RegisterRef, desc: &ProcessDescription) -> Result<(), RexecError> {
    // Some more advanced checks might be required.
    if reg.read().await.get(&desc.alias).is_some() {
        return Err(RexecError::code(RexecErrorType::AlreadyRunning));
    }
    do_start(reg, desc).await
}

struct ChildProc {
    alias: String,
    child: Child,
    stop_rx: StopRx,
    exit_tx: ExitTx,
    reg: RegisterRef,
    bcst_tx: Option<broadcast::Sender<String>>,
    stdin_rx: Option<mpsc::Receiver<String>>,
    filename: String,
}

async fn do_start(reg: &RegisterRef, desc: &ProcessDescription) -> Result<(), RexecError> {
    let child_res = Command::new(&desc.cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .args(&desc.args)
        .current_dir(&desc.cwd)
        .envs(&desc.envs)
        .spawn();

    match child_res {
        Ok(child) => {
            debug!("All stdin, stdout, or stderr are OK for {}", desc.alias);
            let date = Utc::now().format("%Y%M%d-%H%M%S");
            let filename = format!("{}-utc-{date}.log", desc.alias);
            let (bcst_tx, bcst_rx) = broadcast::channel::<String>(32);
            debug!("filename {filename}");

            let (stop_tx, stop_rx) = oneshot::channel::<StopMessage>();
            let (exit_tx, exit_rx) = oneshot::channel::<ExitMessage>();
            let (stdin_tx, stdin_rx) = mpsc::channel::<String>(128);

            let proc = Process {
                desc: desc.clone(),
                filename: filename.clone(),
                stop_tx: Some(stop_tx),
                exit_rx: Some(exit_rx),
                bcst_rx,
                stdin_tx,
                status: ProcessStatusId::Run,
            };
            let a = desc.alias.clone();
            let reg_ref = reg.clone();
            tokio::task::spawn(async move {
                run_child(ChildProc {
                    alias: a,
                    child,
                    stop_rx,
                    exit_tx,
                    reg: reg_ref,
                    bcst_tx: Some(bcst_tx),
                    filename,
                    stdin_rx: Some(stdin_rx),
                })
                .await
            });
            reg.write().await.add(proc);
            Ok(())
        }
        Err(e) => {
            info!("FailedToExecuteProcess {}", &e.to_string());
            Err(RexecError::code(RexecErrorType::FailedToExecuteProcess))
        }
    }
}

async fn write_log<T: AsyncRead + Unpin>(
    lines: &mut tokio::io::Lines<BufReader<T>>,
) -> Result<String, RexecError> {
    match lines.next_line().await {
        Err(e) => Err(RexecError::code_msg(
            RexecErrorType::UnexpectedEof,
            e.to_string(),
        )),
        Ok(Some(line)) => Ok(line),
        Ok(None) => Err(RexecError::code(RexecErrorType::UnexpectedEof)),
    }
}
fn open_file(filename: &String, alias: &String, dir: &String) {}

async fn run_child(mut child_proc: ChildProc) {
    let io = child_proc.child.stdout.take().and_then(|o| {
        child_proc
            .child
            .stderr
            .take()
            .and_then(|e| child_proc.child.stdin.take().map(|i| (o, e, i)))
    });
    if io.is_none() {
        debug!("The child process doesn't have out, err, or in streams. No need to continue the run. Killing child process.");
        child_proc.child.kill().await.ok();
        child_proc.child.wait().await.ok();
        child_proc.exit_tx.send(ExitMessage {}).ok();
        return;
    }
    let alias = child_proc.alias.clone();
    let (ch_exit_tx, mut ch_exit_rx) = tokio::sync::oneshot::channel();
    let ch_bcast = child_proc.bcst_tx.take();
    let mut stdin_rx = child_proc.stdin_rx.take().unwrap();

    tokio::spawn(async move {
        let success = tokio::select! {
            Ok(status) = child_proc.child.wait() => {
                        debug!("Child finished with a status code {status}");
                        true
            },
            _ = &mut child_proc.stop_rx => {
                debug!("Received the process termination via the stop_tx");
                match child_proc.child.kill().await{
                    Ok(_) => {
                        debug!("Process {} stopped.", &child_proc.alias);
                        true
                    },
                    Err(e) => {
                        debug!("Failed to stop process {}, because {e}", &child_proc.alias);
                        child_proc.reg
                            .write()
                            .await
                            .get_mut(&child_proc.alias)
                            .map(|p|{p.status = ProcessStatusId::Failed; p});
                        false
                    },
                }
            },
        };
        debug!("run_child completed.");
        if success {
            child_proc.reg.write().await.remove(&child_proc.alias);
        }
        // Notify the IO coroutine.
        ch_exit_tx.send(()).ok();
        // Notify the Register that the process has exited.
        child_proc.exit_tx.send(ExitMessage {}).ok();
    });
    // The None is already checked before.
    let (o, e, i) = io.unwrap();
    let mut stdout = BufReader::new(o).lines();
    let mut stderr = BufReader::new(e).lines();
    let mut stdin = BufWriter::new(i);
    loop {
        tokio::select! {
        Ok(line) = write_log(&mut stdout) => {
            debug!("[OUT] {line}");
            ch_bcast.as_ref().map(|b| b.send(format!("[OUT]{line}")).ok());
        },
        Ok(line) = write_log(&mut stderr) => {
            debug!("[ERR] {line}");
            ch_bcast.as_ref().map(|b| b.send(format!("[ERR]{line}")).ok());
        },
        Some(line) = stdin_rx.recv() => {
            match stdin.write(line.as_bytes()).await{
                Ok(0) => break, // Most probably the destination is closed
                Ok(_) => (),//number of bytes written. just continue. the bufwriter takes care about partial writes.
                Err(e) => {
                    // Log the error and continue.
                    error!("Failed to write to the {} stdin. Because: {e}", &alias)
                }
            }
            stdin.flush().await.ok();
        },
        _ = &mut ch_exit_rx => break,
        else => {
            debug!("select! detected the default exit condition.");
            break
        }}
    }
    debug!("run_child IO loop completed.");
}
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
