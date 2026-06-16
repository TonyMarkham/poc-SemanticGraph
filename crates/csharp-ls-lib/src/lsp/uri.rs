use crate::{CSharpLsLibError, CSharpLsLibResult};

use lsp_types::Url;
use std::path::{Path, PathBuf};

pub(crate) fn file_uri(path: &Path) -> CSharpLsLibResult<Url> {
    Url::from_file_path(path)
        .map_err(|()| CSharpLsLibError::invalid_path(path, "could not convert path to file URI"))
}

pub(crate) fn path_from_file_uri(uri: &Url) -> CSharpLsLibResult<PathBuf> {
    uri.to_file_path().map_err(|()| {
        CSharpLsLibError::invalid_path(
            PathBuf::from(uri.to_string()),
            "could not convert file URI to local path",
        )
    })
}
