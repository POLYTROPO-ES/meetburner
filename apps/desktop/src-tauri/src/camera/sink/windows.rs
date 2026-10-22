//! Windows virtual-camera producer: one named shared-memory mapping that the
//! DirectShow / Media Foundation DLLs read (ADR-0002, ADR-0013).

use std::ffi::c_void;
use std::slice;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_ALL_ACCESS, PAGE_READWRITE,
};
use windows_sys::Win32::System::Threading::{CreateEventW, SetEvent};

use windows_sys::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS;

use crate::camera::engine::now_ms;
use crate::camera::frame::Frame;
use crate::camera::ipc::{self, CAPACITY_LEN, STATE_LIVE};
use crate::camera::sink::{SinkError, VirtualCameraSink};

/// Names must match `native/vcam-windows/common/meetburner_camera.h`.
pub const MAPPING_NAME: &str = "Local\\MeetBurnerCamera";
pub const EVENT_NAME: &str = "Local\\MeetBurnerCameraFrame";

/// The app is the only producer; meeting apps read the mapping through the DLL.
/// The mapping is created once with room for the largest supported size (ADR-0019).
pub struct WinSharedMemSink {
    mapping_name: String,
    event_name: String,
    mapping: HANDLE,
    view: *mut u8,
    event: HANDLE,
    len: usize,
    fps: u32,
}

// The handles are only touched by the render thread that owns the sink.
unsafe impl Send for WinSharedMemSink {}

impl WinSharedMemSink {
    pub fn new() -> WinSharedMemSink {
        WinSharedMemSink::with_names(MAPPING_NAME, EVENT_NAME)
    }

    /// Test seam: lets each test use its own mapping name.
    pub fn with_names(mapping_name: &str, event_name: &str) -> WinSharedMemSink {
        WinSharedMemSink {
            mapping_name: mapping_name.to_string(),
            event_name: event_name.to_string(),
            mapping: std::ptr::null_mut(),
            view: std::ptr::null_mut(),
            event: std::ptr::null_mut(),
            len: 0,
            fps: 0,
        }
    }

    pub fn is_running(&self) -> bool {
        !self.view.is_null()
    }

    fn write_idle(&self) {
        if self.view.is_null() {
            return;
        }
        // SAFETY: the view is mapped for `self.len` bytes until `stop`.
        let buf = unsafe { slice::from_raw_parts_mut(self.view, self.len) };
        ipc::mark_idle(buf);
    }
}

impl Default for WinSharedMemSink {
    fn default() -> Self {
        WinSharedMemSink::new()
    }
}

impl VirtualCameraSink for WinSharedMemSink {
    /// The requested size is informational: the mapping always has room for the largest
    /// supported frame, and every frame carries its own size in the header.
    fn start(&mut self, _width: u32, _height: u32, fps: u32) -> Result<(), SinkError> {
        if self.is_running() {
            return Ok(());
        }
        let len = CAPACITY_LEN;
        let mapping_name = wide(&self.mapping_name);
        let mapping = unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                std::ptr::null(),
                PAGE_READWRITE,
                0,
                len as u32,
                mapping_name.as_ptr(),
            )
        };
        if mapping.is_null() {
            return Err(win_error("CreateFileMappingW"));
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe { CloseHandle(mapping) };
            return Err(SinkError::Io(
                "another MeetBurner camera is already running".into(),
            ));
        }
        let view = unsafe { MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, len) };
        if view.Value.is_null() {
            unsafe { CloseHandle(mapping) };
            return Err(win_error("MapViewOfFile"));
        }
        let event_name = wide(&self.event_name);
        let event = unsafe { CreateEventW(std::ptr::null(), 0, 0, event_name.as_ptr()) };
        if event.is_null() {
            unsafe {
                UnmapViewOfFile(view);
                CloseHandle(mapping);
            }
            return Err(win_error("CreateEventW"));
        }
        self.mapping = mapping;
        self.view = view.Value as *mut u8;
        self.event = event;
        self.len = len;
        self.fps = fps;
        self.write_idle();
        Ok(())
    }

    fn send(&mut self, frame: &Frame) -> Result<(), SinkError> {
        if !self.is_running() {
            return Err(SinkError::NotStarted);
        }
        // SAFETY: the view is mapped for `self.len` bytes until `stop`.
        let buf = unsafe { slice::from_raw_parts_mut(self.view, self.len) };
        ipc::write_frame(buf, frame, now_ms(), self.fps, STATE_LIVE)?;
        unsafe { SetEvent(self.event) };
        Ok(())
    }

    fn stop(&mut self) {
        if !self.is_running() {
            return;
        }
        self.write_idle();
        unsafe {
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.view as *mut c_void,
            });
            CloseHandle(self.mapping);
            CloseHandle(self.event);
        }
        self.view = std::ptr::null_mut();
        self.mapping = std::ptr::null_mut();
        self.event = std::ptr::null_mut();
        self.len = 0;
    }
}

