use crate::{
    SoulLspConfig, SoulLspLibError, SoulLspLibResult,
    semantic::{
        analysis_worker_command::AnalysisWorkerCommand,
        analysis_worker_handle::AnalysisWorkerHandle, loaded_soul_workspace::LoadedSoulWorkspace,
    },
};

use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

pub struct AnalysisWorker;

impl AnalysisWorker {
    pub fn start(workspace_root: impl AsRef<Path>) -> SoulLspLibResult<AnalysisWorkerHandle> {
        Self::start_with_optional_config(workspace_root, None)
    }

    pub fn start_with_config(
        workspace_root: impl AsRef<Path>,
        config: SoulLspConfig,
    ) -> SoulLspLibResult<AnalysisWorkerHandle> {
        Self::start_with_optional_config(workspace_root, Some(config))
    }

    fn start_with_optional_config(
        workspace_root: impl AsRef<Path>,
        config: Option<SoulLspConfig>,
    ) -> SoulLspLibResult<AnalysisWorkerHandle> {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let (sender, receiver) = mpsc::channel();
        let (init_sender, init_receiver) = mpsc::channel();

        thread::spawn(move || {
            run_analysis_worker(workspace_root, config, receiver, init_sender);
        });

        init_receiver.recv().map_err(|_| {
            SoulLspLibError::analysis_message(
                "start analysis worker",
                "analysis worker closed before initialization completed",
            )
        })??;

        Ok(AnalysisWorkerHandle::new(sender))
    }
}

fn run_analysis_worker(
    workspace_root: PathBuf,
    config: Option<SoulLspConfig>,
    receiver: mpsc::Receiver<AnalysisWorkerCommand>,
    init_sender: mpsc::Sender<SoulLspLibResult<()>>,
) {
    let loaded_result = match config {
        Some(config) => LoadedSoulWorkspace::load_with_config(&workspace_root, config),
        None => LoadedSoulWorkspace::load(&workspace_root),
    };
    let loaded = match loaded_result {
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

fn handle_command(loaded: &LoadedSoulWorkspace, command: AnalysisWorkerCommand) {
    match command {
        AnalysisWorkerCommand::DocumentSymbols {
            file_paths,
            response,
        } => {
            let _send_result = response.send(loaded.document_symbols_for_files(file_paths));
        }
        AnalysisWorkerCommand::References { target, response } => {
            let _send_result = response.send(loaded.references_for_symbol(target));
        }
        AnalysisWorkerCommand::FileSemantic { work, response } => {
            let _send_result = response.send(loaded.file_semantic_work(work));
        }
        AnalysisWorkerCommand::Shutdown { response } => {
            let _send_result = response.send(Ok(()));
        }
    }
}
