use crate::progress::{ProgressOutput, ProgressTask};

use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct ProgressReporter {
    output: Option<Arc<Mutex<ProgressOutput>>>,
}

impl ProgressReporter {
    pub fn disabled() -> Self {
        Self { output: None }
    }

    pub fn stderr(enabled: bool) -> Self {
        if enabled {
            Self::from_writer(Box::new(io::stderr()))
        } else {
            Self::disabled()
        }
    }

    pub fn task(&self, total: usize, label: impl Into<String>) -> ProgressTask {
        ProgressTask::new(self.output.clone(), total, label.into())
    }

    fn from_writer(writer: Box<dyn Write + Send>) -> Self {
        Self {
            output: Some(Arc::new(Mutex::new(ProgressOutput::new(writer)))),
        }
    }

    #[cfg(test)]
    pub fn with_writer(writer: Box<dyn Write + Send>) -> Self {
        Self::from_writer(writer)
    }
}
