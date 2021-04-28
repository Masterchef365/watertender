use erupt::vk;
use crate::SharedCore;
use crate::shortcuts::ManagedBuffer;
use std::marker::PhantomData;
use anyhow::Result;
use bytemuck::Pod;

pub struct FrameDataUbo<T> {
    buffer: ManagedBuffer,
    core: SharedCore,
    _phantom: PhantomData<T>,
}

impl<T: Pod> FrameDataUbo<T> {
    pub fn new(core: SharedCore, frames: usize) -> Result<Self> {
        todo!()
    }

    pub fn descriptor_set(&self, frame: usize) -> vk::DescriptorSet {
        todo!()
    }

    pub fn upload(&mut self, frame: usize, data: &T) -> Result<()> {
        todo!()
    }
}
