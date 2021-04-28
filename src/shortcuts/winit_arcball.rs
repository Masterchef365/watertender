use crate::shortcuts::arcball::ArcBall;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

pub struct WinitArcBall {
    inner: ArcBall,
    pan_sensitivity: f32,
    swivel_sensitivity: f32,
    last_mouse_position: Option<(f64, f64)>,
    width: u32,
    height: u32,
    left_is_clicked: bool,
    right_is_clicked: bool,
}

impl WinitArcBall {
    pub fn new(inner: ArcBall, pan_sensitivity: f32, swivel_sensitivity: f32) -> Self {
        Self {
            inner,
            pan_sensitivity,
            swivel_sensitivity,
            last_mouse_position: None,
            left_is_clicked: false,
            right_is_clicked: false,
            width: 100,
            height: 100,
        }
    }

    pub fn handle_events(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let &PhysicalPosition { x, y } = position;
                if let Some((last_x, last_y)) = self.last_mouse_position {
                    let x_delta = (last_x - x) as f32;
                    let y_delta = (last_y - y) as f32;
                    if self.left_is_clicked {
                        self.mouse_pivot(x_delta, y_delta);
                    } else if self.right_is_clicked {
                        self.mouse_pan(x_delta, y_delta);
                    }
                }
                self.last_mouse_position = Some((x, y));
            }
            WindowEvent::MouseInput { state, button, .. } => match button {
                MouseButton::Left => self.left_is_clicked = *state == ElementState::Pressed,
                MouseButton::Right => self.right_is_clicked = *state == ElementState::Pressed,
                _ => (),
            },
            WindowEvent::MouseWheel { delta, .. } => {
                if let MouseScrollDelta::LineDelta(_x, y) = delta {
                    self.inner.distance += y * 0.3;
                    if self.inner.distance <= 0.01 {
                        self.inner.distance = 0.01;
                    }
                }
            }
            WindowEvent::Resized(size) => {
                self.width = size.width;
                self.height = size.height;
            }
            _ => (),
        }
    }

    fn mouse_pivot(&mut self, delta_x: f32, delta_y: f32) {
        use std::f32::consts::FRAC_PI_2;
        self.inner.yaw -= delta_x * self.swivel_sensitivity;
        self.inner.pitch -= delta_y * self.swivel_sensitivity.max(-FRAC_PI_2).min(FRAC_PI_2);
    }

    fn mouse_pan(&mut self, delta_x: f32, delta_y: f32) {
        let eye = self.inner.eye();
        let x_pan = ArcBall::up().cross(&eye).normalize();
        let y_pan = x_pan.cross(&eye).normalize();
        let rate = self.inner.distance * self.pan_sensitivity;
        self.inner.pivot += x_pan * (delta_x as f32) * rate;
        self.inner.pivot += y_pan * (delta_y as f32) * rate;
    }

    // TODO: Perspective and view matrices?
    pub fn matrix(&self) -> nalgebra::Matrix4<f32> {
        self.inner.matrix(self.width, self.height)
    }
}

impl Default for WinitArcBall {
    fn default() -> Self {
        Self::new(ArcBall::default(), 0.001, 0.004)
    }
}
