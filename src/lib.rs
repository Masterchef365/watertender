/// Vulkan implementation supplied by Erupt
pub use erupt::vk;

use erupt::{utils::loading::DefaultEntryLoader, DeviceLoader, InstanceLoader};
use gpu_alloc::GpuAllocator;
use std::sync::{Arc, Mutex};

pub(crate) mod app_info;
pub(crate) mod core;

#[cfg(all(feature = "openxr", feature = "winit"))]
pub(crate) mod mainloop;

#[cfg(feature = "openxr")]
pub(crate) mod openxr_backend;
#[cfg(feature = "openxr")]
pub use openxr;

#[cfg(feature = "winit")]
pub(crate) mod winit_backend;
#[cfg(feature = "winit")]
pub use winit;

#[cfg(feature = "nalgebra")]
pub use nalgebra;

mod alloc_helpers;
mod hardware_query;
pub(crate) mod shortcuts;

/// Go figure
pub const ENGINE_NAME: &str = "WaterTender";

pub use {
    app_info::AppInfo,
    mainloop::{Frame, MainLoop, Platform, PlatformEvent, PlatformReturn, SyncMainLoop},
};

/// A collection of commonly referenced Vulkan context
pub struct Core {
    /// General purpose queue, must be graphics and compute capable
    pub queue: vk::Queue,

    /// Family the queue is from
    pub queue_family: u32,

    /// GPU memory allocator
    pub allocator: Mutex<GpuAllocator<vk::DeviceMemory>>,

    /// Vulkan device
    pub device: DeviceLoader,

    /// Vulkan physical device
    pub physical_device: vk::PhysicalDevice,

    /// Information about the device
    pub device_properties: vk::PhysicalDeviceProperties,

    /// Vulkan instance
    pub instance: InstanceLoader,

    /// Erupt entry
    pub entry: DefaultEntryLoader,
}

/// An alias of `Arc<Core>`. Useful to include in subsystems for easy access to Vulkan context
pub type SharedCore = Arc<Core>;
