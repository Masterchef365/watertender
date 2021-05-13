use nalgebra::{Matrix4, Quaternion, Unit, Vector3};
use openxr as xr;

/// Create a view matrix for the given pose
/// Ported from:
/// https://gitlab.freedesktop.org/monado/demos/xrgears/-/blob/master/src/main.cpp
pub fn view_from_pose(pose: &xr::Posef) -> Matrix4<f32> {
    let quat = pose.orientation;
    let quat = Quaternion::new(quat.w, quat.x, quat.y, quat.z);
    let quat = Unit::try_new(quat, 0.0).expect("Not a unit quaternion");
    let rotation = quat.to_homogeneous();

    let position = pose.position;
    let position = Vector3::new(position.x, position.y, position.z);
    let translation = Matrix4::new_translation(&position);

    let view = translation * rotation;
    let inv = view.try_inverse().expect("Matrix didn't invert");
    inv
}

/// Create a projection matrix for the given pose
pub fn projection_from_fov(fov: &xr::Fovf, near: f32, far: f32) -> Matrix4<f32> {
    let tan_left = fov.angle_left.tan();
    let tan_right = fov.angle_right.tan();

    let tan_up = fov.angle_up.tan();
    let tan_down = fov.angle_down.tan();

    let tan_width = tan_right - tan_left;
    let tan_height = tan_up - tan_down;

    let a11 = 2.0 / tan_width;
    let a22 = 2.0 / tan_height;

    let a31 = (tan_right + tan_left) / tan_width;
    let a32 = (tan_up + tan_down) / tan_height;
    let a33 = -far / (far - near);

    let a43 = -(far * near) / (far - near);
    Matrix4::new(
        a11, 0.0, a31, 0.0, //
        0.0, -a22, a32, 0.0, //
        0.0, 0.0, a33, a43, //
        0.0, 0.0, -1.0, 0.0, //
    )
}
