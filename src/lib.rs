#![no_std]

mod device;
mod error;
mod framebuffer;
mod mmio;

pub use error::VgaError;
pub use framebuffer::{FramebufferInfo, PixelFormat};

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

const STATE_UNINITIALIZED: u8 = 0;
const STATE_INITIALIZING: u8 = 1;
const STATE_INITIALIZED: u8 = 2;

struct DisplayState {
    init_state: AtomicU8,
    info: UnsafeCell<MaybeUninit<FramebufferInfo>>,
}

unsafe impl Sync for DisplayState {}

static DISPLAY_STATE: DisplayState = DisplayState {
    init_state: AtomicU8::new(STATE_UNINITIALIZED),
    info: UnsafeCell::new(MaybeUninit::uninit()),
};

pub fn init() -> Result<(), VgaError> {
    match DISPLAY_STATE.init_state.compare_exchange(
        STATE_UNINITIALIZED,
        STATE_INITIALIZING,
        Ordering::AcqRel,
        Ordering::Acquire,
    ) {
        Ok(_) => {}
        Err(STATE_INITIALIZING | STATE_INITIALIZED) => return Err(VgaError::AlreadyInitialized),
        Err(_) => return Err(VgaError::MmioFault),
    }

    let info = match device::init_display() {
        Ok(info) => info,
        Err(error) => {
            DISPLAY_STATE
                .init_state
                .store(STATE_UNINITIALIZED, Ordering::Release);
            return Err(error);
        }
    };

    if let Err(error) = framebuffer::clear_screen(&info, 0x0000_0000) {
        DISPLAY_STATE
            .init_state
            .store(STATE_UNINITIALIZED, Ordering::Release);
        return Err(error);
    }

    unsafe {
        (*DISPLAY_STATE.info.get()).write(info);
    }
    DISPLAY_STATE
        .init_state
        .store(STATE_INITIALIZED, Ordering::Release);
    Ok(())
}

pub fn draw_pixel(x: u32, y: u32, color: u32) -> Result<(), VgaError> {
    with_framebuffer_info(|info| framebuffer::draw_pixel(info, x, y, color))
}

pub fn clear_screen(color: u32) -> Result<(), VgaError> {
    with_framebuffer_info(|info| framebuffer::clear_screen(info, color))
}

pub fn resolution() -> Result<(u32, u32), VgaError> {
    with_framebuffer_info(|info| Ok((info.width, info.height)))
}

pub fn framebuffer_info() -> Result<FramebufferInfo, VgaError> {
    with_framebuffer_info(|info| Ok(*info))
}

fn with_framebuffer_info<T>(
    f: impl FnOnce(&FramebufferInfo) -> Result<T, VgaError>,
) -> Result<T, VgaError> {
    if DISPLAY_STATE.init_state.load(Ordering::Acquire) != STATE_INITIALIZED {
        return Err(VgaError::NotInitialized);
    }

    let info = unsafe { (*DISPLAY_STATE.info.get()).assume_init_ref() };
    f(info)
}
