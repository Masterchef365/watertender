use crate::{Core, SharedCore};
use anyhow::Result;
use erupt::vk1_0 as vk;
pub use gpu_alloc::{Request, UsageFlags};
use gpu_alloc_erupt::EruptMemoryDevice as EMD;

/// Block of allocated device memory
pub type MemoryBlock = gpu_alloc::MemoryBlock<vk::DeviceMemory>;

// TODO: Turn this into a trait such as CoreMemoryExt? It's even got a cool name...
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

/// Image with associated memory, deallocates on drop. Best not to keep huge arrays of these; they
/// waste memory.
pub struct ManagedImage {
    instance: vk::Image,
    memory: Option<MemoryBlock>,
    core: SharedCore,
}

/// Buffer with associated memory, deallocates on drop. Best not to keep huge arrays of these; they
/// waste memory.
pub struct ManagedBuffer {
    instance: vk::Buffer,
    pub memory: Option<MemoryBlock>,
    pub core: SharedCore,
}

const USE_AFTER_FREE_MSG: &str = "Use-after-free!";

impl ManagedBuffer {
    /// Allocate a new buffer with the given usage. Note that for the view builder, `buffer` does not
    /// need to be specified as this method will handle adding it.
    pub fn new(
        core: SharedCore,
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
            core,
        })
    }

    pub fn write_bytes(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        Ok(unsafe {
            self.memory
                .as_mut()
                .expect(USE_AFTER_FREE_MSG)
                .write_bytes(EMD::wrap(&self.core.device), offset, data)?;
        })
    }

    pub fn read_bytes(&mut self, offset: u64, data: &mut [u8]) -> Result<()> {
        Ok(unsafe {
            self.memory.as_mut().expect(USE_AFTER_FREE_MSG).read_bytes(
                EMD::wrap(&self.core.device),
                offset,
                data,
            )?;
        })
    }

    pub fn instance(&self) -> vk::Buffer {
        self.instance
    }
}

impl ManagedImage {
    /// Allocate a new image with the given usage. Note that for the view builder, `image` does not
    /// need to be specified as this method will handle adding it.
    pub fn new(
        core: SharedCore,
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
            core,
            instance,
            memory: Some(memory),
        })
    }

    pub fn write_bytes(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        Ok(unsafe {
            self.memory
                .as_mut()
                .expect(USE_AFTER_FREE_MSG)
                .write_bytes(EMD::wrap(&self.core.device), offset, data)?;
        })
    }

    pub fn read_bytes(&mut self, offset: u64, data: &mut [u8]) -> Result<()> {
        Ok(unsafe {
            self.memory.as_mut().expect(USE_AFTER_FREE_MSG).read_bytes(
                EMD::wrap(&self.core.device),
                offset,
                data,
            )?;
        })
    }

    pub fn instance(&self) -> vk::Image {
        self.instance
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

impl Drop for ManagedImage {
    fn drop(&mut self) {
        unsafe {
            self.core.device.destroy_image(Some(self.instance), None);
            self.core
                .deallocate(self.memory.take().expect("Double free of image memory"))
                .unwrap();
        }
    }
}

impl Drop for ManagedBuffer {
    fn drop(&mut self) {
        unsafe {
            self.core.device.queue_wait_idle(self.core.queue).unwrap(); // TODO: Drop without queue wait?
            self.core.device.destroy_buffer(Some(self.instance), None);
            self.core
                .deallocate(self.memory.take().expect("Double free of image memory"))
                .unwrap();
        }
    }
}

// Credit: https://github.com/SaschaWillems/Vulkan/tree/master/examples/dynamicuniformbuffer
pub fn pad_uniform_buffer_size(device_properties: vk::PhysicalDeviceProperties, size: u64) -> u64 {
    let min_align = device_properties.limits.min_uniform_buffer_offset_alignment;
    pad_size(min_align, size)
}

pub fn pad_size(min_align: u64, size: u64) -> u64 {
    if min_align > 0 {
        (size + min_align - 1) & !(min_align - 1)
    } else {
        size
    }
}
