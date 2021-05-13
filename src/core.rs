use anyhow::{format_err, Result};
use erupt::vk;
use erupt::{utils::loading::DefaultEntryLoader, DeviceLoader, InstanceLoader};
use gpu_alloc::{GpuAllocator, MemoryBlock, Request};
use gpu_alloc_erupt::EruptMemoryDevice;
use std::sync::MutexGuard;
use std::sync::{Arc, Mutex};

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

type Memory = MemoryBlock<vk::DeviceMemory>;
impl Core {
    /// Memory allocator
    pub fn allocator(&self) -> Result<MutexGuard<GpuAllocator<vk::DeviceMemory>>> {
        self.allocator
            .lock()
            .map_err(|_| format_err!("GpuAllocator mutex poisoned"))
    }

    pub fn alloc(&self, request: Request) -> Result<Memory> {
        Ok(unsafe {
            self.allocator()?
                .alloc(EruptMemoryDevice::wrap(&self.device), request)?
        })
    }
}
