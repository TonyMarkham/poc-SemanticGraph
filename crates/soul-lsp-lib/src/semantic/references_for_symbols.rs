use crate::{
    SoulLspLibResult,
    model::{ResolvedReferenceSet, ResolvedReferenceTarget},
    semantic::loaded_soul_workspace::LoadedSoulWorkspace,
};

use std::path::Path;

pub fn references_for_symbols(
    workspace_root: impl AsRef<Path>,
    targets: &[ResolvedReferenceTarget],
) -> SoulLspLibResult<Vec<ResolvedReferenceSet>> {
    let loaded = LoadedSoulWorkspace::load(workspace_root)?;
    targets
        .iter()
        .cloned()
        .map(|target| loaded.references_for_symbol(target))
        .collect()
}
