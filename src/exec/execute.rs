use std::{process::Stdio, sync::Arc};

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
    util::{config::Config, time::time_stamp_fsec},
};

use super::files::FileInfo;

pub async fn start(conf: &Arc<Config>, reg: &RegisterRef, desc: &ProcessDescription) -> Result<(), RexecError> {
    // Some more advanced checks might be required.
    if reg.read().await.get(&desc.alias).is_some() {
        return Err(RexecError::code(RexecErrorType::AlreadyRunning));
    }
    do_start(conf, reg, desc).await
}

struct ChildProc {
    alias: String,
    child: Option<Child>,
    stop_rx: Option<StopRx>,
    reg: RegisterRef,
    exit_tx: ExitTx,
    bcst_tx: broadcast::Sender<String>,
    stdin_rx: mpsc::Receiver<String>,
    fileinfo: FileInfo,
}

async fn do_start(conf: &Arc<Config>, reg: &RegisterRef, desc: &ProcessDescription) -> Result<(), RexecError> {
    // Todo. Register the process as soon as possible to avoid a race
    // condition if two requests are coming at the same time.
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
            reg.write().await.add(proc);

            let a = desc.alias.clone();
            let reg_ref = reg.clone();
            tokio::task::spawn(async move {
                run_child(ChildProc {
                    alias: a,
                    child: Some(child),
                    stop_rx: Some(stop_rx),
                    exit_tx,
                    reg: reg_ref,
                    bcst_tx,
                    fileinfo,
                    stdin_rx,
                })
                .await
            });
            Ok(())
        }
        Err(e) => {
            info!("FailedToExecuteProcess {}", &e.to_string());
            Err(RexecError::code(RexecErrorType::FailedToExecuteProcess))
        }
    }
}

async fn next_line<T: AsyncRead + Unpin>(
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
async fn write_to_log(kind: &str, line: &String, child_proc: &mut ChildProc) {
    let l = format!("{}|{kind}|{line}\n", time_stamp_fsec());
    debug!("{l}");
    let _ = &mut child_proc.fileinfo.write(&l).await;
    child_proc.bcst_tx.send(l).ok();
}
async fn run_child(mut child_proc: ChildProc) {
    // Store the child for the future move to the child thread
    let mut child = child_proc.child.take().unwrap();

    let io = child
        .stdout
        .take()
        .and_then(|o| {
            child
                .stderr
                .take()
                .and_then(|e| child
                    .stdin
                    .take()
                    .map(|i| (o, e, i)))
    });

    if io.is_none() {
        debug!("The child process doesn't have out, err, or in streams. No need to continue the run. Killing child process.");
        child.kill().await.ok();
        child.wait().await.ok();
        child_proc.exit_tx.send(ExitMessage {}).ok();
        child_proc
            .reg
            .write()
            .await
            .get_mut(&child_proc.alias)
            .map(|p| p.status = ProcessStatusId::Failed);
        return;
    }
    let alias = child_proc.alias.clone();
    let mut stop_rx = child_proc.stop_rx.take().unwrap();
    let mut child_handle = tokio::spawn(async move {
        tokio::select! {
            Ok(status) = child.wait() => {
                debug!("Child finished with a status code {status}");
                true
            },
            _ = &mut stop_rx => {
                debug!("Received the process termination via the stop_tx");
                match child.kill().await{
                    Ok(_) => {
                        debug!("Process {} stopped.", &alias);
                        true
                    },
                    Err(e) => {
                        debug!("Failed to stop process {}, because {e}", &alias);
                        false
                    },
                }
            },
        }
    });
    child_proc
            .reg
            .write()
            .await
            .get_mut(&child_proc.alias)
            .map(|p| p.status = ProcessStatusId::Run);
    // The None is already checked before.
    let (o, e, i) = io.unwrap();
    let mut stdout = BufReader::new(o).lines();
    let mut stderr = BufReader::new(e).lines();
    let mut stdin = BufWriter::new(i);
    let mut success = true;
    loop {
        tokio::select! {
        Ok(line) = next_line(&mut stdout) => write_to_log("OUT", &line, &mut child_proc).await,
        Ok(line) = next_line(&mut stderr) => write_to_log("ERR", &line, &mut child_proc).await,
        Some(line) = child_proc.stdin_rx.recv() => {
            match stdin.write(line.as_bytes()).await{
                Ok(0) => break, // Most probably the destination is closed
                Ok(_) => (),//number of bytes written. just continue. the bufwriter takes care about partial writes.
                Err(e) => {
                    // Log the error and continue.
                    error!("Failed to write to the {} stdin. Because: {e}", &child_proc.alias)
                }
            }
            stdin.flush().await.ok();
        },
        res_child = &mut child_handle => {
            match res_child { 
                Ok(s) => success = s,
                _ => success = false
            }
            break
        },
        else => {
            debug!("select! detected the default exit condition.");
            break
        }}
    }
    // Read all lines remaining in the stdout and stderr
    while let Ok(line) = next_line(&mut stdout).await {
        write_to_log("OUT", &line, &mut child_proc).await
    }
    while let Ok(line) = next_line(&mut stderr).await {
        write_to_log("ERR", &line, &mut child_proc).await
    }
    // Flush the file to the disk
    child_proc.fileinfo.sync_all().await.ok();
    // Notify the Register that the process has exited.
    child_proc.exit_tx.send(ExitMessage {}).ok();
    //If child finished with error, do not remove, but set the Status accordingly
    if success {
        child_proc.reg.write().await.remove(&child_proc.alias);
    } else {
        child_proc
            .reg
            .write()
            .await
            .get_mut(&child_proc.alias)
            .map(|p| p.status = ProcessStatusId::Failed);
    }
    debug!("run_child IO loop completed.");
}
#[cfg(test)]
mod process_tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    #[test]
    fn test_process_stdout_ok() {}
}
