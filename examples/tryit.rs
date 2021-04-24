#![allow(unused)]
use watertender::*;
use anyhow::Result;

struct App;

fn main() -> Result<()> {
    if std::env::args().count() > 1 {
        openxr_backend::launch::<App>(Default::default())
    } else {
        winit_backend::launch::<App>(Default::default())
    }
}

/// All mainloops run on executors must implement this trait
impl MainLoop for App {
    /// Creates a new instance of your app. Mainly useful for setting up data structures and
    /// allocating memory.
    fn new(core: &Core, platform: Platform<'_>) -> Result<Self> {
        Ok(App)
    }

    /// A frame handled by your app. The command buffers in `frame` are already reset and have begun, and will be ended and submitted.
    fn frame(&mut self, frame: Frame, core: &Core, platform: Platform<'_>) -> Result<()> {
        Ok(())
    }

    /// Renderpass used to output to the framebuffer provided in Frame
    fn swapchain_resize(&self, images: Vec<vk::Image>, extent: vk::Extent2D) -> Result<()> {
        Ok(())
    }

    /// Handle an event produced by the Platform
    fn event(
        &mut self,
        event: PlatformEvent<'_, '_>,
        core: &Core,
        platform: Platform<'_>,
    ) -> Result<()> {
        if let PlatformEvent::Winit(ev) = event {
            dbg!(ev);
        }
        Ok(())
    }
}

impl WinitMainLoop for App {
    fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore) {
        todo!()
    }
}
