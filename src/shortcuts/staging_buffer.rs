use crate::shortcuts::{memory::UsageFlags, ManagedBuffer, ManagedImage};
use crate::SharedCore;
use anyhow::Result;
use bytemuck::Pod;
use erupt::vk;

pub struct StagingBuffer {
    buffer: ManagedBuffer,
    current_size: u64,
    // TODO: Storing this here is sort of wasteful?
    core: SharedCore,
}

impl StagingBuffer {
    pub fn new(core: SharedCore) -> Result<Self> {
        let current_size = 1024 * 1024; // 1 MB
        Ok(Self {
            buffer: Self::build_staging_buffer(core.clone(), current_size)?,
            current_size,
            core,
        })
    }

    // TODO: Make a batched upload option? (So that you don't have to do a million queue idles...
    // TODO: This should also probably use a transfer queue...
    // TODO: Multi-part uploads for BIG data?
    /// Warning: Assumes an inactive command buffer
    pub fn upload_buffer<T: Pod>(
        &mut self,
        command_buffer: vk::CommandBuffer,
        mut ci: vk::BufferCreateInfoBuilder<'static>,
        data: &[T],
    ) -> Result<ManagedBuffer> {
        // Expand our internal buffer to match the size of the data to be uploaded
        if ci.size > self.current_size {
            self.current_size = ci.size;
            self.buffer = Self::build_staging_buffer(self.core.clone(), self.current_size)?;
        }

        // Write to the staging buffer
        self.buffer.write_bytes(0, bytemuck::cast_slice(data))?;

        // Create the final buffer
        ci.usage |= vk::BufferUsageFlags::TRANSFER_DST;
        let gpu_buffer = ManagedBuffer::new(self.core.clone(), ci, UsageFlags::FAST_DEVICE_ACCESS)?;

        // Upload to this new buffer
        unsafe {
            self.core
                .device
                .reset_command_buffer(command_buffer, None)
                .result()?;
            let begin_info = vk::CommandBufferBeginInfoBuilder::new();
            self.core
                .device
                .begin_command_buffer(command_buffer, &begin_info)
                .result()?;

            let region = vk::BufferCopyBuilder::new()
                .size(ci.size)
                .src_offset(0)
                .dst_offset(0);

            self.core.device.cmd_copy_buffer(
                command_buffer,
                self.buffer.instance(),
                gpu_buffer.instance(),
                &[region],
            );

            self.core
                .device
                .end_command_buffer(command_buffer)
                .result()?;
            let command_buffers = [command_buffer];
            let submit_info = vk::SubmitInfoBuilder::new().command_buffers(&command_buffers);
            self.core
                .device
                .queue_submit(self.core.queue, &[submit_info], None)
                .result()?;
            self.core.device.queue_wait_idle(self.core.queue).result()?;
        }

        Ok(gpu_buffer)
    }

    /// Warning: Assumes an inactive command buffer
    pub fn upload_image(
        &mut self,
        command_buffer: vk::CommandBuffer,
        width: u32,
        height: u32,
        data: &[u8],
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        final_layout: vk::ImageLayout,
    ) -> Result<(ManagedImage, vk::ImageSubresourceRangeBuilder<'static>)> {
        // Image settings
        let extent = vk::Extent3DBuilder::new()
            .width(width)
            .height(height)
            .depth(1)
            .build();

        let ci = vk::ImageCreateInfoBuilder::new()
            .image_type(vk::ImageType::_2D)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlagBits::_1);

        let offset = vk::Offset3DBuilder::new().x(0).y(0).z(0).build();

        let image_subresources = vk::ImageSubresourceLayersBuilder::new()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1)
            .build();

        let subresource_range = vk::ImageSubresourceRangeBuilder::new()
            .aspect_mask(image_subresources.aspect_mask)
            .base_mip_level(image_subresources.mip_level)
            .level_count(1)
            .base_array_layer(image_subresources.base_array_layer)
            .layer_count(image_subresources.layer_count);

        let copy = vk::BufferImageCopyBuilder::new()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(image_subresources)
            .image_offset(offset)
            .image_extent(extent);

        // Expand our internal buffer to match the size of the data to be uploaded
        if data.len() as u64 > self.current_size {
            self.current_size = data.len() as u64;
            self.buffer = Self::build_staging_buffer(self.core.clone(), self.current_size)?;
        }

        // Write to the staging buffer
        self.buffer.write_bytes(0, bytemuck::cast_slice(data))?;

        // Create the final buffer
        let gpu_image = ManagedImage::new(self.core.clone(), ci, UsageFlags::FAST_DEVICE_ACCESS)?;

        // NOTE: image_layout must be one of VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, VK_IMAGE_LAYOUT_GENERAL, or VK_IMAGE_LAYOUT_SHARED_PRESENT_KHR
        // Refer to: https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/vkCmdCopyBufferToImage.html
        let image_layout = vk::ImageLayout::GENERAL; // TODO: Add an enum for some common modes? (like DST_OPTIMAL)

        // Upload to this new buffer
        unsafe {
            self.core
                .device
                .reset_command_buffer(command_buffer, None)
                .result()?;
            let begin_info = vk::CommandBufferBeginInfoBuilder::new();
            self.core
                .device
                .begin_command_buffer(command_buffer, &begin_info)
                .result()?;

            let barrier = vk::ImageMemoryBarrierBuilder::new()
                .image(gpu_image.instance())
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(image_layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .subresource_range(subresource_range.build());

            self.core.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                None,
                &[],
                &[],
                &[barrier],
            );

            self.core.device.cmd_copy_buffer_to_image(
                command_buffer,
                self.buffer.instance(),
                gpu_image.instance(),
                image_layout,
                &[copy],
            );

            let barrier = vk::ImageMemoryBarrierBuilder::new()
                .image(gpu_image.instance())
                .old_layout(image_layout)
                .new_layout(final_layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::empty())
                .subresource_range(subresource_range.build());

            self.core.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                None,
                &[],
                &[],
                &[barrier],
            );

            self.core
                .device
                .end_command_buffer(command_buffer)
                .result()?;
            let command_buffers = [command_buffer];
            let submit_info = vk::SubmitInfoBuilder::new().command_buffers(&command_buffers);
            self.core
                .device
                .queue_submit(self.core.queue, &[submit_info], None)
                .result()?;
            self.core.device.queue_wait_idle(self.core.queue).result()?;
        }

        Ok((gpu_image, subresource_range))
    }

    fn build_staging_buffer(core: SharedCore, size: u64) -> Result<ManagedBuffer> {
        let ci = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&[])
            .size(size);
        ManagedBuffer::new(core.clone(), ci, UsageFlags::UPLOAD)
    }
}
