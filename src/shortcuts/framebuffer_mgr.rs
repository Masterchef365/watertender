use erupt::vk;
use crate::SharedCore;
use anyhow::Result;

/// Framebuffer manager, includes depth image and color image views
pub struct FramebufferManager {
    internals: Option<Internals>,
    core: SharedCore,
    vr: bool,
}

impl FramebufferManager {
    pub fn new(core: SharedCore, vr: bool) -> Self {
        Self {
            internals: None,
            core,
            vr,
        }
    }

    pub fn frame(&self, swapchain_image_index: u32) -> vk::Framebuffer {
        let internals = self.internals.as_ref().expect("Frame called before resize");
        let frame = internals.frame.get(swapchain_image_index as usize).expect("Invalid swapchain image index");
        frame.framebuffer
    }

    pub fn resize(&mut self, images: Vec<vk::Image>, extent: vk::Extent2D, render_pass: vk::RenderPass) -> Result<()> {
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
