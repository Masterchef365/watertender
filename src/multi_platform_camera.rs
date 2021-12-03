use crate::mainloop::{Platform, PlatformEvent, PlatformReturn};
use crate::winit_arcball::WinitArcBall;
use anyhow::Result;
use nalgebra::Matrix4;

#[cfg(feature = "openxr")]
use crate::xr_camera;

pub enum MultiPlatformCamera {
    Winit(WinitArcBall),
    #[cfg(feature = "openxr")]
    OpenXr,
}

const PLATFORM_WARNING: &str =
    "Mutli platform camera was created a different platform than this call";

impl MultiPlatformCamera {
    pub fn new(platform: &mut Platform<'_>) -> Self {
        match platform {
            #[cfg(feature = "openxr")]
            Platform::OpenXr { .. } => Self::OpenXr,
            Platform::Winit { .. } => Self::Winit(WinitArcBall::default()),
        }
    }

    /// Get the prefix matrix of this camera
    pub fn get_prefix(&self) -> Matrix4<f32> {
        match self {
            Self::Winit(arcball) => arcball.matrix(),
            #[cfg(feature = "openxr")]
            Self::OpenXr => Matrix4::identity(),
        }
    }

    /// Get the prefix matrix of this camera (appended with VR matrices in VR mode)
    pub fn get_matrices_prefix(&self, platform: &Platform) -> Result<(PlatformReturn, [f32; 4 * 4 * 2])> {
        platform_camera_prefix(platform, self.get_prefix())
    }

    /// Handle a platform event; Returns true if the event was consumed.
    pub fn handle_event(
        &mut self,
        event: &PlatformEvent<'_, '_>,
    ) -> bool {
        match (self, event) {
            (Self::Winit(winit_arcball), PlatformEvent::Winit(event)) => {
                if let winit::event::Event::WindowEvent { event, .. } = event {
                    winit_arcball.handle_events(event)
                } else {
                    false
                }
            }
            #[cfg(feature = "openxr")]
            (Self::OpenXr, PlatformEvent::OpenXr(_)) => false,
            #[allow(unreachable_patterns)]
            _ => panic!("{}", PLATFORM_WARNING),
        }
    }
}

/// Create the specified PlatformReturn and return camera matrices for one or both eyes, prefixed with the given 4x4 matrix
pub fn platform_camera_prefix(platform: &Platform, prefix: Matrix4<f32>) -> Result<(PlatformReturn, [f32; 4 * 4 * 2])> {
    match platform {
        // Winit mode
        Platform::Winit { .. } => {
            let mut data = [0.0; 4 * 4 * 2];
            data[..prefix.len()].copy_from_slice(prefix.as_slice());
            Ok((PlatformReturn::Winit, data))
        }
        // OpenXR mode
        #[cfg(feature = "openxr")]
        Platform::OpenXr {
            xr_core,
            frame_state,
        } => {
            let (_, views) = xr_core.session.locate_views(
                openxr::ViewConfigurationType::PRIMARY_STEREO,
                frame_state.expect("No frame state").predicted_display_time,
                &xr_core.stage,
            )?;
            let view_to_mat = |view: openxr::View| {
                let proj = xr_camera::projection_from_fov(&view.fov, 0.01, 1000.0); // TODO: Settings?
                let view = xr_camera::view_from_pose(&view.pose);
                proj * view * prefix
            };
            let left = view_to_mat(views[0]);
            let right = view_to_mat(views[1]);
            let mut data = [0.0; 32];
            data.iter_mut()
                .zip(left.as_slice().iter().chain(right.as_slice().iter()))
                .for_each(|(o, i)| *o = *i);
            Ok((PlatformReturn::OpenXr(views), data))
        }
    }
}
