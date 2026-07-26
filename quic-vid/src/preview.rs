#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewFrame {
    pub frame_id: u64,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_frame_preserves_metadata() {
        let frame = PreviewFrame {
            frame_id: 42,
            width: 640,
            height: 360,
            pixels: vec![0; 640 * 360],
        };

        assert_eq!(frame.frame_id, 42);
        assert_eq!(frame.width, 640);
        assert_eq!(frame.height, 360);
    }

    #[test]
    fn preview_frame_preserves_pixel_buffer() {
        let pixels = vec![0x00112233, 0x00445566];

        let frame = PreviewFrame {
            frame_id: 7,
            width: 2,
            height: 1,
            pixels: pixels.clone(),
        };

        assert_eq!(frame.pixels, pixels);
    }
}
