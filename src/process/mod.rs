/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

pub(crate) mod description;
pub(crate) mod execute;

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