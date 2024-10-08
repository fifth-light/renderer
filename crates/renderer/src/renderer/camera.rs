use std::time::Duration;

use glam::{Mat4, Vec3};

use crate::renderer::uniform::camera::CameraUniformBuffer;

#[derive(Debug, Clone)]
pub enum CameraProjection {
    Perspective {
        aspect: Option<f32>,
        yfov: f32,
        znear: f32,
        zfar: Option<f32>,
    },
    Orthographic {
        xmag: f32,
        ymag: f32,
        zfar: f32,
        znear: f32,
    },
}

impl CameraProjection {
    pub fn update_aspect(&mut self, new_aspect: f32) {
        if let CameraProjection::Perspective { aspect, .. } = self {
            *aspect = Some(new_aspect);
        }
    }

    pub fn aspect(&self) -> Option<f32> {
        match self {
            CameraProjection::Perspective { aspect, .. } => *aspect,
            CameraProjection::Orthographic { xmag, ymag, .. } => Some(xmag / ymag),
        }
    }

    pub fn matrix(&self, default_aspect: f32) -> Mat4 {
        match self {
            CameraProjection::Perspective {
                aspect,
                yfov,
                znear,
                zfar,
            } => {
                let aspect = aspect.unwrap_or(default_aspect);
                if let Some(zfar) = zfar {
                    Mat4::perspective_rh(yfov.to_radians(), aspect, *znear, *zfar)
                } else {
                    Mat4::perspective_infinite_rh(yfov.to_radians(), aspect, *znear)
                }
            }
            CameraProjection::Orthographic {
                xmag,
                ymag,
                zfar,
                znear,
            } => Mat4::orthographic_rh(
                -*xmag / 2.0,
                *xmag / 2.0,
                -*ymag / 2.0,
                *ymag / 2.0,
                *znear,
                *zfar,
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CameraView {
    pub eye: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

impl CameraView {
    fn front_from_yaw_pitch(yaw: f32, pitch: f32) -> Vec3 {
        let yaw = yaw.to_radians();
        let pitch = pitch.to_radians();
        let x = yaw.cos() * pitch.cos();
        let y = pitch.sin();
        let z = yaw.sin() * pitch.cos();
        Vec3::new(x, y, z).normalize()
    }

    pub fn front(&self) -> Vec3 {
        Self::front_from_yaw_pitch(self.yaw, self.pitch)
    }

    fn front_ignore_pitch(&self, yaw_offset: f32) -> Vec3 {
        Self::front_from_yaw_pitch(self.yaw + yaw_offset, 0.0)
    }

    pub fn move_eye(&mut self, offset: Vec3) {
        self.eye += offset;
    }

    pub fn matrix(&self) -> Mat4 {
        let target = self.eye + self.front();
        Mat4::look_at_rh(self.eye, target, Vec3::Y)
    }
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub view: CameraView,
    pub projection: CameraProjection,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            view: CameraView {
                eye: (1.0, 1.0, 1.0).into(),
                yaw: 0.0,
                pitch: 0.0,
            },
            projection: CameraProjection::Perspective {
                aspect: None,
                yfov: 75.0,
                znear: 0.01,
                zfar: None,
            },
        }
    }
}

impl Camera {
    pub fn new(view: CameraView, projection: CameraProjection) -> Self {
        Self { view, projection }
    }

    pub fn move_eye(&mut self, offset: Vec3) {
        self.view.move_eye(offset);
    }

    pub fn update_aspect(&mut self, new_aspect: f32) {
        self.projection.update_aspect(new_aspect);
    }

    pub fn aspect(&self) -> Option<f32> {
        self.projection.aspect()
    }

    pub fn update_uniform(&self, buffer: &mut CameraUniformBuffer, default_aspect: f32) {
        buffer.update_view(self, default_aspect);
    }

    pub fn matrix(&self, default_aspect: f32) -> Mat4 {
        let proj = self.projection.matrix(default_aspect);
        let view = self.view.matrix();
        proj * view
    }
}

#[derive(Clone, Debug)]
pub struct PositionController {
    pub speed: f32,
    pub forward: f32,
    pub backward: f32,
    pub left: f32,
    pub right: f32,
    pub up: f32,
    pub down: f32,
}

impl Default for PositionController {
    fn default() -> Self {
        Self {
            speed: 0.01,
            forward: 0.0,
            backward: 0.0,
            left: 0.0,
            right: 0.0,
            up: 0.0,
            down: 0.0,
        }
    }
}

impl PositionController {
    pub fn update(&self, duration: Duration, camera: &mut Camera) {
        let milliseconds = duration.as_millis();
        let distance = self.speed * milliseconds as f32;
        let mut movement: Vec3 = (0.0, 0.0, 0.0).into();

        let forward: Vec3 = camera.view.front_ignore_pitch(0.0) * distance;
        movement += forward * self.forward;
        movement -= forward * self.backward;

        let left: Vec3 = camera.view.front_ignore_pitch(-90.0) * distance;
        movement += left * self.left;
        movement -= left * self.right;

        movement.y += distance * self.up;
        movement.y -= distance * self.down;

        camera.move_eye(movement)
    }
}
