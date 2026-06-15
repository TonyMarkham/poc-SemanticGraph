use crate::{AuditNote, foo_bar::FooBar, WidgetState};

pub struct FooBarBaz{
    foobar: FooBar,
    note: AuditNote,
}

impl FooBarBaz{
    fn new() -> Self {
        Self {
            foobar: FooBar::new(String::from("foo"), 16, WidgetState::Draft),
            note: AuditNote::new("001", "Test"),
        }
    }
}
