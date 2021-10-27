use crate::{
    defaults::{COLOR_FORMAT, DEPTH_FORMAT},
    memory::ManagedImage,
};
use crate::{Core, SharedCore};
use anyhow::Result;
use erupt::vk;
use gpu_alloc::UsageFlags;

/// Framebuffer manager, includes depth image and color image views
pub struct FramebufferManager {
    internals: Option<Internals>,
    core: SharedCore,
    msaa_samples: vk::SampleCountFlagBits,
    vr: bool,
}

struct Internals {
    pub extent: vk::Extent2D,
    _depth_image: ManagedImage,
    depth_image_view: vk::ImageView,
    _color_image: ManagedImage,
    color_image_view: vk::ImageView,
    frames: Vec<Frame>,
}

struct Frame {
    pub framebuffer: vk::Framebuffer,
    pub swapchain_image_view: vk::ImageView,
}

/// Return the largest supported sample count flag up to and including `samples`
pub fn max_samples(core: &Core, samples: u16) -> vk::SampleCountFlagBits {
    let counts = core.device_properties.limits.framebuffer_color_sample_counts 
        & core.device_properties.limits.framebuffer_depth_sample_counts;

    let preferences = [
        (vk::SampleCountFlagBits::_64, 64),
        (vk::SampleCountFlagBits::_32, 32),
        (vk::SampleCountFlagBits::_16, 16),
        (vk::SampleCountFlagBits::_8, 8),
        (vk::SampleCountFlagBits::_4, 4),
        (vk::SampleCountFlagBits::_2, 2),
    ];

    for &(vk, sm) in &preferences {
        if sm <= samples && counts & vk.bitmask() != vk::SampleCountFlags::empty() {
            return vk;
        }
    }

    return vk::SampleCountFlagBits::_1;
}

impl FramebufferManager {
    /// Create a new framebuffer manager. NOTE: msaa_samples is assumed to be valid for this
    /// device. Please check core.
    pub fn new(core: SharedCore, vr: bool, msaa_samples: vk::SampleCountFlagBits) -> Self {
        Self {
            internals: None,
            msaa_samples,
            core,
            vr,
        }
    }

    pub fn frame(&self, swapchain_image_index: u32) -> vk::Framebuffer {
        let internals = self.internals.as_ref().expect("Frame called before resize");
        let frame = internals
            .frames
            .get(swapchain_image_index as usize)
            .expect("Invalid swapchain image index");
        frame.framebuffer
    }

    pub fn resize(
        &mut self,
        swapchain_images: Vec<vk::Image>,
        extent: vk::Extent2D,
        render_pass: vk::RenderPass,
    ) -> Result<()> {
        let layers = if self.vr { 2 } else { 1 };

        unsafe {
            self.core.device.queue_wait_idle(self.core.queue).result()?;
        }

        if let Some(internals) = self.internals.take() {
            internals.free(&self.core);
        }

        // Create depth image
        let create_info = vk::ImageCreateInfoBuilder::new()
            .image_type(vk::ImageType::_2D)
            .extent(
                vk::Extent3DBuilder::new()
                    .width(extent.width)
                    .height(extent.height)
                    .depth(1)
                    .build(),
            )
            .mip_levels(1)
            .array_layers(layers)
            .format(DEPTH_FORMAT)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .samples(self.msaa_samples)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let depth_image = ManagedImage::new(
            self.core.clone(),
            create_info,
            UsageFlags::FAST_DEVICE_ACCESS,
        )?;

        let create_info = vk::ImageViewCreateInfoBuilder::new()
            .image(depth_image.instance())
            .view_type(vk::ImageViewType::_2D)
            .format(DEPTH_FORMAT)
            .subresource_range(
                vk::ImageSubresourceRangeBuilder::new()
                    .aspect_mask(vk::ImageAspectFlags::DEPTH)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(layers)
                    .build(),
            );
        let depth_image_view =
            unsafe { self.core.device.create_image_view(&create_info, None, None) }.result()?;

        // Create color image
        let create_info = vk::ImageCreateInfoBuilder::new()
            .image_type(vk::ImageType::_2D)
            .extent(
                vk::Extent3DBuilder::new()
                    .width(extent.width)
                    .height(extent.height)
                    .depth(1)
                    .build(),
            )
            .mip_levels(1)
            .array_layers(layers)
            .format(COLOR_FORMAT)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT)
            .samples(self.msaa_samples)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let color_image = ManagedImage::new(
            self.core.clone(),
            create_info,
            UsageFlags::FAST_DEVICE_ACCESS,
        )?;

        let create_info = vk::ImageViewCreateInfoBuilder::new()
            .image(color_image.instance())
            .view_type(vk::ImageViewType::_2D)
            .format(COLOR_FORMAT)
            .subresource_range(
                vk::ImageSubresourceRangeBuilder::new()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(layers)
                    .build(),
            );
        let color_image_view =
            unsafe { self.core.device.create_image_view(&create_info, None, None) }.result()?;


        // Build swapchain image views and buffers
        let frames = swapchain_images
            .iter()
            .map(|&swapchain_image| {
                // Create swapchain images views
                let create_info = vk::ImageViewCreateInfoBuilder::new()
                    .image(swapchain_image)
                    .view_type(vk::ImageViewType::_2D)
                    .format(COLOR_FORMAT)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(
                        vk::ImageSubresourceRangeBuilder::new()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(layers)
                            .build(),
                    );

                let swapchain_image_view =
                    unsafe { self.core.device.create_image_view(&create_info, None, None) }
                        .result()?;

                let attachments = [color_image_view, depth_image_view, swapchain_image_view];
                let create_info = vk::FramebufferCreateInfoBuilder::new()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1);

                let framebuffer = unsafe {
                    self.core
                        .device
                        .create_framebuffer(&create_info, None, None)
                }
                .result()?;

                Ok(Frame {
                    framebuffer,
                    swapchain_image_view,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        self.internals = Some(Internals {
            _depth_image: depth_image,
            depth_image_view,
            _color_image: color_image,
            color_image_view,
            extent,
            frames,
        });

        Ok(())
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.internals
            .as_ref()
            .expect("Dimensions called before resize")
            .extent
    }
}

impl Drop for FramebufferManager {
    fn drop(&mut self) {
        if let Some(internals) = self.internals.take() {
            internals.free(&self.core);
        }
    }
}

impl Internals {
    fn free(mut self, core: &Core) {
        unsafe {
            core.device.device_wait_idle().result().unwrap();
            for frame in self.frames.drain(..) {
                core.device
                    .destroy_framebuffer(Some(frame.framebuffer), None);
                core.device.destroy_image_view(Some(frame.swapchain_image_view), None);
            }
            core.device
                .destroy_image_view(Some(self.color_image_view), None);
            core.device
                .destroy_image_view(Some(self.depth_image_view), None);
        }
    }
}
