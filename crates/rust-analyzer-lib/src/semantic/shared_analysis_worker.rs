use crate::{
    RustAnalyzerLibError, RustAnalyzerLibResult,
    semantic::{
        analysis_worker_command::AnalysisWorkerCommand,
        shared_analysis_snapshot::SharedAnalysisSnapshot,
        shared_analysis_worker_handle::SharedAnalysisWorkerHandle,
    },
};

use std::{sync::mpsc, thread};

pub struct SharedAnalysisWorker;

impl SharedAnalysisWorker {
    pub fn start(
        snapshot: SharedAnalysisSnapshot,
    ) -> RustAnalyzerLibResult<SharedAnalysisWorkerHandle> {
        let (sender, receiver) = mpsc::channel();
        let (init_sender, init_receiver) = mpsc::channel();

        thread::spawn(move || {
            run_shared_analysis_worker(snapshot, receiver, init_sender);
        });

        init_receiver.recv().map_err(|_| {
            RustAnalyzerLibError::analysis_message(
                "start shared analysis worker",
                "shared analysis worker closed before initialization completed",
            )
        })??;

        Ok(SharedAnalysisWorkerHandle::new(sender))
    }
}

fn run_shared_analysis_worker(
    snapshot: SharedAnalysisSnapshot,
    receiver: mpsc::Receiver<AnalysisWorkerCommand>,
    init_sender: mpsc::Sender<RustAnalyzerLibResult<()>>,
) {
    let _send_result = init_sender.send(Ok(()));

    while let Ok(command) = receiver.recv() {
        let should_shutdown = matches!(command, AnalysisWorkerCommand::Shutdown { .. });
        handle_command(&snapshot, command);
        if should_shutdown {
            break;
        }
    }
}

fn handle_command(snapshot: &SharedAnalysisSnapshot, command: AnalysisWorkerCommand) {
    match command {
        AnalysisWorkerCommand::DocumentSymbols {
            file_paths,
            progress,
            response,
        } => {
            let result = match progress {
                Some(progress) => {
                    snapshot.document_symbols_for_files_with_progress(&file_paths, progress)
                }
                None => snapshot.document_symbols_for_files(&file_paths),
            };
            let _send_result = response.send(result);
        }
        AnalysisWorkerCommand::References { target, response } => {
            let _send_result = response.send(snapshot.references_for_symbol(&target));
        }
        AnalysisWorkerCommand::OutgoingCalls { target, response } => {
            let _send_result = response.send(snapshot.outgoing_calls_for_symbol(&target));
        }
        AnalysisWorkerCommand::FileSemantic { work, response } => {
            let _send_result = response.send(snapshot.file_semantic_work(work));
        }
        AnalysisWorkerCommand::Shutdown { response } => {
            let _send_result = response.send(Ok(()));
        }
    }
}
