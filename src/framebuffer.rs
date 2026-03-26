use crate::error::VgaError;
use crate::mmio::write_framebuffer_u32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PixelFormat {
    Xrgb8888,
    Argb8888,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FramebufferInfo {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub framebuffer_paddr: usize,
    pub framebuffer_size: usize,
    pub bytes_per_pixel: u8,
    pub pixel_format: PixelFormat,
}

impl FramebufferInfo {
    pub(crate) fn pixel_offset(&self, x: u32, y: u32) -> Result<usize, VgaError> {
        if x >= self.width || y >= self.height {
            return Err(VgaError::InvalidPixelPosition);
        }
        let bytes_per_pixel = usize::from(self.bytes_per_pixel);
        let row_offset = (y as usize)
            .checked_mul(self.stride as usize)
            .ok_or(VgaError::InvalidFramebuffer)?;
        let pixel_index = row_offset
            .checked_add(x as usize)
            .ok_or(VgaError::InvalidFramebuffer)?;
        let byte_offset = pixel_index
            .checked_mul(bytes_per_pixel)
            .ok_or(VgaError::InvalidFramebuffer)?;
        let end_offset = byte_offset
            .checked_add(bytes_per_pixel)
            .ok_or(VgaError::InvalidFramebuffer)?;
        if end_offset > self.framebuffer_size {
            return Err(VgaError::InvalidFramebuffer);
        }
        Ok(byte_offset)
    }
}

pub(crate) fn draw_pixel(
    info: &FramebufferInfo,
    x: u32,
    y: u32,
    color: u32,
) -> Result<(), VgaError> {
    if info.bytes_per_pixel != 4 {
        return Err(VgaError::UnsupportedFormat);
    }
    let offset = info.pixel_offset(x, y)?;
    let addr = info
        .framebuffer_paddr
        .checked_add(offset)
        .ok_or(VgaError::InvalidFramebuffer)?;
    unsafe { write_framebuffer_u32(addr, color) };
    Ok(())
}

pub(crate) fn clear_screen(info: &FramebufferInfo, color: u32) -> Result<(), VgaError> {
    if info.bytes_per_pixel != 4 {
        return Err(VgaError::UnsupportedFormat);
    }
    let bytes_per_pixel = usize::from(info.bytes_per_pixel);
    if !info.framebuffer_size.is_multiple_of(bytes_per_pixel) {
        return Err(VgaError::InvalidFramebuffer);
    }
    let pixel_count = info.framebuffer_size / bytes_per_pixel;
    for index in 0..pixel_count {
        let offset = index
            .checked_mul(bytes_per_pixel)
            .ok_or(VgaError::InvalidFramebuffer)?;
        let addr = info
            .framebuffer_paddr
            .checked_add(offset)
            .ok_or(VgaError::InvalidFramebuffer)?;
        unsafe { write_framebuffer_u32(addr, color) };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{FramebufferInfo, PixelFormat, VgaError};

    fn framebuffer_info() -> FramebufferInfo {
        FramebufferInfo {
            width: 8,
            height: 6,
            stride: 8,
            framebuffer_paddr: 0x1000,
            framebuffer_size: 8 * 6 * 4,
            bytes_per_pixel: 4,
            pixel_format: PixelFormat::Xrgb8888,
        }
    }

    #[test]
    fn pixel_offset_matches_layout() {
        let info = framebuffer_info();
        assert_eq!(info.pixel_offset(0, 0), Ok(0));
        assert_eq!(info.pixel_offset(3, 2), Ok((2 * 8 + 3) * 4));
    }

    #[test]
    fn pixel_offset_rejects_out_of_bounds_coordinates() {
        let info = framebuffer_info();
        assert_eq!(info.pixel_offset(8, 0), Err(VgaError::InvalidPixelPosition));
        assert_eq!(info.pixel_offset(0, 6), Err(VgaError::InvalidPixelPosition));
    }

    #[test]
    fn pixel_offset_rejects_invalid_framebuffer_range() {
        let mut info = framebuffer_info();
        info.framebuffer_size = 4;
        assert_eq!(info.pixel_offset(1, 0), Err(VgaError::InvalidFramebuffer));
    }
}
