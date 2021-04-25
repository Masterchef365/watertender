use erupt::vk;
use crate::SharedCore;
use anyhow::Result;

/// Framebuffer manager, includes depth image and color image views
pub struct FramebufferManager {
    internals: Option<Internals>,
    core: SharedCore,
    depth_format: vk::Format,
    vr: bool,
}

impl FramebufferManager {
    pub fn new(core: SharedCore, depth_format: vk::Format, vr: bool) -> Self {
        Self {
            internals: None,
            depth_format,
            core,
            vr,
        }
    }

    pub fn frame(&self, swapchain_image_index: u32) -> vk::Framebuffer {
        let internals = self.internals.as_ref().expect("Frame called before resize");
        let frame = internals.frame.get(swapchain_image_index as usize).expect("Invalid swapchain image index");
        frame.framebuffer
    }

    pub fn resize(&mut self, extent: vk::Extent2D, images: Vec<vk::Image>) -> Result<()> {
        todo!("Make easy alloc first!")
    }

    pub fn dimensions(&self) -> vk::Extent2D {
        self.internals.as_ref().expect("Dimensions called before resize").extent
    }
}

struct Internals {
    pub extent: vk::Extent2D,
    depth_image: vk::Image,
    depth_image_view: vk::ImageView,
    frame: Vec<Frame>,
}

struct Frame {
    pub framebuffer: vk::Framebuffer,
    pub image_view: vk::ImageView,
}
