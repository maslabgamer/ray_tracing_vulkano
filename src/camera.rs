use cgmath::Vector3;

use crate::cs;
use crate::object_traits::Uniform;

#[derive(Debug)]
pub struct Camera {
    pub(crate) position: Vector3<f32>,
}

impl Camera {
    pub fn from_origin() -> Self {
        Camera {
            position: Vector3 { x: 0.0, y: 0.0, z: 0.0 }
        }
    }
}

impl Uniform for Camera {
    type Uniform = cs::ty::Camera;

    fn to_uniform(&self) -> Self::Uniform {
        cs::ty::Camera {
            position: self.position.into(),
        }
    }
}
