use crate::{Config, edge_id, node_id};

#[test]
fn deterministic_ids_are_stable() {
    let first_node_id = node_id(1, "rust", "file:///demo/src/lib.rs#function:caller:1:0");
    let second_node_id = node_id(1, "rust", "file:///demo/src/lib.rs#function:caller:1:0");
    assert_eq!(first_node_id, second_node_id);
    assert_eq!(
        first_node_id,
        "028199ced09ed29adb1aaf9521f63e9d90ef333aba2066f90c51ce4be1739b9c"
    );

    let first_edge_id = edge_id(1, &first_node_id, "callee", "calls", None);
    let second_edge_id = edge_id(1, &first_node_id, "callee", "calls", None);
    assert_eq!(first_edge_id, second_edge_id);
    assert_eq!(
        first_edge_id,
        "3853fd7d3afaa05a34a9142501247d7ec4aabbd056df2cdc842de489766c5193"
    );
}

#[test]
fn default_write_config_matches_plan_defaults() {
    let config = Config::default();

    assert_eq!(config.queue_capacity(), 4096);
    assert_eq!(config.max_rows_per_commit(), 1000);
    assert_eq!(config.max_millis_per_commit(), 250);
    assert_eq!(config.busy_timeout_ms(), 5000);
}
