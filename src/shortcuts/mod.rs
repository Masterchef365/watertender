mod synchronization;
pub use synchronization::Synchronization;
mod framebuffer_mgr;
pub use framebuffer_mgr::FramebufferManager;
mod render_pass;
pub use render_pass::create_render_pass;
mod memory;
pub use memory::{
    buffer_memory_req, image_memory_req, ManagedBuffer, ManagedImage, MemoryBlock, UsageFlags,
};
mod vertex;
pub use vertex::Vertex;
mod shader;
pub use shader::shader;

/// Launch a mainloop, and change platform depending on a boolean
#[cfg(all(feature = "winit", feature = "openxr"))]
pub fn launch<M: crate::WinitMainLoop + 'static>(
    info: crate::AppInfo,
    vr: bool,
) -> anyhow::Result<()> {
    if vr {
        crate::openxr_backend::launch::<M>(info)
    } else {
        crate::winit_backend::launch::<M>(info)
    }
}
