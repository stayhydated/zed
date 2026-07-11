#![cfg(any(target_os = "linux", target_os = "freebsd"))]
mod linux;

#[cfg(feature = "test-support")]
pub use linux::LinuxHeadlessRenderer;
pub use linux::current_platform;
