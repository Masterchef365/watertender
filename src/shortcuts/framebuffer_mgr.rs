use erupt::vk;
use crate::SharedCore;
use anyhow::Result;

/// Framebuffer manager, includes depth image and color image views
pub struct FramebufferManager {
    internals: Option<Internals>,
    core: SharedCore,
    depth_format: vk::Format,
}

impl FramebufferManager {
    pub fn new(core: SharedCore, depth_format: vk::Format) -> Self {
        Self {
            internals: None,
            depth_format,
            core,
        }
    }

    pub fn frame(swapchain_image_index: u32) -> vk::Image {
        todo!()
    }

    pub fn resize(&mut self, extent: vk::Extent2D, images: Vec<vk::Image>) -> Result<()> {
        todo!()
    }

    pub fn dimensions(&self) -> vk::Extent2D {
        self.internals.as_ref().expect("Dimensions called before resize").extent
    }
}

struct Internals {
    extent: vk::Extent2D,
}

struct Frame {
    pub framebuffer: vk::Framebuffer,
    pub image_view: vk::ImageView,
}


#[derive(Copy, Clone)]
pub struct SwapChainImage {
    pub extent: vk::Extent2D,
    pub in_flight: vk::Fence,
}

