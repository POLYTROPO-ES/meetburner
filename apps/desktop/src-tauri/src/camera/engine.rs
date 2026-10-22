//! Render loop and the thread that drives it (30 fps, never on the Tauri main thread).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::camera::fonts::Fonts;
use crate::camera::frame::{FPS, FRAME_HEIGHT, FRAME_WIDTH};
use crate::camera::renderer::{render, CounterInput};
use crate::camera::sink::{SinkError, VirtualCameraSink};

/// One renderer tick: draws a frame and hands it to the sink.
pub struct RenderLoop {
    fonts: Fonts,
    sink: Box<dyn VirtualCameraSink>,
}

impl RenderLoop {
    pub fn new(
        fonts: Fonts,
        mut sink: Box<dyn VirtualCameraSink>,
    ) -> Result<RenderLoop, SinkError> {
        sink.start(FRAME_WIDTH, FRAME_HEIGHT, FPS)?;
        Ok(RenderLoop { fonts, sink })
    }

    pub fn tick(&mut self, input: &CounterInput, now_ms: i64) -> Result<(), SinkError> {
        let frame = render(input, &self.fonts, now_ms, None);
        self.sink.send(&frame)
    }

    pub fn stop(&mut self) {
        self.sink.stop();
    }
}

/// Owns the render thread. `camera_start` creates it, `camera_update` swaps the input.
pub struct CameraEngine {
    input: Arc<RwLock<CounterInput>>,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl CameraEngine {
    pub fn start(input: CounterInput) -> Result<CameraEngine, SinkError> {
        let fonts = Fonts::load_system().map_err(|error| SinkError::Io(error.to_string()))?;
        let input = Arc::new(RwLock::new(input));
        let running = Arc::new(AtomicBool::new(true));
        let thread_input = Arc::clone(&input);
        let thread_running = Arc::clone(&running);
        let thread = std::thread::Builder::new()
            .name("meetburner-camera".into())
            .spawn(move || {
                let mut render_loop = match RenderLoop::new(fonts, create_sink()) {
                    Ok(render_loop) => render_loop,
                    Err(error) => {
                        eprintln!("meetburner: cannot start the virtual camera: {error}");
                        return;
                    }
                };
                let period = Duration::from_millis(1000 / FPS as u64);
                while thread_running.load(Ordering::Relaxed) {
                    let started = Instant::now();
                    let current = thread_input.read().expect("camera input lock").clone();
                    if let Err(error) = render_loop.tick(&current, now_ms()) {
                        eprintln!("meetburner: camera frame dropped: {error}");
                    }
                    std::thread::sleep(period.saturating_sub(started.elapsed()));
                }
                render_loop.stop();
            })
            .map_err(|error| SinkError::Io(error.to_string()))?;
        Ok(CameraEngine {
            input,
            running,
            thread: Some(thread),
        })
    }

    pub fn update(&self, input: CounterInput) {
        *self.input.write().expect("camera input lock") = input;
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for CameraEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(windows)]
fn create_sink() -> Box<dyn VirtualCameraSink> {
    Box::new(crate::camera::sink::windows::WinSharedMemSink::new())
}

#[cfg(not(windows))]
fn create_sink() -> Box<dyn VirtualCameraSink> {
    Box::new(crate::camera::sink::NullSink::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::renderer::{DisplayConfig, Labels};
    use crate::camera::sink::RecordingSink;

    fn input() -> CounterInput {
        CounterInput {
            start_at: 1_000_000,
            end_at: None,
            rate_per_ms: 0.01,
            currency: "EUR".into(),
            locale: "es".into(),
            labels: Labels::default(),
            display: DisplayConfig::default(),
        }
    }

    #[test]
    fn every_tick_sends_one_frame_to_the_sink() {
        let sink = RecordingSink::default();
        let observer = sink.clone();
        let mut render_loop = RenderLoop::new(Fonts::load_system().expect("fonts"), Box::new(sink))
            .expect("render loop");
        render_loop.tick(&input(), 1_001_000).expect("tick");
        render_loop.tick(&input(), 1_002_000).expect("tick");
        render_loop.stop();
        assert_eq!(observer.starts(), 1);
        assert_eq!(observer.stops(), 1);
        assert_eq!(observer.frames().len(), 2);
        assert_eq!(observer.frames()[0].width(), FRAME_WIDTH);
    }

    #[test]
    fn ticks_use_the_current_input() {
        let sink = RecordingSink::default();
        let observer = sink.clone();
        let mut render_loop = RenderLoop::new(Fonts::load_system().expect("fonts"), Box::new(sink))
            .expect("render loop");
        render_loop.tick(&input(), 1_001_000).expect("tick");
        let mut halloween = input();
        halloween.display.theme = "halloween".into();
        render_loop.tick(&halloween, 1_001_000).expect("tick");
        let frames = observer.frames();
        assert_eq!(frames[0].pixel(0, 0), (0x0f, 0x11, 0x15));
        assert_eq!(frames[1].pixel(0, 0), (0x1a, 0x0f, 0x05));
    }
}
