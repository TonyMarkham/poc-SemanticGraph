use crate::model::{
    CallBatchExtraction, CallRouteSummary, ReferenceBatchExtraction, ReferenceRouteSummary,
};

use std::collections::HashSet;

pub(crate) fn reference_route_summary_for_origin_files(
    extraction: &ReferenceBatchExtraction,
    origin_file_uris: &HashSet<String>,
) -> ReferenceRouteSummary {
    let mut reference_edges = 0;
    let mut reference_occurrences = 0;
    let mut file_fallbacks = 0;

    for reference in &extraction.references {
        let occurrence_count = reference
            .occurrences
            .iter()
            .filter(|occurrence| origin_file_uris.contains(&occurrence.file_uri))
            .count();
        if occurrence_count == 0 {
            continue;
        }

        reference_edges += 1;
        reference_occurrences += occurrence_count;
        if reference.source_resolution == "file_fallback" {
            file_fallbacks += 1;
        }
    }

    ReferenceRouteSummary {
        targets_queried: extraction.summary.targets_queried,
        reference_edges,
        reference_occurrences,
        file_fallbacks,
        skipped_external: 0,
    }
}

pub(crate) fn call_route_summary_for_origin_files(
    extraction: &CallBatchExtraction,
    origin_file_uris: &HashSet<String>,
) -> CallRouteSummary {
    let mut call_edges = 0;
    let mut call_occurrences = 0;
    let mut callable_nodes = HashSet::new();

    for call in &extraction.calls {
        let occurrence_count = call
            .occurrences
            .iter()
            .filter(|occurrence| origin_file_uris.contains(&occurrence.file_uri))
            .count();
        if occurrence_count == 0 {
            continue;
        }

        call_edges += 1;
        call_occurrences += occurrence_count;
        callable_nodes.insert(call.caller_symbol_key.clone());
    }

    CallRouteSummary {
        callable_nodes: callable_nodes.len(),
        call_edges,
        call_occurrences,
        skipped_external_targets: 0,
        skipped_unresolved_targets: 0,
        skipped_non_callable_prepare_items: 0,
    }
}
