use std::process::Stdio;

use log::{debug, error, info};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader, BufWriter},
    process::{Child, Command},
    sync::{broadcast, mpsc, oneshot},
};

use crate::{
    error::{RexecError, RexecErrorType},
    proc::{
        comm::{ExitMessage, ExitTx, Process, ProcessStatusId, StopMessage, StopRx},
        description::ProcessDescription,
    },
    register::RegisterRef,
};

use super::files::FileInfo;

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
    fileinfo: Option<FileInfo>,
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
            let fileinfo = FileInfo::next_file(&desc.alias, &desc.cwd).await?;
            debug!("filename {}", fileinfo.filename);

            let (bcst_tx, bcst_rx) = broadcast::channel::<String>(32);
            let (stop_tx, stop_rx) = oneshot::channel::<StopMessage>();
            let (exit_tx, exit_rx) = oneshot::channel::<ExitMessage>();
            let (stdin_tx, stdin_rx) = mpsc::channel::<String>(128);

            let proc = Process {
                desc: desc.clone(),
                filename: fileinfo.filename.clone(),
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
                    fileinfo: Some(fileinfo),
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
    let mut fileinfo = child_proc.fileinfo.take().unwrap();

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
            let l = format!("[OUT]{line}\n");
            let _ = &mut fileinfo.write(&l).await;
            ch_bcast.as_ref().map(|b| b.send(l).ok());
        },
        Ok(line) = write_log(&mut stderr) => {
            debug!("[ERR] {line}");
            let l = format!("[OUT]{line}\n");
            let _ = &mut fileinfo.write(&l).await;
            ch_bcast.as_ref().map(|b| b.send(l).ok());
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
    fileinfo.sync_all().await.ok();
    debug!("run_child IO loop completed.");
}
#[cfg(test)]
mod process_tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    #[test]
    fn test_process_stdout_ok() {}
}
