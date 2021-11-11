use crate::mainloop::{Platform, PlatformEvent, PlatformReturn};
use crate::winit_arcball::WinitArcBall;
use crate::xr_camera;
use anyhow::Result;
use nalgebra::Matrix4;

pub enum MultiPlatformCamera {
    Winit(WinitArcBall),
    OpenXr,
}

const PLATFORM_WARNING: &str =
    "Mutli platform camera was created a different platform than this call";

impl MultiPlatformCamera {
    pub fn new(platform: &mut Platform<'_>) -> Self {
        match platform {
            Platform::OpenXr { .. } => Self::OpenXr,
            Platform::Winit { .. } => Self::Winit(WinitArcBall::default()),
        }
    }

    pub fn get_matrices(&self, platform: &Platform) -> Result<(PlatformReturn, [f32; 4 * 4 * 2])> {
        let prefix = match self {
            Self::Winit(arcball) => arcball.matrix(),
            Self::OpenXr => Matrix4::identity(),
        };
        platform_camera_prefix(platform, bytemuck::cast(*prefix.as_ref()))
    }

    pub fn handle_event(
        &mut self,
        event: &mut PlatformEvent<'_, '_>,
        _platform: &mut Platform<'_>,
    ) {
        match (self, event) {
            (Self::Winit(winit_arcball), PlatformEvent::Winit(event)) => {
                if let winit::event::Event::WindowEvent { event, .. } = event {
                    winit_arcball.handle_events(event);
                }
            }
            (Self::OpenXr, PlatformEvent::OpenXr(_)) => (),
            _ => panic!("{}", PLATFORM_WARNING),
        }
    }
}

/// Create the specified PlatformReturn and return camera matrices for one or both eyes, prefixed with the given 4x4 matrix
pub fn platform_camera_prefix(platform: &Platform, prefix: [f32; 4 * 4]) -> Result<(PlatformReturn, [f32; 4 * 4 * 2])> {
    match platform {
        // Winit mode
        #[cfg(feature = "winit")]
        Platform::Winit { .. } => {
            let mut data = [0.0; 4 * 4 * 2];
            data[..prefix.len()].copy_from_slice(&prefix);
            Ok((PlatformReturn::Winit, data))
        }
        // OpenXR mode
        #[cfg(feature = "openxr")]
        Platform::OpenXr {
            xr_core,
            frame_state,
        } => {
            let prefix = Matrix4::from_row_slice(&prefix);
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