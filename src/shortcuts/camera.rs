use erupt::vk;
use crate::{Platform, shortcuts::ManagedBuffer};
use crate::shortcuts::xr_camera;
use anyhow::Result;

pub struct ProjectionSettings {
    pub width: u32, 
    pub height: u32, 
    pub near: f32, 
    pub far: f32
}

pub trait Camera {
    fn projection(&self, conf: ProjectionSettings) -> Matrix4<f32>;
    fn view(&self) -> Matrix4<f32>;
}
/*

pub trait CameraNav {
    fn new() -> Self;
    fn left_mouse(&mut self, dx: i32, dy: i32);
    fn right_mouse(&mut self, dx: i32, dy: i32);
}
*/

pub enum CameraPlatform<WinitCamera> {
    OpenXr,
    Winit(WinitNav<WinitCamera>),
}

pub struct Camera {
    inner: CameraPlatform
}

impl Camera {
    /// Create a new camera based on the platform
    pub fn new(platform: Platform<'_>, near: f32, far: f32) -> Result<Self> {
    }

    /// Return a packed representation of two cameras in column-major order
    /// For VR, this should be called **as late as possible**
    pub fn packed_lr(&self, platform: Platform<'_>) -> [f32; 4*4*2] {
        
    }

    /// Handle an event 
    pub fn handle_event(&mut self, event: PlatformEvent<'_>) {
        if let 
    }

    pub fn handle_resize(&mut self, extent: vk::Extent2D) {
    }

    pub fn write_buffer(&self, buffer: &mut ManagedBuffer) -> Result<()> {
        buffer.write_bytes(0, bytemuck::cast_slice(self.packed_lr()))
    }
}

impl CameraPlatform {
    pub fn packed_lr(&self, platform: Platform<'_>) -> [f32; 4*4*2] {
        match (self, platform) {
            (Camera::OpenXr, Platform::OpenXr { xr_core, frame_state }) => {
                let (_, views) = xr_core.session.locate_views(
                    openxr::ViewConfigurationType::PRIMARY_STEREO,
                    frame_state.expect("No frame state").predicted_display_time,
                    &xr_core.stage,
                )?;
                pack_vr();
            }
            (Camera::Winit(nav), Platform::Winit { .. }) => {
                pack_single();
            }
            _ => panic!("Platform mismatch"),
        }
    }


}
