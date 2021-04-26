use crate::Core;
use anyhow::{format_err, Result};
use erupt::vk;
use gpu_alloc::{GpuAllocator, MemoryBlock, Request};
use gpu_alloc_erupt::EruptMemoryDevice;
use std::sync::MutexGuard;

pub type Memory = MemoryBlock<vk::DeviceMemory>;

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
