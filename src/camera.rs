use cgmath::Vector3;

use crate::cs;

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

    pub fn to_uniform(&self) -> cs::ty::Camera {
        cs::ty::Camera {
            position: self.position.into(),
        }
    }
}
