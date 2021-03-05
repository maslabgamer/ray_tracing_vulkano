use cgmath::{Vector3, Rad, Matrix4, InnerSpace, Point3, EuclideanSpace};

use crate::cs;
use crate::object_traits::Uniform;

#[derive(Debug)]
pub struct Camera {
    pub(crate) position: Vector3<f32>,
    pub(crate) yaw: Rad<f32>,
    pub(crate) pitch: Rad<f32>,
    pub(crate) speed: f32,
    pub(crate) sensitivity: f32,
}

impl Camera {
    pub fn from_origin() -> Self {
        Camera {
            position: Vector3 { x: 0.0, y: 0.0, z: 0.0 },
            yaw: cgmath::Deg(-90.0).into(),
            pitch: cgmath::Deg(0.0).into(),
            speed: 1.5,
            sensitivity: 0.5,
        }
    }

    fn calc_rotation_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_to_rh(
            Point3::from_vec(self.position),
            Vector3::new(
                self.yaw.0.cos(),
                self.pitch.0.sin(),
                self.yaw.0.sin(),
            ).normalize(),
            Vector3::unit_y(),
        )
    }
}

impl Uniform for Camera {
    type Uniform = cs::ty::Camera;

    fn to_uniform(&self) -> Self::Uniform {
        cs::ty::Camera {
            position: self.position.into(),
            padding: 0.0,
            rotation: self.calc_rotation_matrix().into()
        }
    }
}
