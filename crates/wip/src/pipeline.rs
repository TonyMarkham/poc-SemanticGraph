use crate::{AuditNote, Widget, WidgetId, WidgetState};

pub trait RenderSummary {
    fn render_summary(&self) -> String;
}

impl RenderSummary for Widget {
    fn render_summary(&self) -> String {
        match &self.state {
            WidgetState::Draft => format!("widget {} is draft", self.name),
            WidgetState::Active { owner } => {
                format!("widget {} is active for {owner}", self.name)
            }
            WidgetState::Retired => format!("widget {} is retired", self.name),
        }
    }
}

impl RenderSummary for AuditNote {
    fn render_summary(&self) -> String {
        format!("{}: {}", self.code, self.message)
    }
}

pub trait WidgetStore {
    fn upsert(&mut self, widget: Widget);
    fn get(&self, id: WidgetId) -> Option<&Widget>;
    fn all(&self) -> Vec<&Widget>;
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MemoryWidgetStore {
    widgets: Vec<Widget>,
}

impl MemoryWidgetStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_count(&self) -> usize {
        self.widgets
            .iter()
            .filter(|widget| widget.is_active())
            .count()
    }
}

impl WidgetStore for MemoryWidgetStore {
    fn upsert(&mut self, widget: Widget) {
        if let Some(existing) = self.widgets.iter_mut().find(|item| item.id == widget.id) {
            *existing = widget;
        } else {
            self.widgets.push(widget);
        }
    }

    fn get(&self, id: WidgetId) -> Option<&Widget> {
        self.widgets.iter().find(|widget| widget.id == id)
    }

    fn all(&self) -> Vec<&Widget> {
        self.widgets.iter().collect()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WidgetProcessor {
    store: MemoryWidgetStore,
}

impl WidgetProcessor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ingest(&mut self, widget: Widget) {
        self.store.upsert(widget);
    }

    pub fn active_count(&self) -> usize {
        self.store.active_count()
    }

    pub fn summaries(&self) -> Vec<String> {
        self.store
            .all()
            .into_iter()
            .map(RenderSummary::render_summary)
            .collect()
    }
}
