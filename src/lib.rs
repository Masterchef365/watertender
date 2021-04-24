/// Vulkan implementation supplied by Erupt
pub use erupt::vk;

use anyhow::Result;
use erupt::{utils::loading::DefaultEntryLoader, DeviceLoader, InstanceLoader};
use gpu_alloc::GpuAllocator;
use std::ffi::CString;
use std::sync::{Arc, Mutex};

#[cfg(feature = "openxr")]
mod openxr_backend;

#[cfg(feature = "winit")]
mod winit_backend;

mod alloc_helpers;
mod hardware_query;

/// All mainloops run on executors must implement this trait
pub trait MainLoop: Sized {
    /// Creates a new instance of your app. Mainly useful for setting up data structures and
    /// allocating memory.
    fn new(core: &Core, platform: Platform<'_>) -> Result<Self>;

    /// A frame handled by your app. The command buffers in `frame` are already reset and have begun, and will be ended and submitted.
    fn frame(&mut self, frame: Frame, core: &Core, platform: Platform<'_>) -> Result<()>;

    /// Renderpass used to output to the framebuffer provided in Frame
    fn swapchain_resize(&self, images: Vec<vk::Image>, extent: vk::Extent2D) -> Result<()>;

    /// Handle an event produced by the Platform
    fn event(
        &mut self,
        event: PlatformEvent<'_>,
        core: &Core,
        platform: Platform<'_>,
    ) -> Result<()>;
}

/// Trait required by the winit backend
trait WinitMainLoop: MainLoop {
    /// Return (image_available, render_finished). The first semaphore will be signalled by the runtime when the frame is available, and the runtime will wait to present the image until the second semaphore has been signalled.
    /// Therefore you will want to wait on the first semaphore to begin rendering, and signal the second semaphore when you are finished.
    /// This method will be called once before each `frame()`.
    fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore);
}

/// Interface to the gpu's commands
pub struct Frame {
    /// Swapchain image selection
    pub swapchain_index: usize,
}

/// An alias of `Arc<Core>`. Useful to include in subsystems for easy access to Vulkan context
pub type SharedCore = Arc<Core>;

/// A collection of commonly referenced Vulkan context
pub struct Core {
    /// General purpose queue, must be graphics and compute capable
    pub queue: vk::Queue,

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
pub const COLOR_FORMAT: vk::Format = vk::Format::B8G8R8A8_SRGB;

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

    pub fn with_vk_version_vk(mut self, major: u32, minor: u32, patch: u32) -> Self {
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
        vk::make_version(
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
