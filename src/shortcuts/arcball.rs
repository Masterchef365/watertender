use nalgebra::{Matrix4, Point3, Vector3};

/// An arcball camera
pub struct ArcBall {
    pub pivot: Point3<f32>,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub clipping: (f32, f32),
}

impl ArcBall {
    /// Extract the camera matrix given screen dimensions
    pub fn matrix(&self, width: u32, height: u32) -> Matrix4<f32> {
        self.perspective(width, height) * self.view()
    }

    /// Perspective matrix
    pub fn perspective(&self, width: u32, height: u32) -> Matrix4<f32> {
        Matrix4::new_perspective(
            width as f32 / height as f32,
            self.fov,
            self.clipping.0,
            self.clipping.1,
        )
    }

    /// View matrix
    pub fn view(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(&(self.pivot + self.eye()), &self.pivot, &Self::up())
    }

    /// Eye position
    pub fn eye(&self) -> Vector3<f32> {
        Vector3::new(
            self.yaw.cos() * self.pitch.cos().abs(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos().abs(),
        ) * self.distance
    }

    /// Up direction for the camera
    pub fn up() -> Vector3<f32> {
        Vector3::new(0.0, 1.0, 0.0)
    }
}

impl Default for ArcBall {
    fn default() -> Self {
        Self {
            pivot: Point3::origin(),
            distance: 15.0,
            yaw: 1.0,
            pitch: 1.0,
            fov: 45.0f32.to_radians(),
            clipping: (0.1, 2000.0),
        }
    }
}

