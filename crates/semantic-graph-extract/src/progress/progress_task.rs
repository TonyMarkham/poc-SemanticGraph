use crate::progress::{ProgressOutput, ProgressTaskState};

use std::sync::{Arc, Mutex, atomic::Ordering};

const BAR_WIDTH: usize = 20;

#[derive(Clone)]
pub struct ProgressTask {
    state: Arc<ProgressTaskState>,
}

impl ProgressTask {
    pub fn disabled() -> Self {
        Self::new(None, 0, String::new())
    }

    pub(crate) fn new(
        output: Option<Arc<Mutex<ProgressOutput>>>,
        total: usize,
        label: String,
    ) -> Self {
        Self {
            state: Arc::new(ProgressTaskState::new(output, total, label)),
        }
    }

    pub fn tick(&self) {
        let Some(_output) = &self.state.output else {
            return;
        };

        let current = increment_current(&self.state.current, self.state.total);
        self.write(current, current >= self.state.total);
    }

    pub fn finish(&self) {
        let current = self.state.current.load(Ordering::Acquire);
        self.write(current.min(self.state.total), true);
    }

    fn write(&self, current: usize, finished: bool) {
        if finished && self.state.finished.swap(true, Ordering::AcqRel) {
            return;
        }

        let Some(output) = &self.state.output else {
            return;
        };
        let line = render_progress_line(current, self.state.total, &self.state.label);
        if let Ok(mut output) = output.lock() {
            let _write_result = output.write_line(&line, finished);
        }
    }
}

fn increment_current(current: &std::sync::atomic::AtomicUsize, total: usize) -> usize {
    let mut observed = current.load(Ordering::Acquire);
    loop {
        let next = observed.saturating_add(1).min(total);
        match current.compare_exchange_weak(observed, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_previous) => return next,
            Err(actual) => observed = actual,
        }
    }
}

pub fn render_progress_line(current: usize, total: usize, label: &str) -> String {
    let current = current.min(total);
    let filled = current
        .saturating_mul(BAR_WIDTH)
        .checked_div(total)
        .unwrap_or(BAR_WIDTH);
    let percent = current
        .saturating_mul(100)
        .checked_div(total)
        .unwrap_or(100);
    let empty = BAR_WIDTH - filled;

    format!(
        "[{}{}] {}/{} {}% {}",
        "#".repeat(filled),
        "-".repeat(empty),
        current,
        total,
        percent,
        ascii_label(label)
    )
}

fn ascii_label(label: &str) -> String {
    label
        .chars()
        .map(|value| {
            if value.is_ascii() && !value.is_ascii_control() {
                value
            } else {
                '?'
            }
        })
        .collect()
}
