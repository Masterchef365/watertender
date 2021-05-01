mod synchronization;
pub use synchronization::Synchronization;
mod framebuffer_mgr;
pub use framebuffer_mgr::FramebufferManager;
mod render_pass;
pub use render_pass::create_render_pass;
pub mod memory;
pub use memory::{buffer_memory_req, image_memory_req, ManagedBuffer, ManagedImage, MemoryBlock};
mod vertex;
pub use vertex::Vertex;
mod shader;
pub use shader::shader;
pub mod mesh;
mod staging_buffer;
pub use staging_buffer::StagingBuffer;
mod frame_data_ubo;
pub use frame_data_ubo::FrameDataUbo;

#[cfg(feature = "nalgebra")]
pub mod arcball;

#[cfg(all(feature = "nalgebra", feature = "winit"))]
pub mod winit_arcball;

#[cfg(all(feature = "nalgebra", feature = "openxr"))]
pub mod xr_camera;

#[cfg(all(feature = "nalgebra", feature = "openxr", feature = "winit"))]
mod multi_platform_camera;
#[cfg(all(feature = "nalgebra", feature = "openxr", feature = "winit"))]
pub use multi_platform_camera::MultiPlatformCamera;

#[cfg(all(feature = "nalgebra", feature = "openxr", feature = "winit"))]
pub mod starter_kit;

/// Launch a mainloop, and change platform depending on a boolean
#[cfg(all(feature = "winit", feature = "openxr"))]
pub fn launch<M: crate::SyncMainLoop + 'static>(
    info: crate::AppInfo,
    vr: bool,
) -> anyhow::Result<()> {
    if vr {
        crate::openxr_backend::launch::<M>(info)
    } else {
        crate::winit_backend::launch::<M>(info)
    }
}
