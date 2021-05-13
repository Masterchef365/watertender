pub mod framebuffer_mgr;
pub mod frame_data_ubo;
pub mod render_pass;
pub mod shader;
pub mod staging_buffer;
pub mod synchronization;
pub mod vertex;
pub mod app_info;
pub mod core;
pub mod defaults;
pub mod hardware_query;
pub mod memory;
pub mod mesh;

pub mod prelude {
    pub use super::{
        framebuffer_mgr::FramebufferManager,
        frame_data_ubo::FrameDataUbo,
        memory::{buffer_memory_req, image_memory_req, ManagedBuffer, ManagedImage, MemoryBlock},
        render_pass::create_render_pass,
        shader::shader,
        staging_buffer::StagingBuffer,
        synchronization::Synchronization,
        vertex::Vertex,
    };
}

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

/// Vulkan implementation supplied by Erupt
pub use erupt::vk;

#[cfg(feature = "openxr")]
pub mod openxr_backend;
#[cfg(feature = "openxr")]
pub use openxr;

#[cfg(feature = "winit")]
pub mod winit_backend;
#[cfg(feature = "winit")]
pub use winit;

/// Mainloop abstraction
#[cfg(any(feature = "openxr", feature = "winit"))]
pub mod mainloop;

#[cfg(feature = "nalgebra")]
pub use nalgebra;

/// Go figure
pub const ENGINE_NAME: &str = "WaterTender";

pub use crate::core::{Core, SharedCore};
