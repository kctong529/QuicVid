use image::{Rgb, RgbImage};

pub const TEST_FRAME_WIDTH: u32 = 640;
pub const TEST_FRAME_HEIGHT: u32 = 360;

const DIGIT_SEGMENTS: [[bool; 7]; 10] = [
    [true, true, true, false, true, true, true],     // 0
    [false, false, true, false, false, true, false], // 1
    [true, false, true, true, true, false, true],    // 2
    [true, false, true, true, false, true, true],    // 3
    [false, true, true, true, false, true, false],   // 4
    [true, true, false, true, false, true, true],    // 5
    [true, true, false, true, true, true, true],     // 6
    [true, false, true, false, false, true, false],  // 7
    [true, true, true, true, true, true, true],      // 8
    [true, true, true, true, false, true, true],     // 9
];

pub fn generate_test_pattern(frame_id: u64) -> RgbImage {
    let mut image = RgbImage::new(TEST_FRAME_WIDTH, TEST_FRAME_HEIGHT);

    draw_background(&mut image, frame_id);
    draw_frame_id(&mut image, frame_id);
    draw_progress_marker(&mut image, frame_id);

    image
}

fn draw_background(image: &mut RgbImage, frame_id: u64) {
    let phase = (frame_id % 256) as u8;

    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let horizontal = ((x * 255) / TEST_FRAME_WIDTH) as u8;
        let vertical = ((y * 255) / TEST_FRAME_HEIGHT) as u8;

        *pixel = Rgb([horizontal, vertical, horizontal.wrapping_add(phase)]);
    }
}

fn draw_frame_id(image: &mut RgbImage, frame_id: u64) {
    let text = format!("{:06}", frame_id % 1_000_000);

    let scale = 6;
    let digit_width = 5 * scale;
    let spacing = 2 * scale;

    let total_width = text.len() as u32 * digit_width + (text.len() as u32 - 1) * spacing;

    let start_x = (TEST_FRAME_WIDTH - total_width) / 2;
    let start_y = 120;

    for (index, character) in text.chars().enumerate() {
        let digit = character.to_digit(10).unwrap() as usize;

        draw_digit(
            image,
            digit,
            start_x + index as u32 * (digit_width + spacing),
            start_y,
            scale,
        );
    }
}

fn draw_digit(image: &mut RgbImage, digit: usize, x: u32, y: u32, scale: u32) {
    let segments = DIGIT_SEGMENTS[digit];
    let color = Rgb([255, 255, 255]);

    let rectangles = [
        (x + scale, y, 3 * scale, scale),
        (x, y + scale, scale, 3 * scale),
        (x + 4 * scale, y + scale, scale, 3 * scale),
        (x + scale, y + 4 * scale, 3 * scale, scale),
        (x, y + 5 * scale, scale, 3 * scale),
        (x + 4 * scale, y + 5 * scale, scale, 3 * scale),
        (x + scale, y + 8 * scale, 3 * scale, scale),
    ];

    for (enabled, rectangle) in segments.into_iter().zip(rectangles) {
        if enabled {
            let (rx, ry, width, height) = rectangle;
            draw_rect(image, rx, ry, width, height, color);
        }
    }
}

fn draw_rect(image: &mut RgbImage, x: u32, y: u32, width: u32, height: u32, color: Rgb<u8>) {
    let max_x = (x + width).min(image.width());
    let max_y = (y + height).min(image.height());

    for py in y..max_y {
        for px in x..max_x {
            image.put_pixel(px, py, color);
        }
    }
}

fn draw_progress_marker(image: &mut RgbImage, frame_id: u64) {
    let marker_width = 24;
    let usable_width = TEST_FRAME_WIDTH - marker_width;

    let x = ((frame_id * 7) % u64::from(usable_width)) as u32;

    draw_rect(image, x, 315, marker_width, 24, Rgb([255, 255, 255]));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_frame_has_expected_dimensions() {
        let frame = generate_test_pattern(42);

        assert_eq!(frame.dimensions(), (TEST_FRAME_WIDTH, TEST_FRAME_HEIGHT));
    }

    #[test]
    fn generated_frames_change_with_frame_id() {
        let first = generate_test_pattern(0);

        let second = generate_test_pattern(1);

        assert_ne!(first.as_raw(), second.as_raw());
    }

    #[test]
    fn generation_is_deterministic() {
        let first = generate_test_pattern(42);

        let second = generate_test_pattern(42);

        assert_eq!(first.as_raw(), second.as_raw());
    }
}

#[test]
#[ignore = "manual visual inspection helper"]
fn writes_test_pattern_preview() {
    use std::io::Write;

    let frame = generate_test_pattern(43);
    let path = std::env::temp_dir().join("quicvid-test-pattern-43.ppm");

    let mut file = std::fs::File::create(&path).expect("failed to create preview");

    write!(file, "P6\n{} {}\n255\n", frame.width(), frame.height())
        .expect("failed to write PPM header");

    file.write_all(frame.as_raw())
        .expect("failed to write RGB pixels");

    println!("preview written to {}", path.display());
}
