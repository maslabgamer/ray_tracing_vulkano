use cgmath::{Vector3, Vector4};

use crate::cs;
use crate::object_traits::Uniform;

#[derive(Copy, Clone)]
pub enum LightType {
    Ambient = 0,
    Point = 1,
    Directional = 2
}

#[repr(C)]
pub struct Light {
    light_type: LightType,
    intensity: f32,
    position: Vector4<f32> // Will be used for direction in case it's a directional light
}

impl Light {
    pub fn new(light_type: LightType, intensity: f32, position: Option<Vector3<f32>>) -> Self {
        let position = match position {
            None => Vector4::new(0.0, 0.0, 0.0, 0.0),
            Some(position) => Vector4::new(position.x, position.y, position.z, 0.0)
        };

        Self { light_type, intensity, position }
    }
}

impl Uniform for Light {
    type Uniform = cs::ty::Light;

    fn to_uniform(&self) -> Self::Uniform {
        cs::ty::Light {
            position: self.position.into(),
            intensity: self.intensity.into(),
            lightType: (self.light_type as i32).into(),
            padding: [0.0; 2],
        }
    }
}
