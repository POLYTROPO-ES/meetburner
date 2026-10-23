//! Linux virtual camera: publishes YUYV frames to a `v4l2loopback` device (ADR-0002).
//!
//! The app never loads kernel modules. It looks for an existing loopback device and, when
//! there is none, the engine keeps the `NullSink` and the setup page explains how to load
//! the module (see `native/vcam-linux/README.md`).

use std::path::{Path, PathBuf};

use crate::camera::frame::Frame;
use crate::camera::sink::VirtualCameraSink;

/// Environment override: `/dev/videoN` used when no labelled loopback device is present.
pub const DEVICE_ENV: &str = "MEETBURNER_V4L2_DEVICE";

/// Sysfs root that holds `video4linux/`; [`detect`] takes a root so tests can use a temp dir.
const SYSFS_ROOT: &str = "/sys/class";
/// Devnodes live here; the sysfs entry name (`video42`) is also the node name.
const DEV_ROOT: &str = "/dev";
const MEETBURNER_LABEL: &str = "MeetBurner";
/// v4l2loopback's default card label when the module is loaded without `card_label`.
const LOOPBACK_DEFAULT_LABEL: &str = "Dummy video device";

/// A v4l2loopback device discovered through sysfs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopbackDevice {
    /// Devnode the frames are written to (`/dev/videoN`).
    pub path: PathBuf,
    /// Card label from `<sysfs>/video4linux/videoN/name`.
    pub name: String,
}

/// Loopback devices below `root`, read from `video4linux/video*/name`.
///
/// Real webcams are left out: the only labels treated as a loopback are MeetBurner's and
/// v4l2loopback's default one, because writing YUYV to a capture-only camera cannot work.
pub fn loopback_devices(root: &Path) -> Vec<LoopbackDevice> {
    let entries = match std::fs::read_dir(root.join("video4linux")) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(), // the module is not loaded
    };
    let mut devices: Vec<LoopbackDevice> = entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("video"))
        .filter_map(|entry| {
            let name = std::fs::read_to_string(entry.path().join("name")).ok()?;
            let name = name.trim();
            if !name.contains(MEETBURNER_LABEL) && !name.contains(LOOPBACK_DEFAULT_LABEL) {
                return None;
            }
            Some(LoopbackDevice {
                path: Path::new(DEV_ROOT).join(entry.file_name()),
                name: name.to_string(),
            })
        })
        .collect();
    devices.sort_by(|left, right| left.path.cmp(&right.path));
    devices
}

/// Preference rule: the MeetBurner card, then v4l2loopback's default label, then the
/// `MEETBURNER_V4L2_DEVICE` override. `None` means the module is not loaded.
fn select(devices: &[LoopbackDevice], override_path: Option<&str>) -> Option<LoopbackDevice> {
    let labelled = |needle: &str| {
        devices
            .iter()
            .find(|device| device.name.contains(needle))
            .cloned()
    };
    labelled(MEETBURNER_LABEL)
        .or_else(|| labelled(LOOPBACK_DEFAULT_LABEL))
        .or_else(|| {
            override_path.map(|path| LoopbackDevice {
                path: PathBuf::from(path),
                name: String::new(), // outside sysfs: the card label is unknown
            })
        })
}

/// Device to publish to below `root`, or `None` when v4l2loopback is not loaded.
pub fn detect(root: &Path, override_path: Option<&str>) -> Option<LoopbackDevice> {
    select(&loopback_devices(root), override_path)
}

/// Thin wrapper over [`detect`] that reads the real `/sys/class` and [`DEVICE_ENV`].
pub fn device() -> Option<LoopbackDevice> {
    // A stale override must not claim that a camera exists.
    let override_path = std::env::var(DEVICE_ENV)
        .ok()
        .filter(|path| Path::new(path).exists());
    detect(Path::new(SYSFS_ROOT), override_path.as_deref())
}

