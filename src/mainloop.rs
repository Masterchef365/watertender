use crate::{Core, SharedCore};
use anyhow::Result;
use erupt::vk;

/// Interface to the gpu's commands
pub struct Frame {
    /// Swapchain image selection
    pub swapchain_index: u32,
}

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

/// Multi-platform
pub enum Platform<'a> {
    #[cfg(feature = "winit")]
    Winit {
        window: &'a winit::window::Window,
        control_flow: &'a mut winit::event_loop::ControlFlow, // TODO: Part of PlatformReturn?
    },
    #[cfg(feature = "openxr")]
    OpenXr {
        xr_core: &'a crate::openxr_backend::XrCore,
        frame_state: Option<openxr::FrameState>,
    },
}

/// Multi-platform event
pub enum PlatformEvent<'a, 'b> {
    #[cfg(feature = "winit")]
    Winit(&'b winit::event::Event<'a, ()>),
    #[cfg(feature = "openxr")]
    OpenXr(&'b openxr::Event<'a>),
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
            #[cfg(feature = "openxr")]
            Platform::OpenXr { .. } => true,
            _ => false,
        }
    }
}
