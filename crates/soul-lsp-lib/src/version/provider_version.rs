pub fn provider_version() -> Option<String> {
    Some(format!(
        "soul-lsp-lib {} using pinned Soul submodule live scan",
        env!("CARGO_PKG_VERSION")
    ))
}
