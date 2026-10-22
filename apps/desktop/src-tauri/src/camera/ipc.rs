//! Frame protocol shared with the Windows virtual-camera DLLs (ADR-0013).
//!
//! Layout of the named shared memory:
//! `FrameHeader` (48 bytes) followed by `width * height` pixels of BGRA.
//! A seqlock (`sequence` odd while writing) lets consumers read a frame
//! without ever seeing a torn copy, and without blocking the producer.

use std::sync::atomic::{fence, Ordering};

use crate::camera::frame::Frame;
use crate::camera::sink::SinkError;

pub const MAGIC: u32 = 0x4D42_434D; // "MBCM"
pub const VERSION: u32 = 1;
pub const STATE_IDLE: u32 = 0;
pub const STATE_LIVE: u32 = 1;
const READ_ATTEMPTS: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameHeader {
    pub magic: u32,
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    /// `STATE_IDLE` or `STATE_LIVE`.
    pub state: u32,
    pub frame_counter: u64,
    pub timestamp_ms: u64,
    /// Seqlock: odd while the producer is copying pixels.
    pub sequence: u32,
    pub reserved: u32,
}

pub const HEADER_SIZE: usize = std::mem::size_of::<FrameHeader>();

pub const fn mapping_len(width: u32, height: u32) -> usize {
    HEADER_SIZE + width as usize * height as usize * 4
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReadFrame {
    pub header: FrameHeader,
    pub frame: Frame,
}

fn read_header(buf: &[u8]) -> FrameHeader {
    // SAFETY: `FrameHeader` is `repr(C)` plain data and the buffer is at least `HEADER_SIZE`.
    unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const FrameHeader) }
}

fn write_header(buf: &mut [u8], header: &FrameHeader) {
    // SAFETY: see `read_header`.
    unsafe { std::ptr::write_unaligned(buf.as_mut_ptr() as *mut FrameHeader, *header) }
}

/// Writes one frame into the mapping. Returns an error when the mapping is too small.
pub fn write_frame(
    buf: &mut [u8],
    frame: &Frame,
    timestamp_ms: i64,
    fps: u32,
    state: u32,
) -> Result<(), SinkError> {
    let len = mapping_len(frame.width(), frame.height());
    if buf.len() < len {
        return Err(SinkError::Io(
            "shared memory is too small for this frame".into(),
        ));
    }
    let mut header = read_header(buf);
    header.magic = MAGIC;
    header.version = VERSION;
    header.width = frame.width();
    header.height = frame.height();
    header.fps = fps;
    header.state = state;
    header.timestamp_ms = timestamp_ms.max(0) as u64;
    header.frame_counter = header.frame_counter.wrapping_add(1);
    header.sequence |= 1; // odd: a consumer must not trust the payload now
    write_header(buf, &header);
    fence(Ordering::SeqCst);
    buf[HEADER_SIZE..len].copy_from_slice(frame.pixels());
    fence(Ordering::SeqCst);
    header.sequence = header.sequence.wrapping_add(1); // even: payload is stable
    write_header(buf, &header);
    Ok(())
}

/// Reads the newest frame, retrying while the producer is writing. Returns `None`
/// when there is no valid frame (not initialised, torn for too long, or too small).
pub fn read_frame(buf: &[u8]) -> Option<ReadFrame> {
    if buf.len() < HEADER_SIZE {
        return None;
    }
    for _ in 0..READ_ATTEMPTS {
        let start = read_header(buf);
        if start.magic != MAGIC || start.version != VERSION {
            return None;
        }
        if start.sequence & 1 == 1 {
            std::hint::spin_loop();
            continue;
        }
        let len = mapping_len(start.width, start.height);
        if start.width == 0 || start.height == 0 || len > buf.len() {
            return None;
        }
        fence(Ordering::SeqCst);
        let mut pixels = vec![0u8; len - HEADER_SIZE];
        pixels.copy_from_slice(&buf[HEADER_SIZE..len]);
        fence(Ordering::SeqCst);
        let end = read_header(buf);
        if end.sequence == start.sequence && end.sequence & 1 == 0 {
            let frame = Frame::from_bgra(start.width, start.height, pixels)?;
            return Some(ReadFrame { header: end, frame });
        }
        std::hint::spin_loop();
    }
    None
}

