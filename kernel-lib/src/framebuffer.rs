//! Pixel math for the framebuffer. Pure functions over byte slices, so they are tested on the host.
//!
//! "Never writes outside the buffer" is the contract of this module: out-of-range input is
//! clipped or ignored, it never panics and never writes a byte outside the slice it is given.

use crate::boot_info::{FramebufferInfo, PixelFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    fn grey(&self) -> u8 {
        let r = self.r as u32;
        let g = self.g as u32;
        let b = self.b as u32;
        ((r + g + b) / 3) as u8
    }
}

/// A rectangle in pixels. `x`/`y` is the top-left corner; it may extend past the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

/// Byte offset of the pixel at (`x`, `y`) from the start of the framebuffer.
///
/// `None` if the pixel is off screen (`x >= width` or `y >= height`). Note that `stride` is
/// counted in pixels and can be larger than `width`.
pub fn pixel_offset(info: &FramebufferInfo, x: usize, y: usize) -> Option<usize> {
    if x >= info.width || y >= info.height {
        return None;
    }
    let offset = (y.checked_mul(info.stride)?.checked_add(x))?.checked_mul(info.bytes_per_pixel)?;
    if let Some(offset_end) = offset.checked_add(info.bytes_per_pixel)
        && offset_end <= info.byte_len
    {
        Some(offset)
    } else {
        None
    }
}

/// Fills `rect` with `color`, clipped to the screen.
///
/// * Bytes of each pixel, by `info.pixel_format`: `Rgb` = `[r, g, b]`, `Bgr` = `[b, g, r]`,
///   `Gray` = `[(r + g + b) / 3]` (sum in a wider type!). Remaining bytes of the pixel
///   (`bytes_per_pixel` can be 4) are set to 0.
/// * Padding at the end of each row (`stride > width`) is never touched.
/// * `PixelFormat::Unknown` draws nothing.
/// * A rectangle that is empty, entirely off screen, or so large that `x + width` overflows must
///   not panic; the visible part (if any) is still drawn.
/// * If `buffer` is shorter than `info.byte_len` claims, pixels that do not fit are skipped.
pub fn fill_rect(buffer: &mut [u8], info: &FramebufferInfo, rect: Rect, color: Color) {
    let y_pixels = rect.y..rect.y.saturating_add(rect.height).min(info.height);
    for y in y_pixels {
        let x_pixels = rect.x..rect.x.saturating_add(rect.width).min(info.width);
        for x in x_pixels {
            if let Some(pixel_byte_start) = pixel_offset(info, x, y)
                && let Some(pixel_byte_end) = pixel_byte_start.checked_add(info.bytes_per_pixel)
            {
                let pixel_bytes_range = pixel_byte_start..pixel_byte_end;
                if let Some(pixel_bytes) = buffer.get_mut(pixel_bytes_range) {
                    fill_pixel_bytes(pixel_bytes, info.pixel_format, color);
                }
            }
        }
    }
}

fn fill_pixel_bytes(pixel_bytes: &mut [u8], pixel_format: PixelFormat, color: Color) {
    match pixel_format {
        PixelFormat::Rgb => {
            if pixel_bytes.len() >= 3 {
                pixel_bytes.fill(0);
                pixel_bytes[..3].copy_from_slice(&[color.r, color.g, color.b]);
            }
        }
        PixelFormat::Bgr => {
            if pixel_bytes.len() >= 3 {
                pixel_bytes.fill(0);
                pixel_bytes[..3].copy_from_slice(&[color.b, color.g, color.r])
            }
        }
        PixelFormat::Gray => {
            if !pixel_bytes.is_empty() {
                pixel_bytes.fill(0);
                pixel_bytes[0] = color.grey();
            }
        }
        PixelFormat::Unknown => {}
    }
}
