use crate::{CSharpLsWorker, ResolvedCallTarget, ResolvedReferenceTarget, solution_source_files};

use lsp_types::DocumentSymbol;
use std::{error::Error, io, path::PathBuf, process::Command};
use tokio::runtime::Builder;

#[test]
#[ignore = "requires installed csharp-ls on PATH"]
fn csharp_ls_help_responds() -> Result<(), Box<dyn Error>> {
    let output = Command::new("csharp-ls").arg("--help").output()?;
    let help_text = format!(
        "{}{}",
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?
    );

    assert!(output.status.success());
    assert!(help_text.contains("--solution"));
    Ok(())
}

#[test]
#[ignore = "requires installed csharp-ls and Roslyn/MSBuild build-host IPC"]
fn csharp_ls_returns_fixture_symbols_references_and_incoming_calls() -> Result<(), Box<dyn Error>> {
    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        let repo_root = repo_root()?;
        let solution_path = repo_root.join("__SmokeTestAssets__/csharp-wip/CSharpWip.sln");
        let model = crate::load_solution(&solution_path)?;
        let files = solution_source_files(&model);
        assert_eq!(files.len(), 1);
        let file_path = files
            .first()
            .cloned()
            .ok_or_else(|| io::Error::other("C# fixture had no source file"))?;

        let mut worker = CSharpLsWorker::start(
            PathBuf::from("csharp-ls"),
            solution_path,
            "warning".to_string(),
            Vec::new(),
            30_000,
            30_000,
        )
        .await?;

        let symbol_sets = worker
            .document_symbols_for_files(vec![file_path.clone()])
            .await?;
        let symbols = symbol_sets
            .first()
            .map(|(_path, symbols)| symbols)
            .ok_or_else(|| io::Error::other("C# fixture returned no document symbols"))?;
        assert!(find_symbol(symbols, "Worker").is_some());
        let format_symbol = find_symbol(symbols, "Format")
            .ok_or_else(|| io::Error::other("C# fixture returned no Format method"))?;

        let references = worker
            .references_for_symbol(&ResolvedReferenceTarget {
                file_path: file_path.clone(),
                selection_range: format_symbol.selection_range,
            })
            .await?;
        assert!(!references.references.is_empty());

        let incoming_calls = worker
            .incoming_calls_for_symbol(&ResolvedCallTarget {
                file_path,
                selection_range: format_symbol.selection_range,
            })
            .await?;
        assert!(
            incoming_calls
                .incoming_calls
                .iter()
                .any(|call| call.caller_name.starts_with("Run"))
        );

        worker.shutdown().await?;
        Ok::<(), Box<dyn Error>>(())
    })?;

    Ok(())
}

fn find_symbol<'a>(symbols: &'a [DocumentSymbol], name_prefix: &str) -> Option<&'a DocumentSymbol> {
    for symbol in symbols {
        if symbol.name.starts_with(name_prefix) {
            return Some(symbol);
        }
        if let Some(children) = &symbol.children
            && let Some(found) = find_symbol(children, name_prefix)
        {
            return Some(found);
        }
    }

    None
}

fn repo_root() -> Result<PathBuf, Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest_dir
        .parent()
        .ok_or_else(|| io::Error::other("csharp-ls-lib manifest dir has no parent"))?;
    let repo_root = crates_dir
        .parent()
        .ok_or_else(|| io::Error::other("crates directory has no parent"))?;

    Ok(repo_root.to_path_buf())
}
