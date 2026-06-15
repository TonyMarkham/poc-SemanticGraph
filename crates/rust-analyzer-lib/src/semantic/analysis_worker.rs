use crate::{
    RustAnalyzerLibError, RustAnalyzerLibResult,
    semantic::{
        analysis_worker_command::AnalysisWorkerCommand,
        analysis_worker_handle::AnalysisWorkerHandle, loaded_analysis::LoadedAnalysis,
    },
};

use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

pub struct AnalysisWorker;

impl AnalysisWorker {
    pub fn start(workspace_root: impl AsRef<Path>) -> RustAnalyzerLibResult<AnalysisWorkerHandle> {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let (sender, receiver) = mpsc::channel();
        let (init_sender, init_receiver) = mpsc::channel();

        thread::spawn(move || {
            run_analysis_worker(workspace_root, receiver, init_sender);
        });

        init_receiver.recv().map_err(|_| {
            RustAnalyzerLibError::analysis_message(
                "start analysis worker",
                "analysis worker closed before initialization completed",
            )
        })??;

        Ok(AnalysisWorkerHandle::new(sender))
    }
}

fn run_analysis_worker(
    workspace_root: PathBuf,
    receiver: mpsc::Receiver<AnalysisWorkerCommand>,
    init_sender: mpsc::Sender<RustAnalyzerLibResult<()>>,
) {
    let loaded = match LoadedAnalysis::load(&workspace_root) {
        Ok(loaded) => {
            let _send_result = init_sender.send(Ok(()));
            loaded
        }
        Err(error) => {
            let _send_result = init_sender.send(Err(error));
            return;
        }
    };

    while let Ok(command) = receiver.recv() {
        let should_shutdown = matches!(command, AnalysisWorkerCommand::Shutdown { .. });
        handle_command(&loaded, command);
        if should_shutdown {
            break;
        }
    }
}

fn handle_command(loaded: &LoadedAnalysis, command: AnalysisWorkerCommand) {
    match command {
        AnalysisWorkerCommand::DocumentSymbols {
            file_paths,
            response,
        } => {
            let _send_result = response.send(loaded.document_symbols_for_files(&file_paths));
        }
        AnalysisWorkerCommand::References { target, response } => {
            let _send_result = response.send(loaded.references_for_symbol(&target));
        }
        AnalysisWorkerCommand::OutgoingCalls { target, response } => {
            let _send_result = response.send(loaded.outgoing_calls_for_symbol(&target));
        }
        AnalysisWorkerCommand::FileSemantic { work, response } => {
            let _send_result = response.send(loaded.file_semantic_work(work));
        }
        AnalysisWorkerCommand::Shutdown { response } => {
            let _send_result = response.send(Ok(()));
        }
    }
}
