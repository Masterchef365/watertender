use crate::{memory, memory::ManagedBuffer};
use crate::SharedCore;
use anyhow::Result;
use bytemuck::Pod;
use erupt::vk;
use std::marker::PhantomData;

pub struct FrameDataUbo<T> {
    buffer: ManagedBuffer,
    padded_size: u64,
    frames: usize,
    _phantom: PhantomData<T>,
}

impl<T: Pod> FrameDataUbo<T> {
    pub fn new(core: SharedCore, frames: usize) -> Result<Self> {
        // Calculate the stride for the uniform buffer entries
        let padded_size = memory::pad_uniform_buffer_size(
            core.device_properties,
            std::mem::size_of::<T>() as u64,
        );
        let total_size = padded_size * frames as u64;

        let ci = vk::BufferCreateInfoBuilder::new()
            .size(total_size)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER);
        let buffer = ManagedBuffer::new(core, ci, memory::UsageFlags::UPLOAD)?;

        Ok(Self {
            frames,
            buffer,
            padded_size,
            _phantom: PhantomData,
        })
    }

    pub fn descriptor_buffer_info(&self, frame: usize) -> vk::DescriptorBufferInfoBuilder<'static> {
        vk::DescriptorBufferInfoBuilder::new()
            .buffer(self.buffer.buffer())
            .offset(self.offset(frame))
            .range(self.padded_size)
    }

    fn offset(&self, frame: usize) -> u64 {
        debug_assert!(frame < self.frames, "Invalid frame {}", frame);
        self.padded_size * frame as u64
    }

    pub fn upload(&mut self, frame: usize, data: &T) -> Result<()> {
        self.buffer.write_bytes(
            self.offset(frame),
            bytemuck::cast_slice(std::slice::from_ref(data)),
        )
    }
}