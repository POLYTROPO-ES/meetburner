//! One BGRA frame of the virtual camera.

pub const FRAME_WIDTH: u32 = 1280;
pub const FRAME_HEIGHT: u32 = 720;
pub const FPS: u32 = 30;

/// Output sizes the desktop app offers for the camera. Mirrors `CAMERA_SIZES` in
/// `packages/core/src/camera.ts` (ADR-0015): the same ratios in landscape, portrait and square.
pub const SUPPORTED_SIZES: &[(u32, u32)] = &[
    // 16:9 landscape
    (1920, 1080),
    (1280, 720),
    (640, 360),
    // 9:16 portrait
    (1080, 1920),
    (720, 1280),
    (360, 640),
    // 4:3 landscape
    (1440, 1080),
    (960, 720),
    (640, 480),
    // 3:4 portrait
    (1080, 1440),
    (720, 960),
    (480, 640),
    // 1:1 square
    (1080, 1080),
    (720, 720),
    (480, 480),
];

/// Only the sizes in `SUPPORTED_SIZES` are produced; anything else falls back to 1280x720.
pub fn normalize_size(width: u32, height: u32) -> (u32, u32) {
    if SUPPORTED_SIZES.contains(&(width, height)) {
        (width, height)
    } else {
        (FRAME_WIDTH, FRAME_HEIGHT)
    }
}

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

    /// Builds a frame from RGBA bytes (what webcam backends hand out), swapping to BGRA.
    /// Returns `None` when the buffer and the size disagree.
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Option<Frame> {
        if width == 0 || height == 0 || rgba.len() != width as usize * height as usize * 4 {
            return None;
        }
        let mut frame = Frame::new(width, height);
        for (pixel, source) in frame
            .pixels
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(rgba.as_chunks::<4>().0)
        {
            *pixel = [source[2], source[1], source[0], 0xff];
        }
        Some(frame)
    }

    /// Mirror of this frame, horizontally and/or vertically (ADR-0022).
    pub fn flipped(&self, horizontal: bool, vertical: bool) -> Frame {
        if !horizontal && !vertical {
            return self.clone();
        }
        let mut frame = Frame::new(self.width, self.height);
        let source_stride = self.stride();
        let target_stride = frame.stride();
        for y in 0..self.height {
            let source_y = if vertical { self.height - 1 - y } else { y };
            let source_row = &self.pixels[source_y as usize * source_stride..];
            let target_row = &mut frame.pixels[y as usize * target_stride..];
            if horizontal {
                for x in 0..self.width {
                    let from = (self.width - 1 - x) as usize * 4;
                    target_row[x as usize * 4..x as usize * 4 + 4]
                        .copy_from_slice(&source_row[from..from + 4]);
                }
            } else {
                target_row[..source_stride].copy_from_slice(&source_row[..source_stride]);
            }
        }
        frame
    }

    /// Nearest-neighbour resize: cheap enough at 30 fps and keeps the counter sharp.
    pub fn scaled_to(&self, width: u32, height: u32) -> Frame {
        if width == self.width && height == self.height {
            return self.clone();
        }
        let mut frame = Frame::new(width, height);
        let source_stride = self.stride();
        let target_stride = frame.stride();
        for y in 0..height {
            let source_y = (y as u64 * self.height as u64 / height as u64) as u32;
            let source_row = &self.pixels[source_y as usize * source_stride..];
            let target_row = &mut frame.pixels[y as usize * target_stride..];
            for x in 0..width {
                let source_x = (x as u64 * self.width as u64 / width as u64) as u32;
                let from = source_x as usize * 4;
                target_row[x as usize * 4..x as usize * 4 + 4]
                    .copy_from_slice(&source_row[from..from + 4]);
            }
        }
        frame
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

    #[test]
    fn only_supported_sizes_are_produced() {
        for size in SUPPORTED_SIZES {
            assert_eq!(normalize_size(size.0, size.1), *size);
        }
        assert_eq!(normalize_size(800, 600), (FRAME_WIDTH, FRAME_HEIGHT));
        assert_eq!(normalize_size(0, 0), (FRAME_WIDTH, FRAME_HEIGHT));
        assert_eq!(normalize_size(1920, 720), (FRAME_WIDTH, FRAME_HEIGHT));
    }

    #[test]
    fn covers_landscape_portrait_and_square_frames() {
        assert!(SUPPORTED_SIZES.iter().any(|(w, h)| w > h), "landscape");
        assert!(SUPPORTED_SIZES.iter().any(|(w, h)| w < h), "portrait");
        assert!(SUPPORTED_SIZES.iter().any(|(w, h)| w == h), "square");
        for (width, height) in SUPPORTED_SIZES {
            let (w, h) = (*width as u64, *height as u64);
            let known = [(16u64, 9u64), (9, 16), (4, 3), (3, 4), (1, 1)]
                .iter()
                .any(|(ratio_w, ratio_h)| w * ratio_h == h * ratio_w);
            assert!(known, "{width}x{height} keeps a known ratio");
        }
    }

    #[test]
    fn webcam_rgba_becomes_bgra() {
        // Two pixels: red then green, as RGBA.
        let rgba = [255, 0, 0, 255, 0, 255, 0, 255];
        let frame = Frame::from_rgba(2, 1, &rgba).expect("a valid buffer");
        assert_eq!(frame.pixel(0, 0), (255, 0, 0));
        assert_eq!(frame.pixel(1, 0), (0, 255, 0));
        assert!(Frame::from_rgba(2, 1, &rgba[..7]).is_none(), "short buffer");
    }

    #[test]
    fn scaling_keeps_the_corners() {
        let mut source = Frame::new(2, 2);
        source.set_pixel(0, 0, (1, 1, 1));
        source.set_pixel(1, 1, (9, 9, 9));
        let big = source.scaled_to(4, 4);
        assert_eq!((big.width(), big.height()), (4, 4));
        assert_eq!(big.pixel(0, 0), (1, 1, 1));
        assert_eq!(big.pixel(3, 3), (9, 9, 9));
        assert_eq!(big.pixel(0, 3), (0, 0, 0));
        assert_eq!(source.scaled_to(2, 2), source, "same size is a copy");
    }

    #[test]
    fn flipping_mirrors_the_requested_axes() {
        let mut source = Frame::new(2, 2);
        source.set_pixel(0, 0, (1, 1, 1));
        source.set_pixel(1, 0, (2, 2, 2));
        source.set_pixel(0, 1, (3, 3, 3));
        source.set_pixel(1, 1, (4, 4, 4));

        let unchanged = source.flipped(false, false);
        assert_eq!(unchanged, source);

        let horizontal = source.flipped(true, false);
        assert_eq!(horizontal.pixel(0, 0), (2, 2, 2));
        assert_eq!(horizontal.pixel(1, 1), (3, 3, 3));

        let vertical = source.flipped(false, true);
        assert_eq!(vertical.pixel(0, 0), (3, 3, 3));
        assert_eq!(vertical.pixel(1, 1), (2, 2, 2));

        let both = source.flipped(true, true);
        assert_eq!(both.pixel(0, 0), (4, 4, 4));
        assert_eq!(both.pixel(1, 1), (1, 1, 1));
    }
}
