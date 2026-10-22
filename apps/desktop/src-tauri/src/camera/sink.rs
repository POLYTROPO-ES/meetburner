//! Virtual camera sinks (ADR-0002). The Windows implementation lives in `sink/windows.rs`.

use std::fmt;
use std::sync::{Arc, Mutex};

use crate::camera::frame::Frame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SinkError {
    /// `send` was called before `start`.
    NotStarted,
    Io(String),
}

impl fmt::Display for SinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SinkError::NotStarted => write!(f, "the virtual camera was not started"),
            SinkError::Io(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for SinkError {}

pub trait VirtualCameraSink: Send {
    fn start(&mut self, width: u32, height: u32, fps: u32) -> Result<(), SinkError>;
    fn send(&mut self, frame: &Frame) -> Result<(), SinkError>;
    fn stop(&mut self);
}

/// Discards frames. Used on platforms without a sink yet (Linux: M5).
#[derive(Debug, Default)]
pub struct NullSink {
    running: bool,
}

impl VirtualCameraSink for NullSink {
    fn start(&mut self, _width: u32, _height: u32, _fps: u32) -> Result<(), SinkError> {
        self.running = true;
        Ok(())
    }

    fn send(&mut self, _frame: &Frame) -> Result<(), SinkError> {
        if self.running {
            Ok(())
        } else {
            Err(SinkError::NotStarted)
        }
    }

    fn stop(&mut self) {
        self.running = false;
    }
}

/// Keeps the frames in memory so tests can inspect what the renderer produced.
#[derive(Clone, Debug, Default)]
pub struct RecordingSink {
    frames: Arc<Mutex<Vec<Frame>>>,
    state: Arc<Mutex<RecordingState>>,
}

#[derive(Debug, Default)]
struct RecordingState {
    running: bool,
    starts: usize,
    stops: usize,
    sizes: Vec<(u32, u32)>,
}

impl RecordingSink {
    pub fn frames(&self) -> Vec<Frame> {
        self.frames.lock().expect("frames lock").clone()
    }

    pub fn starts(&self) -> usize {
        self.state.lock().expect("state lock").starts
    }

    pub fn stops(&self) -> usize {
        self.state.lock().expect("state lock").stops
    }

    /// Sizes passed to `start`, in order: a resolution change adds an entry.
    pub fn sizes(&self) -> Vec<(u32, u32)> {
        self.state.lock().expect("state lock").sizes.clone()
    }
}

impl VirtualCameraSink for RecordingSink {
    fn start(&mut self, width: u32, height: u32, _fps: u32) -> Result<(), SinkError> {
        let mut state = self.state.lock().expect("state lock");
        state.running = true;
        state.starts += 1;
        state.sizes.push((width, height));
        Ok(())
    }

    fn send(&mut self, frame: &Frame) -> Result<(), SinkError> {
        if !self.state.lock().expect("state lock").running {
            return Err(SinkError::NotStarted);
        }
        self.frames.lock().expect("frames lock").push(frame.clone());
        Ok(())
    }

    fn stop(&mut self) {
        let mut state = self.state.lock().expect("state lock");
        state.running = false;
        state.stops += 1;
    }
}

#[cfg(windows)]
pub mod windows;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_sink_refuses_frames_until_started() {
        let mut sink = NullSink::default();
        let frame = Frame::new(2, 2);
        assert_eq!(sink.send(&frame), Err(SinkError::NotStarted));
        sink.start(2, 2, 30).expect("start");
        assert_eq!(sink.send(&frame), Ok(()));
        sink.stop();
        assert_eq!(sink.send(&frame), Err(SinkError::NotStarted));
    }

    #[test]
    fn recording_sink_keeps_the_frames_it_receives() {
        let mut sink = RecordingSink::default();
        let observer = sink.clone();
        sink.start(2, 2, 30).expect("start");
        sink.send(&Frame::filled(2, 2, (1, 2, 3))).expect("send");
        sink.send(&Frame::filled(2, 2, (4, 5, 6))).expect("send");
        sink.stop();
        assert_eq!(observer.starts(), 1);
        assert_eq!(observer.stops(), 1);
        assert_eq!(observer.sizes(), vec![(2, 2)]);
        let frames = observer.frames();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].pixel(0, 0), (1, 2, 3));
        assert_eq!(frames[1].pixel(0, 0), (4, 5, 6));
    }
}
