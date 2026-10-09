use kernel_lib::boot_info::{FramebufferInfo, PixelFormat};
use kernel_lib::framebuffer::{Color, Rect, fill_rect, pixel_offset};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const SENTINEL: u8 = 0xAA;

fn info(
    width: usize,
    height: usize,
    stride: usize,
    bpp: usize,
    format: PixelFormat,
) -> FramebufferInfo {
    FramebufferInfo {
        address: 0,
        byte_len: stride * height * bpp,
        width,
        height,
        stride,
        bytes_per_pixel: bpp,
        pixel_format: format,
    }
}

fn buffer(info: &FramebufferInfo) -> Vec<u8> {
    vec![SENTINEL; info.byte_len]
}

fn pixel(buf: &[u8], info: &FramebufferInfo, x: usize, y: usize) -> Vec<u8> {
    let start = (y * info.stride + x) * info.bytes_per_pixel;
    buf[start..start + info.bytes_per_pixel].to_vec()
}

#[test]
fn offset_of_the_origin_is_zero() {
    assert_eq!(
        pixel_offset(&info(10, 5, 10, 4, PixelFormat::Rgb), 0, 0),
        Some(0)
    );
}

#[test]
fn offset_scales_by_bytes_per_pixel() {
    assert_eq!(
        pixel_offset(&info(10, 5, 10, 4, PixelFormat::Rgb), 3, 0),
        Some(12)
    );
    assert_eq!(
        pixel_offset(&info(10, 5, 10, 3, PixelFormat::Rgb), 3, 0),
        Some(9)
    );
}

#[test]
fn offset_uses_stride_not_width_to_find_the_row() {
    let i = info(10, 5, 16, 4, PixelFormat::Rgb);
    assert_eq!(pixel_offset(&i, 2, 3), Some((3 * 16 + 2) * 4));
}

#[test]
fn offset_of_the_last_visible_pixel() {
    let i = info(10, 5, 16, 4, PixelFormat::Rgb);
    assert_eq!(pixel_offset(&i, 9, 4), Some((4 * 16 + 9) * 4));
}

#[test]
fn offset_is_none_off_screen_even_inside_the_stride_padding() {
    let i = info(10, 5, 16, 4, PixelFormat::Rgb);
    assert_eq!(pixel_offset(&i, 10, 0), None);
    assert_eq!(pixel_offset(&i, 12, 0), None);
    assert_eq!(pixel_offset(&i, 0, 5), None);
    assert_eq!(pixel_offset(&i, usize::MAX, usize::MAX), None);
}

#[test]
fn fills_rgb_pixels() {
    let i = info(8, 4, 8, 4, PixelFormat::Rgb);
    let mut buf = buffer(&i);
    fill_rect(
        &mut buf,
        &i,
        Rect {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        },
        Color { r: 1, g: 2, b: 3 },
    );
    assert_eq!(pixel(&buf, &i, 1, 1), [1, 2, 3, 0]);
    assert_eq!(pixel(&buf, &i, 2, 2), [1, 2, 3, 0]);
}

#[test]
fn fills_bgr_pixels() {
    let i = info(8, 4, 8, 4, PixelFormat::Bgr);
    let mut buf = buffer(&i);
    fill_rect(
        &mut buf,
        &i,
        Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        Color { r: 1, g: 2, b: 3 },
    );
    assert_eq!(pixel(&buf, &i, 0, 0), [3, 2, 1, 0]);
}

#[test]
fn fills_three_byte_pixels_without_touching_the_next_pixel() {
    let i = info(8, 4, 8, 3, PixelFormat::Rgb);
    let mut buf = buffer(&i);
    fill_rect(
        &mut buf,
        &i,
        Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        RED,
    );
    assert_eq!(pixel(&buf, &i, 0, 0), [255, 0, 0]);
    assert_eq!(pixel(&buf, &i, 1, 0), [SENTINEL; 3]);
}

#[test]
fn gray_is_the_average_of_the_channels() {
    let i = info(4, 4, 4, 1, PixelFormat::Gray);
    let mut buf = buffer(&i);
    fill_rect(
        &mut buf,
        &i,
        Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        Color {
            r: 255,
            g: 255,
            b: 255,
        },
    );
    assert_eq!(pixel(&buf, &i, 0, 0), [255]);
    fill_rect(
        &mut buf,
        &i,
        Rect {
            x: 1,
            y: 0,
            width: 1,
            height: 1,
        },
        Color {
            r: 30,
            g: 60,
            b: 90,
        },
    );
    assert_eq!(pixel(&buf, &i, 1, 0), [60]);
}

