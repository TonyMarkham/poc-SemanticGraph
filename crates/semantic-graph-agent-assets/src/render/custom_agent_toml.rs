use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct CustomAgentToml<'a> {
    pub(crate) name: &'a str,
    pub(crate) description: &'a str,
    pub(crate) developer_instructions: &'a str,
}
