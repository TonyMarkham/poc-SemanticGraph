use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FileActionKind {
    #[serde(rename = "created")]
    Create,

    #[serde(rename = "updated")]
    Update,

    #[serde(rename = "skipped")]
    Skip,

    #[serde(rename = "refused")]
    Refuse,
}

impl FileActionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Create => "created",
            Self::Update => "updated",
            Self::Skip => "skipped",
            Self::Refuse => "refused",
        }
    }

    pub fn writes_file(self) -> bool {
        matches!(self, Self::Create | Self::Update)
    }
}
