use crate::{Widget, WidgetId, WidgetProcessor};

#[test]
fn processor_tracks_active_widgets() {
    let mut processor = WidgetProcessor::new();
    processor.ingest(Widget::new(WidgetId::new(1), "alpha"));
    processor.ingest(Widget::new(WidgetId::new(2), "beta").activate("ops"));

    assert_eq!(processor.active_count(), 1);
    assert_eq!(
        processor.summaries(),
        vec![
            "widget alpha is draft".to_string(),
            "widget beta is active for ops".to_string(),
        ]
    );
}
