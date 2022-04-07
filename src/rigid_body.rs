use rapier::prelude::RigidBodyBuilder;
use specs::{Component, DenseVecStorage, FlaggedStorage};

use crate::RapierRigidBodyHandle;

pub struct RigidBodyHandle(pub RapierRigidBodyHandle);

impl Component for RigidBodyHandle {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl RigidBodyHandle {
    pub fn new(handle: RapierRigidBodyHandle) -> Self {
        Self { 0: handle }
    }
}