#[test]
fn only_the_rectangle_changes() {
    let i = info(8, 6, 8, 4, PixelFormat::Rgb);
    let mut buf = buffer(&i);
    fill_rect(
        &mut buf,
        &i,
        Rect {
            x: 2,
            y: 1,
            width: 3,
            height: 2,
        },
        RED,
    );
    for y in 0..6 {
        for x in 0..8 {
            let inside = (2..5).contains(&x) && (1..3).contains(&y);
            let expected: &[u8] = if inside {
                &[255, 0, 0, 0]
            } else {
                &[SENTINEL; 4]
            };
            assert_eq!(pixel(&buf, &i, x, y), expected, "pixel ({x}, {y})");
        }
    }
}

#[test]
fn stride_padding_is_never_touched() {
    let i = info(4, 3, 7, 4, PixelFormat::Rgb);
    let mut buf = buffer(&i);
    fill_rect(
        &mut buf,
        &i,
        Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        },
        RED,
    );
    for y in 0..3 {
        for x in 0..4 {
            assert_eq!(pixel(&buf, &i, x, y), [255, 0, 0, 0]);
        }
        for x in 4..7 {
            assert_eq!(pixel(&buf, &i, x, y), [SENTINEL; 4], "padding ({x}, {y})");
        }
    }
}

#[test]
fn rectangle_is_clipped_at_the_right_and_bottom_edges() {
    let i = info(8, 4, 8, 4, PixelFormat::Rgb);
    let mut buf = buffer(&i);
    fill_rect(
        &mut buf,
        &i,
        Rect {
            x: 6,
            y: 3,
            width: 10,
            height: 10,
        },
        RED,
    );
    assert_eq!(pixel(&buf, &i, 7, 3), [255, 0, 0, 0]);
    assert_eq!(pixel(&buf, &i, 5, 3), [SENTINEL; 4]);
    assert_eq!(pixel(&buf, &i, 6, 2), [SENTINEL; 4]);
}

#[test]
fn empty_and_off_screen_rectangles_draw_nothing() {
    let i = info(8, 4, 8, 4, PixelFormat::Rgb);
    let untouched = buffer(&i);
    for rect in [
        Rect {
            x: 1,
            y: 1,
            width: 0,
            height: 5,
        },
        Rect {
            x: 1,
            y: 1,
            width: 5,
            height: 0,
        },
        Rect {
            x: 8,
            y: 0,
            width: 3,
            height: 3,
        },
        Rect {
            x: 0,
            y: 4,
            width: 3,
            height: 3,
        },
        Rect {
            x: usize::MAX,
            y: usize::MAX,
            width: 5,
            height: 5,
        },
    ] {
        let mut buf = untouched.clone();
        fill_rect(&mut buf, &i, rect, RED);
        assert_eq!(buf, untouched, "{rect:?}");
    }
}

#[test]
fn huge_rectangles_do_not_overflow() {
    let i = info(8, 4, 8, 4, PixelFormat::Rgb);
    let mut buf = buffer(&i);
    fill_rect(
        &mut buf,
        &i,
        Rect {
            x: 3,
            y: 2,
            width: usize::MAX,
            height: usize::MAX,
        },
        RED,
    );
    assert_eq!(pixel(&buf, &i, 3, 2), [255, 0, 0, 0]);
    assert_eq!(pixel(&buf, &i, 7, 3), [255, 0, 0, 0]);
    assert_eq!(pixel(&buf, &i, 2, 2), [SENTINEL; 4]);
}

#[test]
fn unknown_pixel_format_draws_nothing() {
    let i = info(8, 4, 8, 4, PixelFormat::Unknown);
    let untouched = buffer(&i);
    let mut buf = untouched.clone();
    fill_rect(
        &mut buf,
        &i,
        Rect {
            x: 0,
            y: 0,
            width: 8,
            height: 4,
        },
        RED,
    );
    assert_eq!(buf, untouched);
}

