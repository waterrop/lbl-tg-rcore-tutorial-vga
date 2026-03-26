use crate::error::VgaError;
use crate::framebuffer::{FramebufferInfo, PixelFormat};
use crate::mmio::write_mmio_u64;
use core::ptr::addr_of_mut;

const FW_CFG_BASE: usize = 0x1010_0000;
const FW_CFG_DMA_REGISTER: usize = FW_CFG_BASE + 0x10;

const FW_CFG_SIGNATURE: u16 = 0x0000;
const FW_CFG_ID: u16 = 0x0001;
const FW_CFG_FILE_DIR: u16 = 0x0019;

const FW_CFG_DMA_CTL_ERROR: u32 = 1 << 0;
const FW_CFG_DMA_CTL_READ: u32 = 1 << 1;
const FW_CFG_DMA_CTL_SELECT: u32 = 1 << 3;
const FW_CFG_DMA_CTL_WRITE: u32 = 1 << 4;

const FW_CFG_FEATURE_DMA: u32 = 1 << 1;

const RAMFB_FILE_NAME: &[u8] = b"etc/ramfb";
const RAMFB_FOURCC_XRGB8888: u32 = fourcc_code(b'X', b'R', b'2', b'4');

const FRAMEBUFFER_WIDTH: u32 = 640;
const FRAMEBUFFER_HEIGHT: u32 = 480;
const FRAMEBUFFER_STRIDE_PIXELS: u32 = FRAMEBUFFER_WIDTH;
const FRAMEBUFFER_BYTES_PER_PIXEL: u8 = 4;
const FRAMEBUFFER_STRIDE_BYTES: u32 = FRAMEBUFFER_WIDTH * (FRAMEBUFFER_BYTES_PER_PIXEL as u32);
const FRAMEBUFFER_PIXELS: usize = (FRAMEBUFFER_WIDTH as usize) * (FRAMEBUFFER_HEIGHT as usize);
const FRAMEBUFFER_SIZE: usize = FRAMEBUFFER_PIXELS * (FRAMEBUFFER_BYTES_PER_PIXEL as usize);

#[repr(C, align(4096))]
struct FramebufferMemory([u32; FRAMEBUFFER_PIXELS]);

#[repr(C)]
struct FwCfgDmaAccess {
    control: u32,
    length: u32,
    address: u64,
}

static mut FRAMEBUFFER: FramebufferMemory = FramebufferMemory([0; FRAMEBUFFER_PIXELS]);

pub(crate) fn init_display() -> Result<FramebufferInfo, VgaError> {
    validate_fw_cfg()?;
    let ramfb_selector = find_ramfb_selector()?;
    let framebuffer_paddr = framebuffer_address();
    configure_ramfb(ramfb_selector, framebuffer_paddr)?;

    Ok(FramebufferInfo {
        width: FRAMEBUFFER_WIDTH,
        height: FRAMEBUFFER_HEIGHT,
        stride: FRAMEBUFFER_STRIDE_PIXELS,
        framebuffer_paddr,
        framebuffer_size: FRAMEBUFFER_SIZE,
        bytes_per_pixel: FRAMEBUFFER_BYTES_PER_PIXEL,
        pixel_format: PixelFormat::Xrgb8888,
    })
}

fn validate_fw_cfg() -> Result<(), VgaError> {
    let mut signature = [0u8; 4];
    fw_cfg_read_item(FW_CFG_SIGNATURE, &mut signature)?;
    if signature != *b"QEMU" {
        return Err(VgaError::DeviceNotReady);
    }

    let mut features = [0u8; 4];
    fw_cfg_read_item(FW_CFG_ID, &mut features)?;
    let feature_bits = u32::from_le_bytes(features);
    if feature_bits & FW_CFG_FEATURE_DMA == 0 {
        return Err(VgaError::MmioFault);
    }

    Ok(())
}

fn find_ramfb_selector() -> Result<u16, VgaError> {
    let mut count = [0u8; 4];
    fw_cfg_read_item(FW_CFG_FILE_DIR, &mut count)?;
    let file_count = u32::from_be_bytes(count);

    for _ in 0..file_count {
        let mut entry = [0u8; 64];
        fw_cfg_read_current(&mut entry)?;
        if file_name_matches(&entry[8..], RAMFB_FILE_NAME) {
            return Ok(u16::from_be_bytes([entry[4], entry[5]]));
        }
    }

    Err(VgaError::DeviceNotReady)
}

fn configure_ramfb(selector: u16, framebuffer_paddr: usize) -> Result<(), VgaError> {
    let config = build_ramfb_config(framebuffer_paddr)?;
    fw_cfg_write_item(selector, &config)
}

fn fw_cfg_read_item(selector: u16, buffer: &mut [u8]) -> Result<(), VgaError> {
    fw_cfg_dma_transfer(
        (u32::from(selector) << 16) | FW_CFG_DMA_CTL_SELECT | FW_CFG_DMA_CTL_READ,
        buffer,
    )
}

