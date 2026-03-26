#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VgaError {
    DeviceNotReady,
    InvalidResolution,
    InvalidPixelPosition,
    InvalidFramebuffer,
    UnsupportedFormat,
    MmioFault,
    NotInitialized,
    AlreadyInitialized,
}
