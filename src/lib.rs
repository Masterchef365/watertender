/// Vulkan implementation supplied by Erupt
pub use erupt::vk;

use anyhow::Result;
use erupt::extensions::khr_surface::ColorSpaceKHR;
use erupt::{utils::loading::DefaultEntryLoader, DeviceLoader, InstanceLoader};
use gpu_alloc::GpuAllocator;
use std::sync::{Arc, Mutex};

#[cfg(feature = "openxr")]
pub mod openxr_backend;
#[cfg(feature = "openxr")]
pub use openxr;

#[cfg(feature = "winit")]
pub mod winit_backend;
#[cfg(feature = "winit")]
pub use winit;

#[cfg(feature = "nalgebra")]
pub use nalgebra;

mod alloc_helpers;
mod hardware_query;
pub mod shortcuts;

/// All mainloops run on executors must implement this trait
pub trait MainLoop: Sized {
    /// Creates a new instance of your app. Mainly useful for setting up data structures and
    /// allocating memory.
    fn new(core: &SharedCore, platform: Platform<'_>) -> Result<Self>;

    /// A frame handled by your app.
    fn frame(
        &mut self,
        frame: Frame,
        core: &SharedCore,
        platform: Platform<'_>,
    ) -> Result<PlatformReturn>;

    /// Renderpass used to output to the framebuffer provided in Frame
    fn swapchain_resize(&mut self, images: Vec<vk::Image>, extent: vk::Extent2D) -> Result<()>;

    /// Handle an event produced by the Platform
    fn event(
        &mut self,
        event: PlatformEvent<'_, '_>,
        core: &Core,
        platform: Platform<'_>,
    ) -> Result<()>;
}

/// Trait required by the winit backend to synchronize with the swapchain
pub trait SyncMainLoop: MainLoop {
    /// Return (image_available, render_finished). The first semaphore will be signalled by the runtime when the frame is available, and the runtime will wait to present the image until the second semaphore has been signalled.
    /// Therefore you will want to wait on the first semaphore to begin rendering, and signal the second semaphore when you are finished.
    /// This method will be called once before each `frame()`.
    fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore);
}

/// Interface to the gpu's commands
pub struct Frame {
    /// Swapchain image selection
    pub swapchain_index: u32,
}

/// An alias of `Arc<Core>`. Useful to include in subsystems for easy access to Vulkan context
pub type SharedCore = Arc<Core>;

/// A collection of commonly referenced Vulkan context
pub struct Core {
    /// General purpose queue, must be graphics and compute capable
    pub queue: vk::Queue,

    /// Family the queue is from
    pub queue_family: u32,

    /// GPU memory allocator
    pub allocator: Mutex<GpuAllocator<vk::DeviceMemory>>,

    /// Vulkan device
    pub device: DeviceLoader,

    /// Vulkan physical device
    pub physical_device: vk::PhysicalDevice,

    /// Information about the device
    pub device_properties: vk::PhysicalDeviceProperties,

    /// Vulkan instance
    pub instance: InstanceLoader,

    /// Erupt entry
    pub entry: DefaultEntryLoader,
}

/// Multi-platform event
pub enum PlatformEvent<'a, 'b> {
    #[cfg(feature = "winit")]
    Winit(&'b winit::event::Event<'a, ()>),
    #[cfg(feature = "openxr")]
    OpenXr(&'b openxr::Event<'a>),
}

/// Multi-platform
pub enum Platform<'a> {
    #[cfg(feature = "winit")]
    Winit {
        window: &'a winit::window::Window,
        control_flow: &'a mut winit::event_loop::ControlFlow, // TODO: Part of PlatformReturn?
    },
    #[cfg(feature = "openxr")]
    OpenXr {
        xr_core: &'a openxr_backend::XrCore,
        frame_state: Option<openxr::FrameState>,
    },
}

/// Multi-platform return value
pub enum PlatformReturn {
    #[cfg(feature = "winit")]
    Winit,
    #[cfg(feature = "openxr")]
    OpenXr(Vec<openxr::View>),
}

impl Platform<'_> {
    pub fn is_vr(&self) -> bool {
        match self {
            Platform::Winit { .. } => false,
            Platform::OpenXr { .. } => true,
        }
    }
}

/// If you need a different swapchain format, modify the source or get a different engine. I draw
/// the line at color format for presentation, sorry.
pub const COLOR_FORMAT: vk::Format = vk::Format::B8G8R8A8_SRGB;
/// Used in shortcuts, to make things easier
pub const COLOR_SPACE: ColorSpaceKHR = ColorSpaceKHR::SRGB_NONLINEAR_KHR;
/// Used in shortcuts, to make things easier
pub const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;
/// Go figure
pub const ENGINE_NAME: &str = "WaterTender";

/// Application info
pub struct AppInfo {
    pub(crate) name: String,
    pub(crate) version: u32,
    pub(crate) api_version: u32,
    pub(crate) validation: bool,
}

// TODO: Device extensions!
impl AppInfo {
    pub fn app_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.version = vk::make_version(major, minor, patch);
        self
    }

    pub fn vk_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.api_version = vk::make_version(major, minor, patch);
        self
    }

    pub fn name(mut self, name: String) -> Result<Self> {
        self.name = name;
        Ok(self)
    }

    pub fn validation(mut self, validation: bool) -> Self {
        self.validation = validation;
        self
    }
}

impl Default for AppInfo {
    /// Defaults to Vulkan 1.1, with validation layers disabled.
    fn default() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_owned(),
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
