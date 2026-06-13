#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetId(u64);

impl WidgetId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetState {
    Draft,
    Active { owner: String },
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Widget {
    pub id: WidgetId,
    pub name: String,
    pub state: WidgetState,
}

impl Widget {
    pub fn new(id: WidgetId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            state: WidgetState::Draft,
        }
    }

    pub fn activate(mut self, owner: impl Into<String>) -> Self {
        self.state = WidgetState::Active {
            owner: owner.into(),
        };
        self
    }

    pub fn retire(mut self) -> Self {
        self.state = WidgetState::Retired;
        self
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, WidgetState::Active { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditNote {
    pub code: String,
    pub message: String,
}

impl AuditNote {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
