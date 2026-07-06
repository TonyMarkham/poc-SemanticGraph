use crate::progress::ProgressOutput;

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize},
};

pub(crate) struct ProgressTaskState {
    pub(crate) output: Option<Arc<Mutex<ProgressOutput>>>,
    pub(crate) total: usize,
    pub(crate) label: String,
    pub(crate) current: AtomicUsize,
    pub(crate) finished: AtomicBool,
}

impl ProgressTaskState {
    pub(crate) fn new(
        output: Option<Arc<Mutex<ProgressOutput>>>,
        total: usize,
        label: String,
    ) -> Self {
        Self {
            output,
            total,
            label,
            current: AtomicUsize::new(0),
            finished: AtomicBool::new(false),
        }
    }
}
