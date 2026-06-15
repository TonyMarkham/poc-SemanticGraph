use crate::models::WidgetState;

pub struct FooBar {
    foo: String,
    bar: usize,
    widget_state: WidgetState,
}
impl FooBar {
    fn new(foo: String, bar: usize, widget_state: WidgetState) -> Self {
        Self { foo, bar, widget_state }
    }
}
