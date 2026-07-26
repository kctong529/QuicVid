use minifb::{Key, Window, WindowOptions};
#[cfg(test)]
use tokio::sync::watch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewFrame {
    pub frame_id: u64,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

#[cfg(test)]
pub type PreviewSender = watch::Sender<Option<PreviewFrame>>;

#[cfg(test)]
pub type PreviewReceiver = watch::Receiver<Option<PreviewFrame>>;

#[cfg(test)]
pub fn channel() -> (PreviewSender, PreviewReceiver) {
    watch::channel(None)
}

#[cfg(test)]
pub fn publish(sender: &PreviewSender, frame: PreviewFrame) -> anyhow::Result<()> {
    sender
        .send(Some(frame))
        .map_err(|_| anyhow::anyhow!("preview receiver has closed"))?;

    Ok(())
}

#[cfg(test)]
pub fn take_latest_if_changed(receiver: &mut PreviewReceiver) -> Option<PreviewFrame> {
    if !receiver.has_changed().ok()? {
        return None;
    }

    receiver.borrow_and_update().clone()
}

pub fn preview_frame_from_jpeg(frame_id: u64, jpeg: &[u8]) -> anyhow::Result<PreviewFrame> {
    let image = image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg)?;

    let rgb = image.to_rgb8();

    let width: usize = rgb.width().try_into()?;
    let height: usize = rgb.height().try_into()?;

    let pixels = rgb
        .pixels()
        .map(|pixel| {
            let [red, green, blue] = pixel.0;

            (u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue)
        })
        .collect();

    Ok(PreviewFrame {
        frame_id,
        width,
        height,
        pixels,
    })
}

pub fn show_preview_frame(frame: &PreviewFrame) -> anyhow::Result<()> {
    validate_preview_frame(frame)?;

    let title = format!("QuicVid Receiver — frame {:06}", frame.frame_id);

    let mut window = Window::new(&title, frame.width, frame.height, WindowOptions::default())?;

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.update_with_buffer(&frame.pixels, frame.width, frame.height)?;
    }

    Ok(())
}

fn validate_preview_frame(frame: &PreviewFrame) -> anyhow::Result<()> {
    let expected_pixels = frame
        .width
        .checked_mul(frame.height)
        .ok_or_else(|| anyhow::anyhow!("preview dimensions overflow"))?;

    if frame.pixels.len() != expected_pixels {
        anyhow::bail!(
            "preview buffer size mismatch: expected {} pixels for {}x{}, got {}",
            expected_pixels,
            frame.width,
            frame.height,
            frame.pixels.len(),
        );
    }

    Ok(())
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

    #[test]
    fn generated_jpeg_converts_to_preview_frame() {
        let jpeg =
            crate::test_pattern::generate_jpeg_frame(42, crate::test_pattern::DEFAULT_JPEG_QUALITY)
                .unwrap();

        let frame = preview_frame_from_jpeg(42, &jpeg).unwrap();

        assert_eq!(frame.frame_id, 42);
        assert_eq!(frame.width, 640);
        assert_eq!(frame.height, 360);
        assert_eq!(frame.pixels.len(), 640 * 360);
    }

    #[test]
    fn invalid_jpeg_is_rejected() {
        assert!(preview_frame_from_jpeg(42, b"not a jpeg",).is_err());
    }

    #[test]
    fn preview_pixels_use_rgb_channel_order() {
        use image::{codecs::jpeg::JpegEncoder, Rgb, RgbImage};

        let mut image = RgbImage::new(1, 1);
        image.put_pixel(0, 0, Rgb([255, 0, 0]));

        let mut jpeg = Vec::new();

        {
            let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, 100);

            encoder.encode_image(&image).unwrap();
        }

        let frame = preview_frame_from_jpeg(1, &jpeg).unwrap();

        assert_eq!(frame.width, 1);
        assert_eq!(frame.height, 1);

        let pixel = frame.pixels[0];

        let red = (pixel >> 16) & 0xff;
        let green = (pixel >> 8) & 0xff;
        let blue = pixel & 0xff;

        assert!(red > green);
        assert!(red > blue);
    }

    #[test]
    fn preview_buffer_matches_frame_dimensions() {
        let jpeg =
            crate::test_pattern::generate_jpeg_frame(7, crate::test_pattern::DEFAULT_JPEG_QUALITY)
                .unwrap();

        let frame = preview_frame_from_jpeg(7, &jpeg).unwrap();

        assert_eq!(frame.pixels.len(), frame.width * frame.height);
    }

    #[test]
    fn preview_frame_rejects_wrong_buffer_size() {
        let frame = PreviewFrame {
            frame_id: 42,
            width: 2,
            height: 2,
            pixels: vec![0; 3],
        };

        assert!(validate_preview_frame(&frame).is_err());
    }

    #[test]
    fn preview_frame_accepts_matching_buffer_size() {
        let frame = PreviewFrame {
            frame_id: 42,
            width: 2,
            height: 2,
            pixels: vec![0; 4],
        };

        assert!(validate_preview_frame(&frame).is_ok());
    }

    #[test]
    fn preview_channel_starts_empty() {
        let (_sender, receiver) = channel();

        assert!(receiver.borrow().is_none());
    }

    #[test]
    fn published_frame_becomes_latest() {
        let (sender, mut receiver) = channel();

        let frame = PreviewFrame {
            frame_id: 42,
            width: 1,
            height: 1,
            pixels: vec![0x00112233],
        };

        publish(&sender, frame.clone()).unwrap();

        assert_eq!(take_latest_if_changed(&mut receiver), Some(frame));
    }

    #[test]
    fn unchanged_preview_is_not_returned_twice() {
        let (sender, mut receiver) = channel();

        let frame = PreviewFrame {
            frame_id: 42,
            width: 1,
            height: 1,
            pixels: vec![0],
        };

        publish(&sender, frame).unwrap();

        assert!(take_latest_if_changed(&mut receiver).is_some());

        assert!(take_latest_if_changed(&mut receiver).is_none());
    }

    #[test]
    fn preview_channel_keeps_only_latest_frame() {
        let (sender, mut receiver) = channel();

        for frame_id in 40..=46 {
            publish(
                &sender,
                PreviewFrame {
                    frame_id,
                    width: 1,
                    height: 1,
                    pixels: vec![frame_id as u32],
                },
            )
            .unwrap();
        }

        let latest = take_latest_if_changed(&mut receiver).unwrap();

        assert_eq!(latest.frame_id, 46);
    }

    #[test]
    fn publishing_after_receiver_closes_returns_error() {
        let (sender, receiver) = channel();

        drop(receiver);

        let result = publish(
            &sender,
            PreviewFrame {
                frame_id: 42,
                width: 1,
                height: 1,
                pixels: vec![0],
            },
        );

        assert!(result.is_err());
    }
}
