use crate::model::FtsLineRange;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FtsSearchSnippet {
    pub line_range: FtsLineRange,
    pub text: String,
}
