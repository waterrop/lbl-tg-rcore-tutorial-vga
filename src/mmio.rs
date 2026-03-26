use core::ptr::write_volatile;

pub(crate) unsafe fn write_mmio_u64(addr: usize, value: u64) {
    unsafe { write_volatile(addr as *mut u64, value) }
}

pub(crate) unsafe fn write_framebuffer_u32(addr: usize, value: u32) {
    unsafe { write_volatile(addr as *mut u32, value) }
}
