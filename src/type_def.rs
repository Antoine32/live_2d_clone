use palette::rgb::Rgba;
use rapier::math::{
    AngVector as MAngVector, Point as MPoint, Real as MReal, Rotation as MRotation,
    Vector as MVector,
};
use rapier::na::{Isometry3 as MIsometry3, Perspective3 as MPerspective};
use rapier::prelude::{ColliderHandle as MColliderHandle, RigidBodyHandle as MRigidBodyHandle};
use rapier2d::na::Orthographic3;
use specs::{Component, DenseVecStorage, FlaggedStorage};

pub type Real = MReal;
pub type Vector = MVector<Real>;
pub type Point = MPoint<Real>;
pub type Rotator = MRotation<Real>;
pub type AngVector = MAngVector<Real>;
pub type Isometry = MIsometry3<Real>;
pub type Perspective = MPerspective<Real>;
pub type Indices = u32;
pub type RapierColliderHandle = MColliderHandle;
pub type RapierRigidBodyHandle = MRigidBodyHandle;

#[derive(Debug)]
pub struct Position(pub Point);

impl Position {
    pub fn new(x: Real, y: Real) -> Self {
        Self {
            0: Point::new(x, y),
        }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

impl Component for Position {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Debug)]
pub struct Rotation(pub Rotator);

impl Default for Rotation {
    fn default() -> Self {
        Self {
            0: Rotator::new(0.0),
        }
    }
}

impl Rotation {
    pub fn new(angle: Real) -> Self {
        Self {
            0: Rotator::new(angle),
        }
    }
}

impl Component for Rotation {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Debug, Default)]
pub struct Scale(pub Vector);

impl Scale {
    pub fn new(x: Real, y: Real) -> Self {
        Self {
            0: Vector::new(x, y),
        }
    }
}

impl Component for Scale {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Debug, Default)]
pub struct Translation(pub Vector);

impl Translation {
    pub fn new(x: Real, y: Real) -> Self {
        Self {
            0: Vector::new(x, y),
        }
    }
}

impl Component for Translation {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Debug, Default)]
pub struct Color(pub Rgba); // Rgba (linear) or Srgba (non-linear (as in exponentiel))

impl Color {
    pub fn new_rgba(r: Real, g: Real, b: Real, a: Real) -> Self {
        Self {
            0: Rgba::new(r, g, b, a),
        }
    }

    pub fn new_rgb(r: Real, g: Real, b: Real) -> Self {
        Self {
            0: Rgba::new(r, g, b, 1.0),
        }
    }

    pub fn to_uniform_rgba(color: &Rgba) -> [Real; 4] {
        let (r, g, b, a) = color.into_components();
        return [r, g, b, a];
    }

    pub fn to_uniform_rgb(color: &Rgba) -> [Real; 3] {
        let (r, g, b, _) = color.into_components();
        return [r, g, b];
    }
}

impl Component for Color {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}
