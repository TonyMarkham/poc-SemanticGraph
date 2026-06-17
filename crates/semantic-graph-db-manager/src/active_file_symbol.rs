use crate::TextRange;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFileSymbol {
    pub node_id: String,
    pub symbol_key: String,
    pub kind: String,
    pub name: String,
    pub qualified_name: Option<String>,
    pub range: Option<TextRange>,
    pub selection_range: Option<TextRange>,
    pub container_node_id: Option<String>,
    pub properties_json: String,
}
