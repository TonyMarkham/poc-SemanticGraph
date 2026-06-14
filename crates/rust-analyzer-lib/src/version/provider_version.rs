pub fn provider_version() -> Option<String> {
    Some(format!(
        "rust-analyzer-lib {} using pinned rust-analyzer submodule",
        env!("CARGO_PKG_VERSION")
    ))
}
