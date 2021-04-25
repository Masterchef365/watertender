use crate::SharedCore;
use anyhow::Result;
use erupt::vk;
use std::collections::HashMap;

/// Basic frmame/swapchain synchronization utility
pub struct Synchronization {
    in_flight_fences: Vec<vk::Fence>,
    swapchain_sync: Vec<(vk::Semaphore, vk::Semaphore)>,
    swapchain_img_lut: HashMap<u32, vk::Fence>, // Mapping from swapchain image to
    core: SharedCore,
}

impl Synchronization {
    /// Create a new synchronization shortcut. If khr_sync is specified, semaphores will be created
    /// to synchronize with a swapchain.
    pub fn new(core: SharedCore, frames_in_flight: usize, khr_sync: bool) -> Result<Self> {
        let mut swapchain_sync = Vec::new();
        let mut in_flight_fences = Vec::new();

        for _ in 0..frames_in_flight {
            unsafe {
                let create_info =
                    vk::FenceCreateInfoBuilder::new().flags(vk::FenceCreateFlags::SIGNALED);
                let fence = core
                    .device
                    .create_fence(&create_info, None, None)
                    .result()?;
                in_flight_fences.push(fence);
            }

            if khr_sync {
                let create_info = vk::SemaphoreCreateInfoBuilder::new();
                unsafe {
                    let image_available = core
                        .device
                        .create_semaphore(&create_info, None, None)
                        .result()?;
                    let render_finished = core
                        .device
                        .create_semaphore(&create_info, None, None)
                        .result()?;
                    swapchain_sync.push((image_available, render_finished));
                }
            }
        }

        Ok(Self {
            in_flight_fences,
            swapchain_sync,
            swapchain_img_lut: Default::default(),
            core,
        })
    }

    /// Synchronize with per-frame gpu resources and swapchain frame. Blocks if a needed GPU
    /// resources is unavailable. Returns a fence that must be signalled when the corresponding
    /// frame is complete.
    pub fn sync(&mut self, swapchain_image_index: u32, frame: usize) -> Result<vk::Fence> {
        // Ensure this swapchain image is not already in use by the GPU
        if let Some(&fence) = self.swapchain_img_lut.get(&swapchain_image_index) {
            unsafe {
                self.core
                    .device
                    .wait_for_fences(&[fence], false, u64::MAX)
                    .result()?;
            }
        }

        // Ensure this frame is not already in use by the GPU
        let fence = self.in_flight_fences[frame];
        unsafe {
            self.core
                .device
                .wait_for_fences(&[fence], false, u64::MAX)
                .result()?;
            self.core.device.reset_fences(&[fence]).result()?; // TODO: Move this into the swapchain next_image
        }
        self.swapchain_img_lut.insert(swapchain_image_index, fence);
        Ok(fence)
    }

    /// Swapchain sync components. May be used as a direct return from `winit_sync()` from
    /// `WinitMainLoop`.
    pub fn swapchain_sync(&self, frame: usize) -> Option<(vk::Semaphore, vk::Semaphore)> {
        self.swapchain_sync.get(frame).copied()
    }
}

impl Drop for Synchronization {
    fn drop(&mut self) {
        for (i, r) in self.swapchain_sync.drain(..) {
            unsafe {
                self.core
                    .device
                    .destroy_semaphore(Some(i), None);
                self.core
                    .device
                    .destroy_semaphore(Some(r), None);
            }
        }

        for fence in self.in_flight_fences.drain(..) {
            unsafe {
                self.core
                    .device
                    .destroy_fence(Some(fence), None);
            }
        }
    }
}
