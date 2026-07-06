use std::io::{self, Write};

pub(crate) struct ProgressOutput {
    writer: Box<dyn Write + Send>,
    last_len: usize,
}

impl ProgressOutput {
    pub(crate) fn new(writer: Box<dyn Write + Send>) -> Self {
        Self {
            writer,
            last_len: 0,
        }
    }

    pub(crate) fn write_line(&mut self, line: &str, finished: bool) -> io::Result<()> {
        let padding = self.last_len.saturating_sub(line.len());
        write!(self.writer, "\r{line}")?;
        for _ in 0..padding {
            write!(self.writer, " ")?;
        }
        if finished {
            writeln!(self.writer)?;
            self.last_len = 0;
        } else {
            self.last_len = line.len();
        }
        self.writer.flush()
    }
}
