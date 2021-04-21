/// Vulkan implementation supplied by Erupt
pub use erupt::vk;
use erupt::{utils::loading::DefaultEntryLoader, DeviceLoader, InstanceLoader};
use anyhow::{Result, format_err};
use std::sync::{Arc, Mutex, MutexGuard};
use gpu_alloc::GpuAllocator;

#[cfg(feature = "openxr")]
mod openxr_backend;

#[cfg(feature = "winit")]
mod winit_backend;

/// All mainloops run on executors must implement this trait
trait MainLoop: Sized {
    fn new(init_cmds: vk::CommandBuffer, core: &Core, platform: Platform<'_>) -> Result<Self>;
    fn event(&mut self, event: PlatformEvent<'_>, core: &Core, platform: Platform<'_>) -> Result<()>;
    fn frame(&mut self, frame: Frame, core: &Core, platform: Platform<'_>) -> Result<()>;
}

/// Interface to the gpu's commands
pub struct Frame {
    /// Which in-flight frame this is
    pub index: usize,
    pub serial_cmds: vk::CommandBuffer,
    pub frame_cmds: vk::CommandBuffer,
    pub framebuffer: vk::Framebuffer,
    pub extent: vk::Extent2D,
}

/// An alias of `Arc<Core>`. Useful to include in subsystems for easy access to Vulkan context
pub type SharedCore = Arc<Core>;

/// A collection of commonly referenced Vulkan context
pub struct Core {
    /// The utility queue should be compute-capable. Note that it should share a queue family with
    /// the graphics queue!
    pub utility_queue: vk::Queue,

    /// The graphics queue should be graphics-capable
    pub graphics_queue: vk::Queue,

    /// GPU memory allocator
    pub allocator: Mutex<GpuAllocator<vk::DeviceMemory>>,

    /// Vulkan device
    pub device: DeviceLoader,

    /// Vulkan instance
    pub instance: InstanceLoader,

    /// Erupt entry
    pub entry: DefaultEntryLoader,
}

impl Core {
    /// Memory allocator
    pub fn allocator(&self) -> Result<MutexGuard<GpuAllocator<vk::DeviceMemory>>> {
        self.allocator
            .lock()
            .map_err(|_| format_err!("GpuAllocator mutex poisoned"))
    }
}

/// Multiplatform event
pub enum PlatformEvent<'a> {
    #[cfg(feature = "winit")]
    Winit(winit::event::Event<'a, ()>),
    #[cfg(feature = "openxr")]
    OpenXr(openxr::Event<'a>),
}

/// Multiplatform event
pub enum Platform<'a> {
    #[cfg(feature = "winit")]
    Winit {
        window: &'a winit::window::Window,
        flow: &'a mut winit::event_loop::ControlFlow,
    },
    #[cfg(feature = "openxr")]
    OpenXr {
        xr_core: &'a openxr_backend::XrCore,
    },
}

// TODO: Re-exported stuff from other files
pub const FRAMES_IN_FLIGHT: usize = 3;