fn fw_cfg_read_current(buffer: &mut [u8]) -> Result<(), VgaError> {
    fw_cfg_dma_transfer(FW_CFG_DMA_CTL_READ, buffer)
}

fn fw_cfg_write_item(selector: u16, buffer: &[u8]) -> Result<(), VgaError> {
    fw_cfg_dma_transfer_readonly(
        (u32::from(selector) << 16) | FW_CFG_DMA_CTL_SELECT | FW_CFG_DMA_CTL_WRITE,
        buffer,
    )
}

fn fw_cfg_dma_transfer(control: u32, buffer: &mut [u8]) -> Result<(), VgaError> {
    let buffer_addr = u64::try_from(buffer.as_mut_ptr() as usize).map_err(|_| VgaError::MmioFault)?;
    let mut access = FwCfgDmaAccess {
        control: control.to_be(),
        length: u32::try_from(buffer.len()).map_err(|_| VgaError::MmioFault)?.to_be(),
        address: buffer_addr.to_be(),
    };
    fw_cfg_dma_execute(&mut access)
}

fn fw_cfg_dma_transfer_readonly(control: u32, buffer: &[u8]) -> Result<(), VgaError> {
    let buffer_addr = u64::try_from(buffer.as_ptr() as usize).map_err(|_| VgaError::MmioFault)?;
    let mut access = FwCfgDmaAccess {
        control: control.to_be(),
        length: u32::try_from(buffer.len()).map_err(|_| VgaError::MmioFault)?.to_be(),
        address: buffer_addr.to_be(),
    };
    fw_cfg_dma_execute(&mut access)
}

fn fw_cfg_dma_execute(access: &mut FwCfgDmaAccess) -> Result<(), VgaError> {
    let access_addr = u64::try_from(access as *mut FwCfgDmaAccess as usize).map_err(|_| VgaError::MmioFault)?;
    unsafe {
        write_mmio_u64(FW_CFG_DMA_REGISTER, access_addr.to_be());
    }

    loop {
        let status = u32::from_be(access.control);
        if status == 0 {
            return Ok(());
        }
        if status & FW_CFG_DMA_CTL_ERROR != 0 {
            return Err(VgaError::MmioFault);
        }
        core::hint::spin_loop();
    }
}

fn framebuffer_address() -> usize {
    unsafe { addr_of_mut!(FRAMEBUFFER.0) as *mut u32 as usize }
}

fn build_ramfb_config(framebuffer_paddr: usize) -> Result<[u8; 28], VgaError> {
    let framebuffer_paddr =
        u64::try_from(framebuffer_paddr).map_err(|_| VgaError::InvalidFramebuffer)?;
    let mut config = [0u8; 28];
    config[0..8].copy_from_slice(&framebuffer_paddr.to_be_bytes());
    config[8..12].copy_from_slice(&RAMFB_FOURCC_XRGB8888.to_be_bytes());
    config[12..16].copy_from_slice(&0u32.to_be_bytes());
    config[16..20].copy_from_slice(&FRAMEBUFFER_WIDTH.to_be_bytes());
    config[20..24].copy_from_slice(&FRAMEBUFFER_HEIGHT.to_be_bytes());
    config[24..28].copy_from_slice(&FRAMEBUFFER_STRIDE_BYTES.to_be_bytes());
    Ok(config)
}

const fn fourcc_code(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

fn file_name_matches(field: &[u8], expected: &[u8]) -> bool {
    match field.iter().position(|&byte| byte == 0) {
        Some(len) => &field[..len] == expected,
        None => field == expected,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FRAMEBUFFER_HEIGHT, FRAMEBUFFER_STRIDE_BYTES, FRAMEBUFFER_WIDTH, RAMFB_FILE_NAME,
        RAMFB_FOURCC_XRGB8888, build_ramfb_config, file_name_matches, fourcc_code,
    };

    #[test]
    fn fourcc_matches_xrgb8888() {
        assert_eq!(RAMFB_FOURCC_XRGB8888, fourcc_code(b'X', b'R', b'2', b'4'));
    }

    #[test]
    fn file_name_lookup_matches_nul_terminated_entry() {
        let mut field = [0u8; 56];
        field[..RAMFB_FILE_NAME.len()].copy_from_slice(RAMFB_FILE_NAME);
        assert!(file_name_matches(&field, RAMFB_FILE_NAME));
    }

    #[test]
    fn ramfb_geometry_is_consistent() {
        assert_eq!(FRAMEBUFFER_STRIDE_BYTES, FRAMEBUFFER_WIDTH * 4);
    }

    #[test]
    fn ramfb_config_encodes_geometry() {
        let config = build_ramfb_config(0x8020_0000).unwrap();
        assert_eq!(&config[8..12], &RAMFB_FOURCC_XRGB8888.to_be_bytes());
        assert_eq!(&config[16..20], &FRAMEBUFFER_WIDTH.to_be_bytes());
        assert_eq!(&config[20..24], &FRAMEBUFFER_HEIGHT.to_be_bytes());
        assert_eq!(&config[24..28], &FRAMEBUFFER_STRIDE_BYTES.to_be_bytes());
    }
}
