use cgmath::Vector4;

use crate::cs;
use crate::object_traits::Uniform;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Sphere {
    pub(crate) center: Vector4<f32>,
    pub(crate) radius: i32,
    pub(crate) color: [f32; 4],
    pub(crate) specular: f32,
}

impl Sphere {
    pub fn new(x: f32, y: f32, z: f32, radius: i32, color: &[f32; 4], specular: i32) -> Self {
        let center = Vector4 { x, y, z, w: 1.0 };
        Sphere { center, radius, color: *color, specular: specular as f32 }
    }
}

impl Uniform for Sphere {
    type Uniform = cs::ty::Sphere;

    fn to_uniform(&self) -> Self::Uniform {
        cs::ty::Sphere {
            center: self.center.into(),
            color: self.color.into(),
            radius: (self.radius as f32).into(),
            specular: self.specular.into(),
            padding: [0.0; 2],
        }
    }
}
