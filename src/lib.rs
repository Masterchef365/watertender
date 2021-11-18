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
pub mod headless_backend;

#[cfg(feature = "nalgebra")]
pub mod arcball;

#[cfg(all(feature = "nalgebra", feature = "winit"))]
pub mod winit_arcball;

#[cfg(all(feature = "nalgebra", feature = "openxr"))]
pub mod xr_camera;

#[cfg(all(feature = "nalgebra", feature = "openxr", feature = "winit"))]
pub mod multi_platform_camera;
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

#[cfg(all(feature = "nalgebra", feature = "openxr", feature = "winit"))]
pub mod trivial;

/// Go figure
pub const ENGINE_NAME: &str = "WaterTender";

pub use crate::core::{Core, SharedCore};

pub mod prelude {
    pub use super::{
        render_pass::create_render_pass, 
        framebuffer_mgr::FramebufferManager, 
        staging_buffer::StagingBuffer, 
        synchronization::Synchronization,
        mesh::{ManagedMesh, upload_mesh, draw_mesh},
        memory::{ManagedImage, ManagedBuffer},
        starter_kit::{self, launch, StarterKit},
        frame_data_ubo::FrameDataUbo,
        app_info::AppInfo,
        vertex::Vertex,
        shader::shader,
        Core, SharedCore,
        defaults,
    };
    pub use erupt::vk;

    #[cfg(any(feature = "openxr", feature = "winit"))]
    pub use super::mainloop::{MainLoop, Platform, PlatformReturn, PlatformEvent, SyncMainLoop, Frame};

    #[cfg(all(feature = "nalgebra", any(feature = "openxr", feature = "winit")))]
    pub use super::multi_platform_camera::MultiPlatformCamera;
}
