use heapsize::HeapSizeOf;
use itertools::Itertools;
use slog::Logger;
use slog::*;
use std::time::Instant;

#[derive(Debug)]
pub struct Timer {
    stack: Vec<StackFrame>,
    logger: Logger,
}

impl Timer {
    pub fn new(logger: &Logger) -> Timer {
        Timer {
            logger: logger.new(o!("phase" => "timer")),
            stack: Vec::new(),
        }
    }

    pub fn time_it<F, R>(&mut self, label: &'static str, thunk: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.start(label);
        let ret = thunk();
        self.pop();
        ret
    }

    pub(crate) fn push_frame(&mut self, frame: StackFrame) {
        self.stack.push(frame);
    }

    pub fn start(&mut self, label: &'static str) {
        let frame = StackFrame {
            label,
            started: Instant::now(),
        };
        debug!(self.logger, "Starting new timer frame"; "label" => label);

        self.push_frame(frame);
    }

    pub fn log_memory_usage(&self, items: &[&dyn HeapSizeOf]) {
        let bytes_used: usize = items.into_iter().map(|it| it.heap_size_of_children()).sum();

        debug!(self.logger, "Memory usage";
              "label" => self.label(),
              "bytes-used" => bytes_used);
    }

    pub fn pop(&mut self) {
        let frame = self
            .stack
            .pop()
            .expect("Tried to pop from the timer too many times");

        let duration = frame.started.elapsed();
        let micros = duration.subsec_micros() as f64 / 1_000_000.0;
        let secs = duration.as_secs() as f64 + micros;

        debug!(self.logger, "Pass finished";
              "label" => frame.label,
              "seconds" => secs);
    }

    fn label(&self) -> String {
        self.stack.iter().map(|frame| frame.label).join("/")
    }

    pub fn cancel(&mut self) {
        self.stack.clear();
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if !self.stack.is_empty() {
            error!(self.logger, "Timer was dropped before all timing frames were popped";
                   "frames" => &format_args!("{:?}", self.stack));
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct StackFrame {
    label: &'static str,
    started: Instant,
}
