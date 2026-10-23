//! Render loop and the thread that drives it (30 fps, never on the Tauri main thread).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::camera::fonts::Fonts;
use crate::camera::frame::FPS;
use crate::camera::renderer::{render, CounterInput};
use crate::camera::sink::{SinkError, VirtualCameraSink};
use crate::camera::source::{FrameSource, NoSource, WebcamSource};
use crate::camera::theme::Layout;

/// One renderer tick: draws a frame and hands it to the sink.
pub struct RenderLoop {
    fonts: Fonts,
    sink: Box<dyn VirtualCameraSink>,
    source: Box<dyn FrameSource>,
    size: Option<(u32, u32)>,
    /// Webcam the current `source` was opened for, and the size it was asked to fill.
    source_key: Option<(String, (u32, u32))>,
}

impl RenderLoop {
    pub fn new(fonts: Fonts, sink: Box<dyn VirtualCameraSink>) -> RenderLoop {
        RenderLoop {
            fonts,
            sink,
            source: Box::new(NoSource),
            size: None,
            source_key: None,
        }
    }

    /// Applies the latest input for one frame. The mapping is created on the first tick and
    /// kept: a resolution change only changes the size in the frame header, so consumers
    /// keep reading the same device (ADR-0019).
    pub fn tick(&mut self, input: &CounterInput, now_ms: i64) -> Result<(), SinkError> {
        let size = input.size();
        if self.size.is_none() {
            self.sink.start(size.0, size.1, FPS)?;
        }
        self.size = Some(size);
        self.reconcile_source(input, size);
        // Only the widget card has a readable panel: overlaying the fullscreen layout would
        // paint the counter straight over the video (ADR-0021).
        let background = if input.display.layout == Layout::Widget {
            self.source.latest()
        } else {
            None
        };
        let frame = render(input, &self.fonts, now_ms, background.as_ref());
        self.sink.send(&frame)
    }

    /// Opens, re-opens or closes the webcam capture when the payload asks for a different
    /// source or a different output size.
    fn reconcile_source(&mut self, input: &CounterInput, size: (u32, u32)) {
        let wanted = input.source.clone().map(|id| (id, size));
        if self.source_key == wanted {
            return;
        }
        self.source.stop();
        self.source = match wanted.as_ref() {
            Some((id, (width, height))) => match WebcamSource::start(id, *width, *height) {
                Ok(source) => Box::new(source),
                Err(error) => {
                    eprintln!("meetburner: {error}");
                    Box::new(NoSource)
                }
            },
            None => Box::new(NoSource),
        };
        self.source_key = wanted;
    }

    pub fn stop(&mut self) {
        self.source.stop();
        self.source_key = None;
        self.sink.stop();
        self.size = None;
    }