#[test]
fn a_buffer_shorter_than_the_info_claims_is_not_overrun() {
    let i = info(8, 4, 8, 4, PixelFormat::Rgb);
    let mut buf = vec![SENTINEL; 10 * 4]; // only 10 of the 32 pixels exist
    fill_rect(
        &mut buf,
        &i,
        Rect {
            x: 0,
            y: 0,
            width: 8,
            height: 4,
        },
        RED,
    );
    assert_eq!(buf.len(), 40);
    assert_eq!(&buf[36..40], &[255, 0, 0, 0]); // pixel 9, the last one that fits
}

// --- Misuse: a `FramebufferInfo` that does not make sense must never cause a panic. ---

const WHOLE_SCREEN: Rect = Rect {
    x: 0,
    y: 0,
    width: 100,
    height: 100,
};

#[test]
fn pixels_too_small_for_rgb_or_bgr_draw_nothing() {
    for format in [PixelFormat::Rgb, PixelFormat::Bgr] {
        for bpp in 0..3 {
            let i = info(4, 4, 4, bpp, format);
            let mut buf = buffer(&i);
            fill_rect(&mut buf, &i, WHOLE_SCREEN, RED);
            assert!(
                buf.iter().all(|&b| b == SENTINEL),
                "{format:?} with {bpp} bytes per pixel must not draw"
            );
        }
    }
}

#[test]
fn zero_byte_pixels_draw_nothing_in_gray() {
    let i = info(4, 4, 4, 0, PixelFormat::Gray);
    let mut buf = buffer(&i);
    fill_rect(&mut buf, &i, WHOLE_SCREEN, RED);
    assert!(buf.iter().all(|&b| b == SENTINEL));
}

#[test]
fn one_byte_gray_pixels_still_work() {
    let i = info(4, 4, 4, 1, PixelFormat::Gray);
    let mut buf = buffer(&i);
    fill_rect(
        &mut buf,
        &i,
        WHOLE_SCREEN,
        Color {
            r: 30,
            g: 60,
            b: 90,
        },
    );
    assert!(buf.iter().all(|&b| b == 60));
}

#[test]
fn an_enormous_stride_does_not_overflow_the_offset_math() {
    let i = FramebufferInfo {
        address: 0,
        byte_len: 64,
        width: 4,
        height: 4,
        stride: usize::MAX,
        bytes_per_pixel: 4,
        pixel_format: PixelFormat::Rgb,
    };
    assert_eq!(pixel_offset(&i, 1, 1), None);
    assert_eq!(pixel_offset(&i, 3, 3), None);
    let mut buf = vec![SENTINEL; 64];
    fill_rect(&mut buf, &i, WHOLE_SCREEN, RED);
    assert_eq!(pixel_offset(&i, 0, 0), Some(0));
}

#[test]
fn an_enormous_bytes_per_pixel_does_not_overflow() {
    let i = FramebufferInfo {
        address: 0,
        byte_len: 64,
        width: 4,
        height: 4,
        stride: 4,
        bytes_per_pixel: usize::MAX,
        pixel_format: PixelFormat::Rgb,
    };
    assert_eq!(pixel_offset(&i, 1, 1), None);
    let mut buf = vec![SENTINEL; 64];
    fill_rect(&mut buf, &i, WHOLE_SCREEN, RED);
    assert!(buf.iter().all(|&b| b == SENTINEL));
}

#[test]
fn a_pixel_that_ends_past_byte_len_is_off_screen() {
    // 2x2 pixels of 4 bytes need 16 bytes, but the info only claims 15.
    let mut i = info(2, 2, 2, 4, PixelFormat::Rgb);
    i.byte_len = 15;
    assert_eq!(pixel_offset(&i, 0, 1), Some(8));
    assert_eq!(pixel_offset(&i, 1, 1), None);
}

#[test]
fn nothing_is_written_past_byte_len_even_if_the_buffer_is_longer() {
    let mut i = info(2, 2, 2, 4, PixelFormat::Rgb);
    i.byte_len = 15;
    let mut buf = vec![SENTINEL; 16];
    fill_rect(&mut buf, &i, WHOLE_SCREEN, RED);
    assert!(buf[12..].iter().all(|&b| b == SENTINEL));
    assert_eq!(&buf[..4], &[255, 0, 0, 0]);
}