/// The sink the render loop uses: the loopback device when there is one, otherwise frames
/// are discarded by `NullSink`, exactly like before M5.
pub(crate) fn sink_for(device: Option<LoopbackDevice>) -> Box<dyn VirtualCameraSink> {
    match device {
        #[cfg(target_os = "linux")]
        Some(device) => Box::new(V4l2Sink::new(device.path)),
        _ => Box::new(crate::camera::sink::NullSink::default()),
    }
}

/// Converts a BGRA frame into packed YUYV (`V4L2_PIX_FMT_YUYV`), two bytes per pixel.
///
/// Chroma comes from the first pixel of each pair (the counter frame has no fine chroma
/// detail worth averaging) and an odd-width row repeats its last pixel, because the format
/// only stores whole pairs. Every supported camera size is even, so the padding never
/// reaches a real device.
fn to_yuyv(frame: &Frame, out: &mut Vec<u8>) {
    let width = frame.width() as usize;
    let height = frame.height() as usize;
    let stride = width.div_ceil(2) * 2;
    out.clear();
    out.resize(stride * height * 2, 0);
    let pixels = frame.pixels();
    for y in 0..height {
        let row = &pixels[y * width * 4..(y + 1) * width * 4];
        let out_row = &mut out[y * stride * 2..(y + 1) * stride * 2];
        for pair in 0..stride / 2 {
            let first = pair * 2;
            let (y0, u, v) = yuv(&row[first * 4..first * 4 + 4]);
            let y1 = if first + 1 < width {
                yuv(&row[(first + 1) * 4..(first + 1) * 4 + 4]).0
            } else {
                y0
            };
            out_row[pair * 4..pair * 4 + 4].copy_from_slice(&[y0, u, y1, v]);
        }
    }
}

/// BT.601 limited-range YUV, integer-only.
fn yuv(bgra: &[u8]) -> (u8, u8, u8) {
    let b = i32::from(bgra[0]);
    let g = i32::from(bgra[1]);
    let r = i32::from(bgra[2]);
    let y = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
    let u = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
    let v = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
    (clamp(y), clamp(u), clamp(v))
}

