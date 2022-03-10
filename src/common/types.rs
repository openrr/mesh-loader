//! A module that defines the types used in all formats and their associated traits.

use std::ops;

mod sealed {
    pub trait Sealed {}
}

pub trait Float: sealed::Sealed + Sized + Copy + Default {}

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type Color3 = [f32; 3];
pub type Color4 = [f32; 4];

pub trait Vector2D: Copy + From<[f32; 2]> {
    fn x(&self) -> f32;
    fn y(&self) -> f32;
    fn set_x(&mut self, x: f32);
    fn set_y(&mut self, y: f32);
}

pub trait Vector3D: Copy + From<[f32; 3]> {
    fn x(&self) -> f32;
    fn y(&self) -> f32;
    fn z(&self) -> f32;
    fn set_x(&mut self, x: f32);
    fn set_y(&mut self, y: f32);
    fn set_z(&mut self, z: f32);
}

impl Vector2D for Vec2 {
    #[inline]
    fn x(&self) -> f32 {
        self[0]
    }
    #[inline]
    fn y(&self) -> f32 {
        self[1]
    }
    #[inline]
    fn set_x(&mut self, x: f32) {
        self[0] = x;
    }
    #[inline]
    fn set_y(&mut self, y: f32) {
        self[1] = y;
    }
}

impl Vector3D for Vec3 {
    #[inline]
    fn x(&self) -> f32 {
        self[0]
    }
    #[inline]
    fn y(&self) -> f32 {
        self[1]
    }
    #[inline]
    fn z(&self) -> f32 {
        self[2]
    }
    #[inline]
    fn set_x(&mut self, x: f32) {
        self[0] = x;
    }
    #[inline]
    fn set_y(&mut self, y: f32) {
        self[1] = y;
    }
    #[inline]
    fn set_z(&mut self, z: f32) {
        self[2] = z;
    }
}

macro_rules! vector {
    (
        $(
            $trait_name:ident for $struct_name:ident {
                $($set_method:ident($field:ident)),*
            },
        )*
    ) => {
        $(
            #[allow(clippy::exhaustive_structs)]
            #[derive(Debug, Clone, Copy, Default, PartialEq)]
            pub struct $struct_name {
                $(pub $field: f32,)*
            }

            impl $trait_name for $struct_name {
                $(
                    #[inline]
                    fn $field(&self) -> f32 {
                        self.$field
                    }
                    #[inline]
                    fn $set_method(&mut self, $field: f32) {
                        self.$field = $field;
                    }
                )*
            }

            impl ops::Mul<f32> for $struct_name {
                type Output = Self;

                #[inline]
                fn mul(mut self, rhs: f32) -> Self::Output {
                    self *= rhs;
                    self
                }
            }

            impl ops::MulAssign<f32> for $struct_name {
                #[inline]
                fn mul_assign(&mut self, rhs: f32) {
                    $(self.$field *= rhs;)*
                }
            }

            impl ops::Div<f32> for $struct_name {
                type Output = Self;

                #[inline]
                fn div(mut self, rhs: f32) -> Self::Output {
                    self /= rhs;
                    self
                }
            }

            impl ops::DivAssign<f32> for $struct_name {
                #[inline]
                fn div_assign(&mut self, rhs: f32) {
                    $(self.$field /= rhs;)*
                }
            }
        )*
    };
}

vector! {
    Vector2D for Vec2D { set_x(x), set_y(y) },
    Vector3D for Vec3D { set_x(x), set_y(y), set_z(z) },
}

impl From<[f32; 2]> for Vec2D {
    #[inline]
    fn from(array: [f32; 2]) -> Self {
        Self {
            x: array[0],
            y: array[1],
        }
    }
}

impl From<[f32; 3]> for Vec3D {
    #[inline]
    fn from(array: [f32; 3]) -> Self {
        Self {
            x: array[0],
            y: array[1],
            z: array[2],
        }
    }
}
