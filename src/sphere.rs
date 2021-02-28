use cgmath::Vector4;

use crate::cs;
use std::convert::TryInto;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Sphere {
    pub(crate) center: Vector4<f32>,
    pub(crate) radius: i32,
    pub(crate) color: [i32; 4],
}

impl Sphere {
    pub fn new(x: f32, y: f32, z: f32, radius: i32, color: &[i32; 4]) -> Self {
        let center = Vector4 { x, y, z, w: 1.0 };
        Sphere { center, radius, color: *color }
    }

    pub fn color_and_pack_radius(&self) -> [i32; 4] {
        let mut color = self.color;
        color[3] = self.radius;
        color
    }

    pub fn to_uniform(&self) -> cs::ty::Sphere {
        cs::ty::Sphere {
            center: self.center.try_into().unwrap(),
            color: self.color_and_pack_radius(),
        }
    }
}
