use crate::Core;
use anyhow::Result;
use drop_bomb::DropBomb;
use erupt::vk1_0 as vk;
pub use gpu_alloc::{Request, UsageFlags};
use gpu_alloc_erupt::EruptMemoryDevice as EMD;

/// Block of allocated device memory
pub type MemoryBlock = gpu_alloc::MemoryBlock<vk::DeviceMemory>;

// TODO: Turn this into CoreMemoryExt?
impl Core {
    /// Allocate using a gpu-alloc request
    pub fn allocate(&self, request: Request) -> Result<MemoryBlock> {
        unsafe { Ok(self.allocator()?.alloc(EMD::wrap(&self.device), request)?) }
    }

    /// Deallocate using a gpu-alloc request
    pub fn deallocate(&self, memory: MemoryBlock) -> Result<()> {
        unsafe { Ok(self.allocator()?.dealloc(EMD::wrap(&self.device), memory)) }
    }
}

/// Simple allocated vk::Image or vk::Buffer
pub struct MemObject<T> {
    instance: T,
    memory: Option<MemoryBlock>,
    bomb: DropBomb,
}

impl<T> MemObject<T> {
    pub fn memory(&self) -> &MemoryBlock {
        self.memory.as_ref().expect("Use after free")
    }

    pub fn memory_mut(&mut self) -> &mut MemoryBlock {
        self.memory.as_mut().expect("Use after free")
    }

    pub fn write_bytes(&mut self, core: &Core, offset: u64, data: &[u8]) -> Result<()> {
        Ok(unsafe {
            self.memory_mut().write_bytes(EMD::wrap(&core.device), offset, data)?;
        })
    }

    pub fn read_bytes(&mut self, core: &Core, offset: u64, data: &mut [u8]) -> Result<()> {
        Ok(unsafe {
            self.memory_mut().read_bytes(EMD::wrap(&core.device), offset, data)?;
        })
    }

    pub fn instance(&self) -> T where T: Copy {
        self.instance
    }
}

impl MemObject<vk::Image> {
    /// Allocate a new image with the given usage. Note that for the view builder, `image` does not
    /// need to be specified as this method will handle adding it.
    pub fn new_image(
        core: &Core,
        create_info: vk::ImageCreateInfoBuilder<'static>,
        usage: gpu_alloc::UsageFlags,
    ) -> Result<Self> {
        let instance = unsafe { core.device.create_image(&create_info, None, None) }.result()?;
        let memory = core.allocate(image_memory_req(&core, instance, usage))?;
        unsafe {
            core.device
                .bind_image_memory(instance, *memory.memory(), memory.offset())
                .result()?;
        }
        Ok(Self {
            instance,
            memory: Some(memory),
            bomb: DropBomb::new("Image memory object dropped without calling free()!"),
        })
    }

    pub fn free(&mut self, core: &Core) {
        unsafe {
            core.device.destroy_image(Some(self.instance), None);
            core.deallocate(self.memory.take().expect("Double free of image memory"))
                .unwrap();
            self.bomb.defuse();
        }
    }
}

impl MemObject<vk::Buffer> {
    /// Allocate a new buffer with the given usage. Note that for the view builder, `buffer` does not
    /// need to be specified as this method will handle adding it.
    pub fn new_buffer(
        core: &Core,
        create_info: vk::BufferCreateInfoBuilder<'static>,
        usage: gpu_alloc::UsageFlags,
    ) -> Result<Self> {
        let instance = unsafe { core.device.create_buffer(&create_info, None, None) }.result()?;
        let memory = core.allocate(buffer_memory_req(&core, instance, usage))?;
        unsafe {
            core.device
                .bind_buffer_memory(instance, *memory.memory(), memory.offset())
                .result()?;
        }
        Ok(Self {
            instance,
            memory: Some(memory),
            bomb: DropBomb::new("Buffer memory object dropped without calling free()!"),
        })
    }

    pub fn free(&mut self, core: &Core) {
        unsafe {
            core.device.destroy_buffer(Some(self.instance), None);
            core.deallocate(self.memory.take().expect("Double free of buffer memory"))
                .unwrap();
            self.bomb.defuse();
        }
    }
}

/// Calculate image memory requirements for gpu_alloc
pub fn image_memory_req(core: &Core, image: vk::Image, usage: UsageFlags) -> Request {
    request_from_usage_requirements(
        unsafe { core.device.get_image_memory_requirements(image, None) },
        usage,
    )
}

/// Calculate buffer memory requirements for gpu_alloc
pub fn buffer_memory_req(core: &Core, buffer: vk::Buffer, usage: UsageFlags) -> Request {
    request_from_usage_requirements(
        unsafe { core.device.get_buffer_memory_requirements(buffer, None) },
        usage,
    )
}

/// Create a request from memory requirements and usage
pub fn request_from_usage_requirements(
    requirements: vk::MemoryRequirements,
    usage: UsageFlags,
) -> Request {
    Request {
        size: requirements.size,
        align_mask: requirements.alignment,
        usage,
        memory_types: requirements.memory_type_bits,
    }
}
