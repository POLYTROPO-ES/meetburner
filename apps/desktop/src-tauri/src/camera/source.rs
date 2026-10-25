//! Physical webcam capture for the overlay mode (M7, ADR-0021).
//!
//! Capture runs on its own thread and only keeps the newest frame; the render thread reads
//! it and composites the counter card on top. A slow, busy or unplugged camera therefore
//! never stalls the virtual camera: the overlay just disappears and the counter keeps going.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::Serialize;

use crate::camera::frame::{preview_size, Frame};

/// Pause before retrying when the camera returns no frame (it blocks on its own otherwise).
const RETRY_PAUSE: Duration = Duration::from_millis(5);

/// What the capture thread last produced, shared with the setup preview (ADR-0035): the newest
/// frame, or the reason there is none.
#[derive(Debug, Default)]
pub struct Capture {
    pub frame: Option<Frame>,
    pub error: Option<String>,
}

pub type SharedCapture = Arc<Mutex<Capture>>;

/// The newest frame shrunk for the setup preview; `Ok(None)` while the camera warms up.
pub fn preview(capture: &SharedCapture, longest: u32) -> Result<Option<Frame>, String> {
    let capture = capture
        .lock()
        .map_err(|_| "webcam frame lock poisoned".to_string())?;
    match (&capture.frame, &capture.error) {
        (Some(frame), _) => {
            let (width, height) = preview_size(frame.width(), frame.height(), longest);
            Ok(Some(frame.scaled_to(width, height)))
        }
        (None, Some(error)) => Err(error.clone()),
        (None, None) => Ok(None),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraSource {
    /// Opaque id the frontend sends back in the camera payload.
    pub id: String,
    pub name: String,
}

/// A provider of webcam frames, behind a trait so the render loop is testable without one.
pub trait FrameSource: Send {
    fn latest(&self) -> Option<Frame>;
    fn stop(&mut self);
}

/// No webcam: the counter fills the frame.
#[derive(Debug, Default)]
pub struct NoSource;

impl FrameSource for NoSource {
    fn latest(&self) -> Option<Frame> {
        None
    }

    fn stop(&mut self) {}
}

/// Lists the physical cameras of the system. "MeetBurner Camera" is not in the list on
/// purpose: it is a DirectShow filter and this enumerates Media Foundation devices, so the
/// overlay can never feed itself.
pub fn list_sources() -> Vec<CameraSource> {
    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    {
        nokhwa::query(nokhwa::utils::ApiBackend::Auto)
            .map(|infos| infos.iter().filter_map(source_of).collect())
            .unwrap_or_default()
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        Vec::new()
    }
}

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
fn source_of(info: &nokhwa::utils::CameraInfo) -> Option<CameraSource> {
    let index = info.index().as_index().ok()?;
    Some(CameraSource {
        id: index.to_string(),
        name: info.human_name(),
    })
}

/// Latest frame of a physical camera, captured on its own thread.
pub struct WebcamSource {
    capture: SharedCapture,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl WebcamSource {
    /// Starts capturing `id` (a [`CameraSource::id`]) into `capture`, scaling every frame to
    /// `width` × `height`. Failures land in `capture.error` rather than failing the camera:
    /// the counter keeps being published on its own.
    pub fn start(
        id: &str,
        width: u32,
        height: u32,
        capture: SharedCapture,
    ) -> Result<WebcamSource, String> {
        let index: u32 = id
            .parse()
            .map_err(|_| format!("'{id}' is not a camera id"))?;
        *capture.lock().expect("webcam frame lock") = Capture::default();
        let running = Arc::new(AtomicBool::new(true));
        let thread_capture = Arc::clone(&capture);
        let thread_running = Arc::clone(&running);
        let thread = std::thread::Builder::new()
            .name("meetburner-webcam".into())
            .spawn(move || capture_loop(index, width, height, thread_capture, thread_running))
            .map_err(|error| format!("cannot start the webcam thread: {error}"))?;
        Ok(WebcamSource {
            capture,
            running,
            thread: Some(thread),
        })
    }
}

impl FrameSource for WebcamSource {
    fn latest(&self) -> Option<Frame> {
        self.capture
            .lock()
            .expect("webcam frame lock")
            .frame
            .clone()
    }

    fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        *self.capture.lock().expect("webcam frame lock") = Capture::default();
    }
}

impl Drop for WebcamSource {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
fn capture_loop(
    index: u32,
    width: u32,
    height: u32,
    capture: SharedCapture,
    running: Arc<AtomicBool>,
) {
    use nokhwa::pixel_format::RgbAFormat;
    use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};

    // Opening is where a webcam held by another app fails; the preview shows the reason.
    let fail = |message: String| {
        eprintln!("meetburner: {message}");
        capture.lock().expect("webcam frame lock").error = Some(message);
    };
    let format = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    let mut camera = match nokhwa::Camera::new(CameraIndex::Index(index), format) {
        Ok(camera) => camera,
        Err(error) => {
            fail(format!("cannot open camera {index}: {error}"));
            return;
        }
    };
    if let Err(error) = camera.open_stream() {
        fail(format!("cannot start camera {index}: {error}"));
        return;
    }
    let mut last_error: Option<String> = None;
    while running.load(Ordering::Relaxed) {
        match camera.frame() {
            Ok(buffer) => match buffer.decode_image::<RgbAFormat>() {
                Ok(image) => {
                    if let Some(frame) =
                        Frame::from_rgba(image.width(), image.height(), image.as_raw())
                    {
                        let mut shared = capture.lock().expect("webcam frame lock");
                        shared.frame = Some(frame.scaled_to(width, height));
                        shared.error = None;
                    }
                    last_error = None;
                }
                Err(error) => report(&mut last_error, &format!("cannot decode: {error}")),
            },
            Err(error) => {
                report(&mut last_error, &format!("camera {index}: {error}"));
                std::thread::sleep(RETRY_PAUSE);
            }
        }
    }
    let _ = camera.stop_stream();
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn capture_loop(_: u32, _: u32, _: u32, _: SharedCapture, _: Arc<AtomicBool>) {}

/// Logs a capture error once: a camera that stays broken would otherwise flood the log.
fn report(last: &mut Option<String>, message: &str) {
    if last.as_deref() != Some(message) {
        eprintln!("meetburner: webcam {message}");
        *last = Some(message.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_source_never_provides_a_frame() {
        let mut source = NoSource;
        assert!(source.latest().is_none());
        source.stop();
    }

    #[test]
    fn an_unknown_device_id_is_rejected_without_starting_a_thread() {
        assert!(WebcamSource::start("not-a-camera", 640, 360, SharedCapture::default()).is_err());
    }

    #[test]
    fn a_camera_that_cannot_be_opened_degrades_to_no_frames_and_says_why() {
        // Reaches the backend, which fails for a missing index; the thread just exits.
        let capture = SharedCapture::default();
        let started = WebcamSource::start("9999", 640, 360, Arc::clone(&capture));
        if let Ok(mut source) = started {
            let mut waited = Duration::ZERO;
            while capture.lock().unwrap().error.is_none() && waited < Duration::from_secs(3) {
                std::thread::sleep(Duration::from_millis(50));
                waited += Duration::from_millis(50);
            }
            assert!(source.latest().is_none());
            assert!(
                capture.lock().unwrap().error.is_some(),
                "the preview can tell the user why"
            );
            source.stop();
            assert!(
                capture.lock().unwrap().error.is_none(),
                "stopping clears it"
            );
        }
    }

    #[test]
    fn the_preview_scales_the_newest_frame_down() {
        let capture = SharedCapture::default();
        capture.lock().unwrap().frame = Some(Frame::filled(1280, 720, (1, 2, 3)));
        let frame = preview(&capture, 480).expect("no error").expect("a frame");
        assert_eq!((frame.width(), frame.height()), (480, 270));
        assert_eq!(frame.pixel(0, 0), (1, 2, 3));
    }

    #[test]
    fn the_preview_reports_why_there_is_no_frame() {
        let capture = SharedCapture::default();
        assert_eq!(preview(&capture, 480), Ok(None), "still warming up");
        capture.lock().unwrap().error = Some("device in use".into());
        assert_eq!(preview(&capture, 480), Err("device in use".to_string()));
    }
}
