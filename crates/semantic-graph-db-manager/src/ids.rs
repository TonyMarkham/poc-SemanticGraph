use sha2::{Digest, Sha256};

pub fn node_id(workspace_id: i64, language: &str, symbol_key: &str) -> String {
    hash_parts(&[&workspace_id.to_string(), language, symbol_key])
}

pub fn edge_id(
    workspace_id: i64,
    src_node_id: &str,
    dst_node_id: &str,
    relation: &str,
    context: Option<&str>,
) -> String {
    hash_parts(&[
        &workspace_id.to_string(),
        src_node_id,
        dst_node_id,
        relation,
        context.unwrap_or_default(),
    ])
}

fn hash_parts(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();

    for part in parts {
        hasher.update(part.len().to_string());
        hasher.update([0]);
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }

    hex::encode(hasher.finalize())
}
