#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteScope {
    value: &'static str,
}

impl RouteScope {
    pub const FILE: Self = Self { value: "file" };

    pub const WORKSPACE: Self = Self { value: "workspace" };

    pub fn as_str(self) -> &'static str {
        self.value
    }
}