fn clamp(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

/// Producer for a v4l2loopback device. Only the device I/O is Linux-only: the preference
/// rule and the pixel conversion above are tested on every host.
#[cfg(target_os = "linux")]
mod v4l2 {
    use std::ffi::CString;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use crate::camera::frame::Frame;
    use crate::camera::sink::{SinkError, VirtualCameraSink};

    use super::to_yuyv;

    /// `enum v4l2_buf_type`.
    const V4L2_BUF_TYPE_VIDEO_OUTPUT: u32 = 2;
    /// `v4l2_fourcc('Y', 'U', 'Y', 'V')`.
    const V4L2_PIX_FMT_YUYV: u32 = 0x5659_5559;
    /// `enum v4l2_field`: progressive frames.
    const V4L2_FIELD_NONE: u32 = 1;
    /// `enum v4l2_colorspace`: BT.601, which is what [`super::yuv`] computes.
    const V4L2_COLORSPACE_SMPTE170M: u32 = 1;
    /// `enum v4l2_quantization`: the converter emits limited-range (16-235) luma.
    const V4L2_QUANTIZATION_LIM_RANGE: u32 = 2;

    /// `struct v4l2_pix_format`.
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct PixFormat {
        width: u32,
        height: u32,
        pixelformat: u32,
        field: u32,
        bytesperline: u32,
        sizeimage: u32,
        colorspace: u32,
        priv_flags: u32,
        flags: u32,
        ycbcr_enc: u32,
        quantization: u32,
        xfer_func: u32,
    }

    /// `struct v4l2_format`. The kernel union starts at 8 bytes on 64-bit (it contains
    /// pointers) and at 4 on 32-bit; `_alignment` reproduces that offset and makes
    /// `size_of::<Format>()` the size the ioctl request number is built from.
    #[repr(C)]
    struct Format {
        kind: u32,
        _alignment: [usize; 0],
        pix: PixFormat,
        _reserved: [u8; 152],
    }

    /// `_IOWR(kind, nr, size)` from `asm-generic/ioctl.h`.
    const fn iowr(nr: u32, size: usize) -> libc::c_ulong {
        ((3 << 30) | ((size as u32) << 16) | ((b'V' as u32) << 8) | nr) as libc::c_ulong
    }

    /// `VIDIOC_S_FMT`.
    const VIDIOC_S_FMT: libc::c_ulong = iowr(5, std::mem::size_of::<Format>());

    // The request number is derived from the struct size, and the kernel copies that many
    // bytes: pin both the offset of the union and the resulting ioctl number.
    const _: () = assert!(std::mem::offset_of!(Format, pix) == std::mem::size_of::<usize>());
    const _: () = assert!(
        std::mem::size_of::<Format>()
            == if cfg!(target_pointer_width = "64") {
                208
            } else {
                204
            }
    );
    #[cfg(target_pointer_width = "64")]
    const _: () = assert!(VIDIOC_S_FMT == 0xC0D0_5605);
    #[cfg(target_pointer_width = "32")]
    const _: () = assert!(VIDIOC_S_FMT == 0xC0CC_5605);

    /// Writes YUYV frames to `/dev/videoN`. Holds no thread and no consumer state: the
    /// meeting app negotiates the same devnode as a camera while this end is the producer.
    pub struct V4l2Sink {
        path: PathBuf,
        /// `-1` while stopped; the render thread owns the sink, so no lock is needed.
        fd: libc::c_int,
        width: u32,
        height: u32,
        /// Reused between frames: a 1080p frame is 4 MB and this runs at 30 fps.
        buffer: Vec<u8>,
    }

    impl V4l2Sink {
        pub fn new(path: PathBuf) -> V4l2Sink {
            V4l2Sink {
                path,
                fd: -1,
                width: 0,
                height: 0,
                buffer: Vec::new(),
            }
        }

        fn open(&mut self) -> Result<(), SinkError> {
            let path = CString::new(self.path.as_os_str().as_bytes()).map_err(|_| {
                SinkError::Io(format!(
                    "{} is not a valid device path",
                    self.path.display()
                ))
            })?;
            // SAFETY: `path` is a NUL-terminated string that outlives the call.
            let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
            if fd < 0 {
                return Err(io_error("cannot open", &self.path));
            }
            self.fd = fd;
            Ok(())
        }

        /// `VIDIOC_S_FMT` with YUYV at the requested size: every consumer supports that
        /// format and the kernel needs no buffer queue for the producer side.
        fn set_format(&mut self, width: u32, height: u32) -> Result<(), SinkError> {
            let mut format = Format {
                kind: V4L2_BUF_TYPE_VIDEO_OUTPUT,
                _alignment: [],
                pix: PixFormat {
                    width,
                    height,
                    pixelformat: V4L2_PIX_FMT_YUYV,
                    field: V4L2_FIELD_NONE,
                    bytesperline: width * 2,
                    sizeimage: width * height * 2,
                    colorspace: V4L2_COLORSPACE_SMPTE170M,
                    quantization: V4L2_QUANTIZATION_LIM_RANGE,
                    ..PixFormat::default()
                },
                _reserved: [0; 152],
            };
            // SAFETY: `format` is a valid `v4l2_format` for the duration of the call.
            if unsafe { libc::ioctl(self.fd, VIDIOC_S_FMT, &mut format) } < 0 {
                return Err(io_error("cannot set YUYV on", &self.path));
            }
            self.width = width;
            self.height = height;
            Ok(())
        }

        fn write_frame(&self, bytes: &[u8]) -> Result<(), SinkError> {
            let mut offset = 0;
            while offset < bytes.len() {
                // SAFETY: the slice points at `bytes.len() - offset` readable bytes.
                let written = unsafe {
                    libc::write(
                        self.fd,
                        bytes[offset..].as_ptr().cast(),
                        bytes.len() - offset,
                    )
                };
                if written < 0 {
                    let error = io::Error::last_os_error();
                    if error.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(SinkError::Io(format!(
                        "cannot write a frame to {}: {error}",
                        self.path.display()
                    )));
                }
                if written == 0 {
                    return Err(SinkError::Io(format!(
                        "cannot write a frame to {}: the device accepted no data",
                        self.path.display()
                    )));
                }
                offset += written as usize;
            }
            Ok(())
        }
    }

    /// The kernel format carries no frame rate: v4l2loopback publishes at the pace frames
    /// are written, which is the render loop's `FPS` (30).
    impl VirtualCameraSink for V4l2Sink {
        fn start(&mut self, width: u32, height: u32, _fps: u32) -> Result<(), SinkError> {
            if self.fd >= 0 {
                return Ok(());
            }
            self.open()?;
            if let Err(error) = self.set_format(width, height) {
                self.stop();
                return Err(error);
            }
            Ok(())
        }

        fn send(&mut self, frame: &Frame) -> Result<(), SinkError> {
            if self.fd < 0 {
                return Err(SinkError::NotStarted);
            }
            // A live resolution change only re-negotiates the format (ADR-0015), matching
            // the way the Windows sink keeps its mapping.
            if (frame.width(), frame.height()) != (self.width, self.height) {
                self.set_format(frame.width(), frame.height())?;
            }
            to_yuyv(frame, &mut self.buffer);
            self.write_frame(&self.buffer)
        }

        fn stop(&mut self) {
            if self.fd >= 0 {
                // SAFETY: the descriptor was opened by `open` and is closed once.
                unsafe { libc::close(self.fd) };
                self.fd = -1;
            }
        }
    }

    impl Drop for V4l2Sink {
        fn drop(&mut self) {
            self.stop();
        }
    }

    fn io_error(what: &str, path: &Path) -> SinkError {
        SinkError::Io(format!(
            "{what} {}: {}",
            path.display(),
            io::Error::last_os_error()
        ))
    }
}

#[cfg(target_os = "linux")]
pub use v4l2::V4l2Sink;

#[cfg(test)]
mod tests {
    use super::*;

    /// Temporary `/sys/class` replacement, removed when the test ends.
    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new(tag: &str) -> TempRoot {
            let path =
                std::env::temp_dir().join(format!("meetburner-v4l2-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("video4linux")).expect("temp sysfs root");
            TempRoot { path }
        }

        fn video(&self, node: &str, name: &str) -> &TempRoot {
            let dir = self.path.join("video4linux").join(node);
            std::fs::create_dir_all(&dir).expect("device folder");
            std::fs::write(dir.join("name"), name).expect("card label");
            self
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn red() -> (u8, u8, u8) {
        (255, 0, 0)
    }

    fn green() -> (u8, u8, u8) {
        (0, 255, 0)
    }

    fn blue() -> (u8, u8, u8) {
        (0, 0, 255)
    }

    fn white() -> (u8, u8, u8) {
        (255, 255, 255)
    }

    #[test]
    fn bgra_becomes_yuyv_in_row_order() {
        // 2x2: red green / blue white. Each pair shares the first pixel's chroma.
        let mut frame = Frame::new(2, 2);
        frame.set_pixel(0, 0, red());
        frame.set_pixel(1, 0, green());
        frame.set_pixel(0, 1, blue());
        frame.set_pixel(1, 1, white());

        let mut out = Vec::new();
        to_yuyv(&frame, &mut out);

        assert_eq!(
            out,
            vec![
                82, 90, 144, 240, // row 0: Y(red) U(red) Y(green) V(red)
                41, 240, 235, 110, // row 1: Y(blue) U(blue) Y(white) V(blue)
            ]
        );
    }

    #[test]
    fn an_odd_width_repeats_the_last_pixel() {
        // Three pixels per row need two YUYV pairs: the last one repeats the third pixel.
        let mut frame = Frame::new(3, 1);
        frame.set_pixel(0, 0, red());
        frame.set_pixel(1, 0, green());
        frame.set_pixel(2, 0, blue());

        let mut out = Vec::new();
        to_yuyv(&frame, &mut out);

        assert_eq!(
            out,
            vec![
                82, 90, 144, 240, // red, green
                41, 240, 41, 110, // blue, blue again
            ]
        );
    }

    #[test]
    fn a_frame_becomes_two_bytes_per_pixel() {
        let mut out = Vec::new();
        to_yuyv(&Frame::new(4, 2), &mut out);
        assert_eq!(out.len(), 4 * 2 * 2);

        to_yuyv(&Frame::filled(1280, 720, (1, 2, 3)), &mut out);
        assert_eq!(out.len(), 1280 * 720 * 2, "a full camera frame");
    }

    #[test]
    fn loopback_devices_reports_labelled_devices_only() {
        let root = TempRoot::new("detect");
        root.video("video0", "Dummy video device (0x0000)\n")
            .video("video1", "Integrated Camera\n")
            .video("video42", "MeetBurner Camera\n");

        let devices = loopback_devices(&root.path);

        assert_eq!(devices.len(), 2, "the real webcam is not a loopback device");
        assert_eq!(devices[0].path, Path::new("/dev/video0"));
        assert_eq!(devices[0].name, "Dummy video device (0x0000)");
        assert_eq!(devices[1].path, Path::new("/dev/video42"));
        assert_eq!(devices[1].name, "MeetBurner Camera");
    }

    #[test]
    fn the_meetburner_card_wins_over_the_dummy_device() {
        let devices = vec![
            LoopbackDevice {
                path: PathBuf::from("/dev/video0"),
                name: "Dummy video device (0x0000)".into(),
            },
            LoopbackDevice {
                path: PathBuf::from("/dev/video42"),
                name: "MeetBurner Camera".into(),
            },
        ];

        let chosen = detect_with(&devices, Some("/dev/video7"));

        assert_eq!(chosen.expect("a device").path, Path::new("/dev/video42"));
    }

    #[test]
    fn the_dummy_device_is_the_second_choice() {
        let devices = vec![LoopbackDevice {
            path: PathBuf::from("/dev/video0"),
            name: "Dummy video device (0x0000)".into(),
        }];

        let chosen = detect_with(&devices, Some("/dev/video7"));

        assert_eq!(chosen.expect("a device").path, Path::new("/dev/video0"));
    }

    #[test]
    fn the_environment_override_is_the_last_resort() {
        let devices = vec![LoopbackDevice {
            path: PathBuf::from("/dev/video42"),
            name: "MeetBurner Camera".into(),
        }];

        let chosen = detect_with(&devices, Some("/dev/video7"));

        assert_eq!(
            chosen.expect("a device").path,
            Path::new("/dev/video42"),
            "a recognised device beats the override"
        );

        let custom = detect_with(&[], Some("/dev/video7"));

        // Zero-device fallback: a sink that starts without hardware and discards frames.
        let mut sink = sink_for(None);
        sink.start(2, 2, 30).expect("the null sink starts");
        sink.send(&Frame::new(2, 2))
            .expect("the frame is discarded");
        sink.stop();
        assert_eq!(custom.expect("the override").path, Path::new("/dev/video7"));
        assert_eq!(detect_with(&[], None), None, "no override, no device");
    }

    #[test]
    fn no_module_means_no_device_so_the_null_sink_stays() {
        let root = TempRoot::new("no-module");

        assert_eq!(detect(&root.path, None), None);
        assert!(loopback_devices(&root.path).is_empty());
    }

    #[test]
    fn detection_reads_the_sysfs_tree_and_falls_back_to_the_override() {
        let root = TempRoot::new("preference");
        root.video("video0", "Dummy video device (0x0000)\n")
            .video("video42", "MeetBurner Camera\n");

        let chosen = detect(&root.path, Some("/dev/video7"));
        assert_eq!(
            chosen.expect("a device").path,
            Path::new("/dev/video42"),
            "the MeetBurner label wins"
        );

        let empty = TempRoot::new("preference-empty");
        let custom = detect(&empty.path, Some("/dev/video7"));
        assert_eq!(custom.expect("the override").path, Path::new("/dev/video7"));
    }

    /// Calls the preference rule directly, so tests never need the real sysfs or environment.
    fn detect_with(
        devices: &[LoopbackDevice],
        override_path: Option<&str>,
    ) -> Option<LoopbackDevice> {
        super::select(devices, override_path)
    }
}