impl Drop for WinSharedMemSink {
    fn drop(&mut self) {
        self.stop();
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn win_error(what: &str) -> SinkError {
    SinkError::Io(format!("{what} failed (Win32 error {})", unsafe {
        GetLastError()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Memory::{OpenFileMappingW, FILE_MAP_READ};

    use crate::camera::ipc::{mapping_len, ReadFrame};

    fn unique_names(tag: &str) -> (String, String) {
        let id = std::process::id();
        (
            format!("Local\\MeetBurnerTest-{tag}-{id}"),
            format!("Local\\MeetBurnerTestEvent-{tag}-{id}"),
        )
    }

    struct TestReader {
        mapping: HANDLE,
        view: *mut u8,
        len: usize,
    }

    impl TestReader {
        fn open(name: &str, len: usize) -> TestReader {
            let name = wide(name);
            let mapping = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, name.as_ptr()) };
            assert!(!mapping.is_null(), "the producer mapping is open");
            let view = unsafe { MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, len) };
            assert!(!view.Value.is_null(), "the reader can map the view");
            TestReader {
                mapping,
                view: view.Value as *mut u8,
                len,
            }
        }

        fn read(&self) -> Option<ReadFrame> {
            let buf = unsafe { slice::from_raw_parts(self.view, self.len) };
            ipc::read_frame(buf)
        }
    }

    impl Drop for TestReader {
        fn drop(&mut self) {
            unsafe {
                UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.view as *mut c_void,
                });
                CloseHandle(self.mapping);
            }
        }
    }

    #[test]
    fn sends_frames_to_a_named_shared_memory_reader() {
        let (mapping, event) = unique_names("roundtrip");
        let mut sink = WinSharedMemSink::with_names(&mapping, &event);
        sink.start(4, 2, 30).expect("start");
        let reader = TestReader::open(&mapping, mapping_len(4, 2));

        let frame = Frame::filled(4, 2, (10, 20, 30));
        sink.send(&frame).expect("first frame");
        let first = reader.read().expect("a frame is available");
        assert_eq!(first.header.width, 4);
        assert_eq!(first.header.state, STATE_LIVE);
        assert_eq!(first.header.frame_counter, 1);
        assert_eq!(first.frame, frame);

        sink.send(&frame).expect("second frame");
        assert_eq!(reader.read().expect("frame 2").header.frame_counter, 2);
        sink.stop();
    }

    #[test]
    fn refuses_a_second_producer_on_the_same_mapping() {
        let (mapping, event) = unique_names("exclusive");
        let mut first = WinSharedMemSink::with_names(&mapping, &event);
        first.start(2, 2, 30).expect("first producer");
        let mut second = WinSharedMemSink::with_names(&mapping, &event);
        assert!(second.start(2, 2, 30).is_err());
        first.stop();
        second.start(2, 2, 30).expect("the name is free again");
        second.stop();
    }

    #[test]
    fn a_new_size_is_sent_without_reopening_the_mapping() {
        let (mapping, event) = unique_names("size");
        let mut sink = WinSharedMemSink::with_names(&mapping, &event);
        sink.start(2, 2, 30).expect("start");
        // The reader keeps this one view for the whole session, like the DLL does.
        let reader = TestReader::open(&mapping, CAPACITY_LEN);

        sink.send(&Frame::filled(2, 2, (1, 2, 3)))
            .expect("first size");
        let small = reader.read().expect("the small frame");
        assert_eq!((small.header.width, small.header.height), (2, 2));

        sink.send(&Frame::filled(4, 4, (4, 5, 6)))
            .expect("new size");
        let big = reader.read().expect("the resized frame is readable");
        assert_eq!((big.header.width, big.header.height), (4, 4));
        assert_eq!(big.frame, Frame::filled(4, 4, (4, 5, 6)));
        sink.stop();
    }

    #[test]
    fn send_before_start_is_an_error() {
        let (mapping, event) = unique_names("not-started");
        let mut sink = WinSharedMemSink::with_names(&mapping, &event);
        assert_eq!(
            sink.send(&Frame::filled(2, 2, (1, 1, 1))),
            Err(SinkError::NotStarted)
        );
    }
}
