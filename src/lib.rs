use anyhow::{format_err, Result};
/// Vulkan implementation supplied by Erupt
pub use erupt::vk;
use erupt::{utils::loading::DefaultEntryLoader, DeviceLoader, InstanceLoader};
use gpu_alloc::GpuAllocator;
use std::sync::{Arc, Mutex};
use std::ffi::CString;

#[cfg(feature = "openxr")]
mod openxr_backend;

#[cfg(feature = "winit")]
mod winit_backend;

mod hardware_query;
mod swapchain_images;
mod alloc_helpers;

/// All mainloops run on executors must implement this trait
pub trait MainLoop: Sized {
    /// Creates a new instance of your app. Mainly useful for setting up data structures and
    /// allocating memory.
    fn new(init_cmds: vk::CommandBuffer, core: &Core, platform: Platform<'_>) -> Result<Self>;

    /// A frame handled by your app. The command buffers in `frame` are already reset and have begun, and will be ended and submitted.
    fn frame(&mut self, frame: Frame, core: &Core, platform: Platform<'_>) -> Result<()>;

    /// Handle an event produced by the Platform
    fn event(
        &mut self,
        event: PlatformEvent<'_>,
        core: &Core,
        platform: Platform<'_>,
    ) -> Result<()>;
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

/// Multi-platform event
pub enum PlatformEvent<'a> {
    #[cfg(feature = "winit")]
    Winit(winit::event::Event<'a, ()>),
    #[cfg(feature = "openxr")]
    OpenXr(openxr::Event<'a>),
}

/// Multi-platform
pub enum Platform<'a> {
    #[cfg(feature = "winit")]
    Winit {
        window: &'a winit::window::Window,
        flow: &'a mut winit::event_loop::ControlFlow,
    },
    #[cfg(feature = "openxr")]
    OpenXr { xr_core: &'a openxr_backend::XrCore },
}

pub const ENGINE_NAME: &str = "WaterTender";
pub const FRAMES_IN_FLIGHT: usize = 2;
pub const COLOR_FORMAT: vk::Format = vk::Format::B8G8R8A8_SRGB;
pub const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;

/// Application info
pub struct AppInfo {
    name: CString,
    version: u32,
    api_version: u32,
    validation: bool,
}

impl AppInfo {
    pub fn with_app_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.version = vk::make_version(major, minor, patch);
        self
    }

    pub fn with_app_version_vk(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    pub fn with_vk_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.api_version = vk::make_version(major, minor, patch);
        self
    }

    pub fn with_name(mut self, name: &str) -> Result<Self> {
        self.name = CString::new(name)?;
        Ok(self)
    }

    pub fn with_validation(mut self, validation: bool) -> Self {
        self.validation = true;
        self
    }
}

impl Default for AppInfo {
    /// Defaults to Vulkan 1.1, with validation layers disabled.
    fn default() -> Self {
        Self {
            name: CString::new(env!("CARGO_PKG_NAME")).unwrap(),
            api_version: vk::make_version(1, 1, 0),
            version: vk::make_version(1, 0, 0),
            validation: false,
        }
    }
}

/// This crate's version as a Vulkan-formatted u32. Note that this requires `vk` to be in the
/// current namespace.
#[macro_export]
macro_rules! cargo_vk_version {
    () => {
        erupt::vk1_0::make_version(
            env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
            env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
            env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
        )
    };
}

/// Return the Vulkan-ready version of this engine
pub fn engine_version() -> u32 {
    cargo_vk_version!()
}
