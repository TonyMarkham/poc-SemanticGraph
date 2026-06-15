use crate::{
    AuditNote, foo_bar::FooBar
};

pub struct FooBarBaz{
    foobar: FooBar,
    note: AuditNote,
}

impl FooBarBaz{
    fn new(foobar: FooBar, note: AuditNote) -> Self {
        Self { foobar, note, }
    }
}
