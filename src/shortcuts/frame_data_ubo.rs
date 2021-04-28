use crate::shortcuts::{memory, ManagedBuffer};
use crate::SharedCore;
use anyhow::Result;
use bytemuck::Pod;
use erupt::vk;
use std::marker::PhantomData;

pub struct FrameDataUbo<T> {
    buffer: ManagedBuffer,
    core: SharedCore,
    padded_size: u64,
    descriptor_sets: Vec<vk::DescriptorSet>,
    descriptor_pool: vk::DescriptorPool,
    _phantom: PhantomData<T>,
}

impl<T: Pod> FrameDataUbo<T> {
    pub fn new(
        core: SharedCore,
        frames: usize,
        descriptor_set_layout: vk::DescriptorSetLayout,
        binding: u32,
    ) -> Result<Self> {
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
        let buffer = ManagedBuffer::new(core.clone(), ci, memory::UsageFlags::UPLOAD)?;

        // Create descriptor pool
        let pool_sizes = [vk::DescriptorPoolSizeBuilder::new()
            ._type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(frames as u32)];

        let create_info = vk::DescriptorPoolCreateInfoBuilder::new()
            .pool_sizes(&pool_sizes)
            .max_sets(frames as u32);

        let descriptor_pool =
            unsafe { core.device.create_descriptor_pool(&create_info, None, None) }.result()?;

        // Create descriptor sets
        let layouts = vec![descriptor_set_layout; frames];
        let create_info = vk::DescriptorSetAllocateInfoBuilder::new()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets =
            unsafe { core.device.allocate_descriptor_sets(&create_info) }.result()?;

        // Write descriptor sets
        let buffer_infos: Vec<_> = (0..frames)
            .map(|frame| {
                [vk::DescriptorBufferInfoBuilder::new()
                    .buffer(buffer.instance())
                    .offset(padded_size * frame as u64)
                    .range(padded_size)]
            })
            .collect();

        let writes: Vec<_> = buffer_infos
            .iter()
            .zip(descriptor_sets.iter())
            .map(|(info, &descriptor_set)| {
                vk::WriteDescriptorSetBuilder::new()
                    .buffer_info(info)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .dst_set(descriptor_set)
                    .dst_binding(binding)
                    .dst_array_element(0)
            })
            .collect();

        unsafe {
            core.device.update_descriptor_sets(&writes, &[]);
        }

        Ok(Self {
            core,
            buffer,
            padded_size,
            descriptor_pool,
            descriptor_sets,
            _phantom: PhantomData,
        })
    }

    pub fn descriptor_set(&self, frame: usize) -> vk::DescriptorSet {
        self.descriptor_sets[frame]
    }

    pub fn upload(&mut self, frame: usize, data: &T) -> Result<()> {
        self.buffer.write_bytes(
            frame as u64 * self.padded_size,
            bytemuck::cast_slice(std::slice::from_ref(data)),
        )
    }
}

impl<T> Drop for FrameDataUbo<T> {
    fn drop(&mut self) {
        unsafe {
            self.core
                .device
                .destroy_descriptor_pool(Some(self.descriptor_pool), None);
        }
    }
}