/// Marks the mapping as "producer stopped" without touching the pixels.
pub fn mark_idle(buf: &mut [u8]) {
    if buf.len() < HEADER_SIZE {
        return;
    }
    let mut header = read_header(buf);
    header.sequence |= 1;
    write_header(buf, &header);
    fence(Ordering::SeqCst);
    header.state = STATE_IDLE;
    header.sequence = header.sequence.wrapping_add(1);
    write_header(buf, &header);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(width: u32, height: u32) -> Vec<u8> {
        vec![0u8; mapping_len(width, height)]
    }

    #[test]
    fn the_header_is_48_bytes_so_the_payload_starts_aligned() {
        assert_eq!(HEADER_SIZE, 48);
        assert_eq!(mapping_len(1280, 720), 48 + 1280 * 720 * 4);
    }

    #[test]
    fn writes_and_reads_a_frame() {
        let mut buf = buffer(4, 2);
        let frame = Frame::filled(4, 2, (1, 2, 3));
        write_frame(&mut buf, &frame, 1_234, 30, STATE_LIVE).expect("write");
        let read = read_frame(&buf).expect("read");
        assert_eq!(read.header.magic, MAGIC);
        assert_eq!(read.header.version, VERSION);
        assert_eq!(read.header.width, 4);
        assert_eq!(read.header.height, 2);
        assert_eq!(read.header.fps, 30);
        assert_eq!(read.header.state, STATE_LIVE);
        assert_eq!(read.header.timestamp_ms, 1_234);
        assert_eq!(read.header.frame_counter, 1);
        assert_eq!(read.frame, frame);
    }

    #[test]
    fn the_frame_counter_advances_with_every_write() {
        let mut buf = buffer(2, 2);
        let frame = Frame::filled(2, 2, (9, 9, 9));
        write_frame(&mut buf, &frame, 1, 30, STATE_LIVE).expect("write");
        write_frame(&mut buf, &frame, 2, 30, STATE_LIVE).expect("write");
        assert_eq!(read_frame(&buf).expect("read").header.frame_counter, 2);
    }

    #[test]
    fn an_empty_mapping_has_no_frame() {
        assert!(read_frame(&buffer(4, 2)).is_none());
    }

    #[test]
    fn a_torn_frame_is_rejected_instead_of_returned() {
        let mut buf = buffer(4, 2);
        write_frame(&mut buf, &Frame::filled(4, 2, (1, 1, 1)), 1, 30, STATE_LIVE).expect("write");
        // Simulate a producer that died mid-copy: odd sequence.
        let mut header = read_header(&buf);
        header.sequence |= 1;
        write_header(&mut buf, &header);
        assert!(read_frame(&buf).is_none());
    }

    #[test]
    fn refuses_frames_that_do_not_fit_the_mapping() {
        let mut buf = buffer(4, 2);
        let too_big = Frame::filled(8, 8, (1, 1, 1));
        assert!(write_frame(&mut buf, &too_big, 1, 30, STATE_LIVE).is_err());
    }

    #[test]
    fn mark_idle_keeps_the_last_frame_and_flags_the_state() {
        let mut buf = buffer(2, 2);
        let frame = Frame::filled(2, 2, (7, 7, 7));
        write_frame(&mut buf, &frame, 5, 30, STATE_LIVE).expect("write");
        mark_idle(&mut buf);
        let read = read_frame(&buf).expect("read");
        assert_eq!(read.header.state, STATE_IDLE);
        assert_eq!(read.frame, frame);
        assert_eq!(read.header.frame_counter, 1, "no new frame was produced");
    }
}
