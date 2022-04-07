use rapier::prelude::ColliderBuilder;
use specs::{Component, DenseVecStorage, FlaggedStorage};

use crate::RapierColliderHandle;

pub struct ColliderHandle(pub RapierColliderHandle);

impl Component for ColliderHandle {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl ColliderHandle {
    pub fn new(handle: RapierColliderHandle) -> Self {
        Self { 0: handle }
    }
}
