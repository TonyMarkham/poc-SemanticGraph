use crate::{
    constants::render::TEMP_RENDER_DIR_PREFIX,
    error::{AgentAssetsError, AgentAssetsResult},
};
use std::{
    fs,
    path::PathBuf,
    process,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn create_temp_render_root() -> AgentAssetsResult<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    for attempt in 0..100u32 {
        let path = std::env::temp_dir().join(format!(
            "{TEMP_RENDER_DIR_PREFIX}-{}-{nanos}-{attempt}",
            process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(AgentAssetsError::io(
                    "create temporary render directory",
                    Some(path),
                    source,
                ));
            }
        }
    }

    Err(AgentAssetsError::invalid_manifest(
        "could not allocate a unique temporary render directory",
    ))
}
