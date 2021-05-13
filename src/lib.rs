pub mod app_info;
pub mod core;
pub mod hardware_query;

/// Vulkan implementation supplied by Erupt
pub use erupt::vk;

#[cfg(feature = "openxr")]
pub mod openxr_backend;
#[cfg(feature = "openxr")]
pub use openxr;

#[cfg(feature = "winit")]
pub mod winit_backend;
#[cfg(feature = "winit")]
pub use winit;

/// Mainloop abstraction
#[cfg(any(feature = "openxr", feature = "winit"))]
pub mod mainloop;

#[cfg(feature = "nalgebra")]
pub use nalgebra;

pub mod shortcuts;

/// Go figure
pub const ENGINE_NAME: &str = "WaterTender";

pub mod prelude {
    pub use super::*;
    #[cfg(all(feature = "openxr", feature = "winit"))]
    pub use mainloop::{
        Frame, MainLoop, Platform, PlatformEvent, PlatformReturn, SyncMainLoop,
    };
    pub use {app_info::AppInfo, hardware_query::HardwareSelection};
    pub use shortcuts::memory::{MemoryBlock, ManagedImage, ManagedBuffer, UsageFlags};
    pub use {SharedCore, Core};
}

pub use crate::core::{Core, SharedCore};