    /// Test seam: injects a frame source without opening a real camera.
    #[cfg(test)]
    pub fn set_source(&mut self, source: Box<dyn FrameSource>) {
        self.source = source;
        self.source_key = None;
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
                let mut render_loop = RenderLoop::new(fonts, create_sink());
                let period = Duration::from_millis(1000 / FPS as u64);
                let mut last_error: Option<String> = None;
                while thread_running.load(Ordering::Relaxed) {
                    let started = Instant::now();
                    let current = thread_input.read().expect("camera input lock").clone();
                    if let Err(error) = render_loop.tick(&current, now_ms()) {
                        // Only log when the message changes: a broken sink would flood the log.
                        let message = error.to_string();
                        if last_error.as_deref() != Some(message.as_str()) {
                            eprintln!("meetburner: camera frame dropped: {message}");
                            last_error = Some(message);
                        }
                    } else {
                        last_error = None;
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

/// Linux publishes to an existing v4l2loopback device. The app never loads kernel modules:
/// without a device the frames are discarded and the setup page explains how to load it.
#[cfg(target_os = "linux")]
fn create_sink() -> Box<dyn VirtualCameraSink> {
    use crate::camera::sink::linux;

    let device = linux::device();
    match &device {
        Some(device) => eprintln!(
            "meetburner: publishing the camera to {}",
            device.path.display()
        ),
        None => eprintln!(
            "meetburner: no v4l2loopback device; load it with: \
             sudo modprobe v4l2loopback exclusive_caps=1 card_label=\"MeetBurner Camera\""
        ),
    }
    linux::sink_for(device)
}

#[cfg(not(any(windows, target_os = "linux")))]
fn create_sink() -> Box<dyn VirtualCameraSink> {
    Box::new(crate::camera::sink::NullSink::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::frame::{Frame, FRAME_HEIGHT, FRAME_WIDTH};
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
            width: FRAME_WIDTH,
            height: FRAME_HEIGHT,
            source: None,
        }
    }

    #[test]
    fn every_tick_sends_one_frame_to_the_sink() {
        let sink = RecordingSink::default();
        let observer = sink.clone();
        let mut render_loop = RenderLoop::new(Fonts::load_system().expect("fonts"), Box::new(sink));
        render_loop.tick(&input(), 1_001_000).expect("tick");
        render_loop.tick(&input(), 1_002_000).expect("tick");
        render_loop.stop();
        assert_eq!(observer.starts(), 1);
        assert_eq!(observer.stops(), 1);
        assert_eq!(observer.sizes(), vec![(FRAME_WIDTH, FRAME_HEIGHT)]);
        assert_eq!(observer.frames().len(), 2);
        assert_eq!(observer.frames()[0].width(), FRAME_WIDTH);
    }

    #[test]
    fn ticks_use_the_current_input() {
        let sink = RecordingSink::default();
        let observer = sink.clone();
        let mut render_loop = RenderLoop::new(Fonts::load_system().expect("fonts"), Box::new(sink));
        render_loop.tick(&input(), 1_001_000).expect("tick");
        let mut halloween = input();
        halloween.display.theme = "halloween".into();
        render_loop.tick(&halloween, 1_001_000).expect("tick");
        let frames = observer.frames();
        assert_eq!(frames[0].pixel(0, 0), (0x0f, 0x11, 0x15));
        assert_eq!(frames[1].pixel(0, 0), (0x1a, 0x0f, 0x05));
    }

    #[test]
    fn a_resolution_change_keeps_the_same_sink_and_device() {
        let sink = RecordingSink::default();
        let observer = sink.clone();
        let mut render_loop = RenderLoop::new(Fonts::load_system().expect("fonts"), Box::new(sink));
        render_loop.tick(&input(), 1_001_000).expect("tick");

        let mut small = input();
        small.width = 640;
        small.height = 360;
        render_loop
            .tick(&small, 1_002_000)
            .expect("tick after resize");

        assert_eq!(observer.starts(), 1, "the mapping is created once");
        assert_eq!(observer.stops(), 0, "the device is never torn down");
        assert_eq!(observer.sizes(), vec![(1280, 720)]);
        let frames = observer.frames();
        assert_eq!((frames[1].width(), frames[1].height()), (640, 360));

        render_loop.stop();
        assert_eq!(observer.stops(), 1);
    }

    #[test]
    fn the_same_resolution_does_not_restart_the_sink() {
        let sink = RecordingSink::default();
        let observer = sink.clone();
        let mut render_loop = RenderLoop::new(Fonts::load_system().expect("fonts"), Box::new(sink));
        render_loop.tick(&input(), 1_001_000).expect("tick");
        render_loop.tick(&input(), 1_002_000).expect("tick");
        assert_eq!(observer.starts(), 1);
        render_loop.stop();
    }

    /// Stands in for a webcam so the overlay is testable without hardware.
    struct FakeSource {
        frame: Frame,
    }

    impl FrameSource for FakeSource {
        fn latest(&self) -> Option<Frame> {
            Some(self.frame.clone())
        }

        fn stop(&mut self) {}
    }

    fn overlay_source() -> Box<dyn FrameSource> {
        Box::new(FakeSource {
            frame: Frame::filled(FRAME_WIDTH, FRAME_HEIGHT, (0, 200, 0)),
        })
    }

    #[test]
    fn the_widget_layout_composites_the_webcam_behind_the_card() {
        let sink = RecordingSink::default();
        let observer = sink.clone();
        let mut render_loop = RenderLoop::new(Fonts::load_system().expect("fonts"), Box::new(sink));
        render_loop.set_source(overlay_source());
        let mut widget = input();
        widget.display.layout = Layout::Widget;
        render_loop.tick(&widget, 1_001_000).expect("tick");
        let frame = observer.frames().last().cloned().expect("a frame");
        assert_eq!(frame.pixel(0, 0), (0, 200, 0), "the webcam fills the frame");
    }

    #[test]
    fn the_fullscreen_layout_ignores_the_webcam() {
        let sink = RecordingSink::default();
        let observer = sink.clone();
        let mut render_loop = RenderLoop::new(Fonts::load_system().expect("fonts"), Box::new(sink));
        render_loop.set_source(overlay_source());
        render_loop.tick(&input(), 1_001_000).expect("tick");
        let frame = observer.frames().last().cloned().expect("a frame");
        assert_eq!(frame.pixel(0, 0), (0x0f, 0x11, 0x15), "counter only");
    }

    #[test]
    fn a_source_that_cannot_be_opened_falls_back_to_the_counter() {
        let sink = RecordingSink::default();
        let observer = sink.clone();
        let mut render_loop = RenderLoop::new(Fonts::load_system().expect("fonts"), Box::new(sink));
        let mut widget = input();
        widget.display.layout = Layout::Widget;
        widget.source = Some("not-a-camera".into());
        render_loop
            .tick(&widget, 1_001_000)
            .expect("the counter still renders");
        let frame = observer.frames().last().cloned().expect("a frame");
        assert_eq!(
            frame.pixel(0, 0),
            (0, 0, 0),
            "no webcam frame: the black canvas shows"
        );
        assert_eq!(
            frame.pixel(750, 480),
            (0x0f, 0x11, 0x15),
            "the card is still painted"
        );
    }
}
