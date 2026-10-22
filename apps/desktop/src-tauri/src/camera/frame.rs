//! One BGRA frame of the virtual camera.

pub const FRAME_WIDTH: u32 = 1280;
pub const FRAME_HEIGHT: u32 = 720;
pub const FPS: u32 = 30;

/// Tightly packed 32-bit BGRA pixels, top-down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Frame {
    pub fn new(width: u32, height: u32) -> Frame {
        Frame {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        }
    }

    /// Frame filled with an opaque colour.
    pub fn filled(width: u32, height: u32, rgb: (u8, u8, u8)) -> Frame {
        let mut frame = Frame::new(width, height);
        for pixel in frame.pixels.as_chunks_mut::<4>().0 {
            *pixel = [rgb.2, rgb.1, rgb.0, 0xff];
        }
        frame
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Bytes per row: 4 per pixel, no padding.
    pub fn stride(&self) -> usize {
        self.width as usize * 4
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, rgb: (u8, u8, u8)) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = (y as usize * self.stride()) + x as usize * 4;
        self.pixels[offset..offset + 4].copy_from_slice(&[rgb.2, rgb.1, rgb.0, 0xff]);
    }

    pub fn pixel(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let offset = (y as usize * self.stride()) + x as usize * 4;
        (
            self.pixels[offset + 2],
            self.pixels[offset + 1],
            self.pixels[offset],
        )
    }

    /// Builds a frame from a raw BGRA payload (shared memory, tests).
    pub(crate) fn from_bgra(width: u32, height: u32, pixels: Vec<u8>) -> Option<Frame> {
        if pixels.len() != width as usize * height as usize * 4 {
            return None;
        }
        Some(Frame {
            width,
            height,
            pixels,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_has_four_bytes_per_pixel_and_no_row_padding() {
        let frame = Frame::new(FRAME_WIDTH, FRAME_HEIGHT);
        assert_eq!(frame.width(), 1280);
        assert_eq!(frame.height(), 720);
        assert_eq!(frame.stride(), 1280 * 4);
        assert_eq!(frame.pixels().len(), 1280 * 720 * 4);
    }

    #[test]
    fn pixels_are_stored_as_opaque_bgra() {
        let mut frame = Frame::new(2, 1);
        frame.set_pixel(0, 0, (0x11, 0x22, 0x33));
        assert_eq!(&frame.pixels()[0..4], &[0x33, 0x22, 0x11, 0xff]);
        assert_eq!(frame.pixel(0, 0), (0x11, 0x22, 0x33));
    }

    #[test]
    fn filled_paints_every_pixel_of_the_requested_colour() {
        let frame = Frame::filled(3, 2, (0x0f, 0x11, 0x15));
        assert_eq!(frame.pixel(0, 0), (0x0f, 0x11, 0x15));
        assert_eq!(frame.pixel(2, 1), (0x0f, 0x11, 0x15));
    }
}
