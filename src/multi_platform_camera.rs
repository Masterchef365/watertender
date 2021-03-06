use crate::mainloop::{Platform, PlatformEvent, PlatformReturn};
use crate::winit_arcball::WinitArcBall;
use anyhow::Result;

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

    pub fn get_matrices(&self, platform: &Platform) -> Result<(PlatformReturn, [f32; 4 * 4 * 2])> {
        match (self, platform) {
            // Winit mode
            (Self::Winit(winit_arcball), Platform::Winit { .. }) => {
                let matrix = winit_arcball.matrix();
                let mut data = [0.0; 32];
                data.iter_mut()
                    .zip(matrix.as_slice().iter())
                    .for_each(|(o, i)| *o = *i);
                Ok((PlatformReturn::Winit, data))
            }
            // OpenXR mode
            #[cfg(feature = "openxr")]
            (
                Self::OpenXr,
                Platform::OpenXr {
                    xr_core,
                    frame_state,
                },
            ) => {
                let (_, views) = xr_core.session.locate_views(
                    openxr::ViewConfigurationType::PRIMARY_STEREO,
                    frame_state.expect("No frame state").predicted_display_time,
                    &xr_core.stage,
                )?;
                let view_to_mat = |view: openxr::View| {
                    let proj = xr_camera::projection_from_fov(&view.fov, 0.01, 1000.0); // TODO: Settings?
                    let view = xr_camera::view_from_pose(&view.pose);
                    proj * view
                };
                let left = view_to_mat(views[0]);
                let right = view_to_mat(views[1]);
                let mut data = [0.0; 32];
                data.iter_mut()
                    .zip(left.as_slice().iter().chain(right.as_slice().iter()))
                    .for_each(|(o, i)| *o = *i);
                Ok((PlatformReturn::OpenXr(views), data))
            }
            #[cfg(unreachable_patterns)]
            _ => panic!("{}", PLATFORM_WARNING),
        }
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
            #[cfg(feature = "openxr")]
            (Self::OpenXr, PlatformEvent::OpenXr(_)) => (),
            #[allow(unreachable_patterns)]
            _ => panic!("{}", PLATFORM_WARNING),
        }
    }
}
