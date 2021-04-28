use crate::shortcuts::winit_arcball::WinitArcBall;
use crate::{Platform, PlatformEvent, PlatformReturn};

pub enum MultiPlatformCamera {
    Winit(WinitArcBall),
    OpenXr,
}

impl MultiPlatformCamera {
    pub fn new(platform: Platform<'_>) -> Self {
        match platform {
            Platform::OpenXr { .. } => Self::OpenXr,
            Platform::Winit { .. } => Self::Winit(WinitArcBall::default()),
        }
    }

    pub fn get_matrices(&self, platform: Platform) -> (PlatformReturn, [f32; 4 * 4 * 2]) {
        match (self, platform) {
            (Self::Winit(winit_arcball), Platform::Winit { .. }) => {
                let matrix = winit_arcball.matrix();
                let mut data = [0.0; 32];
                data.iter_mut()
                    .zip(matrix.as_slice().iter())
                    .for_each(|(o, i)| *o = *i);
                (PlatformReturn::Winit, data)
            }
            (Self::OpenXr, Platform::OpenXr { .. }) => {
                todo!()
            }
            _ => panic!("Invalid platform"),
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
            (Self::OpenXr, PlatformEvent::OpenXr(_)) => (),
            _ => panic!("Invalid platform"),
        }
    }
}
