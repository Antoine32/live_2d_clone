pub use crate::type_def::*;
use rapier2d::na::{Point3, Vector3};
use specs::{
    prelude::ComponentEvent, shred::DynamicSystemData, shrev::EventIterator, BitSet, Component,
    DenseVecStorage, FlaggedStorage, Read, ReadStorage, ReaderId, System, World, WorldExt,
    WriteStorage,
};

use crate::{actor::Time, model::Model, sprite_selector::SpriteSelector, Isometry};

#[derive(Debug)]
pub struct Projection(pub Perspective);

impl Component for Projection {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl Projection {
    pub fn new(aspect: Real, fovy: Real, znear: Real, zfar: Real) -> Self {
        Self {
            0: Perspective::new(aspect, fovy, znear, zfar),
        }
    }
}

#[derive(Debug)]
pub struct View(pub Isometry);

impl Component for View {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl View {
    fn get_raw(position: &Point, rotation: &Rotator) -> Isometry {
        /*let (roll, pitch, yaw) = rotation.euler_angles();
        let (sin_roll, cos_roll) = roll.sin_cos();
        let (sin_pitch, cos_pitch) = pitch.sin_cos();
        let (sin_yaw, cos_yaw) = yaw.sin_cos();

        let target = Point::new(
            cos_pitch * cos_yaw + position.x,
            sin_pitch + position.y,
            cos_pitch * sin_yaw + position.z,
        );

        let up = Vector::new(sin_roll * -sin_yaw, cos_roll, sin_roll * cos_yaw);

        Isometry3::look_at_rh(position, &target, &up)*/
        let view = Isometry::look_at_rh(
            &Point3::new(position.coords[0], position.coords[1], 0.0),
            &Point3::new(position.coords[0], position.coords[1], 1.0),
            &Vector3::new(rotation.sin_angle(), rotation.cos_angle(), 0.0),
        );

        //let view = Isometry::new(position.coords, 0.0);

        view
    }

    pub fn new(position: &Point, rotation: &Rotator) -> Self {
        Self {
            0: Self::get_raw(position, rotation),
        }
    }

    pub fn update(&mut self, position: &Point, rotation: &Rotator) {
        self.0 = Self::get_raw(position, rotation);
    }
}

#[derive(Default)]
pub struct ViewUpdate {
    pub dirty: BitSet,
    pub reader_id_pos: Option<ReaderId<ComponentEvent>>,
    pub reader_id_rot: Option<ReaderId<ComponentEvent>>,
}

impl ViewUpdate {
    fn event_update(&mut self, events: EventIterator<'_, ComponentEvent>) {
        for event in events.into_iter() {
            match event {
                ComponentEvent::Modified(id) | ComponentEvent::Inserted(id) => {
                    self.dirty.add(*id);
                }
                ComponentEvent::Removed(_id) => (),
            }
        }
    }
}

impl<'a> System<'a> for ViewUpdate {
    type SystemData = (
        WriteStorage<'a, View>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Rotation>,
    );

    fn run(&mut self, (mut view, position, rotation): Self::SystemData) {
        use specs::Join;

        self.dirty.clear();

        let events_pos = position
            .channel()
            .read(self.reader_id_pos.as_mut().unwrap());
        let events_rot = rotation
            .channel()
            .read(self.reader_id_rot.as_mut().unwrap());

        self.event_update(events_pos);
        self.event_update(events_rot);

        (&mut view, &position, &rotation, &self.dirty)
            .join()
            .for_each(|(view, position, rotation, _)| {
                view.update(&position.0, &rotation.0);
            });
    }

    fn setup(&mut self, world: &mut World) {
        Self::SystemData::setup(&self.accessor(), world);
        self.reader_id_pos = Some(world.write_component::<Position>().register_reader());
        self.reader_id_rot = Some(world.write_component::<Rotation>().register_reader());
    }
}

pub struct ActorUpdate;

impl<'a> System<'a> for ActorUpdate {
    type SystemData = (
        ReadStorage<'a, Model>,
        WriteStorage<'a, Rotation>,
        WriteStorage<'a, SpriteSelector>,
        ReadStorage<'a, Translation>,
        Read<'a, Time>,
    );

    fn run(
        &mut self,
        (materials, mut rotations, mut sprite_selectors, offsets, time): Self::SystemData,
    ) {
        use specs::Join;

        let time_delta = time.delta.as_secs_f32();

        sprite_selectors.set_event_emission(false);
        for (_, mut rotation, sprite_selector, _) in (
            &materials,
            &mut rotations.restrict_mut(),
            &mut sprite_selectors,
            &offsets,
        )
            .join()
        {
            if time.on {
                rotation.get_mut_unchecked().0 *= Rotator::new(time_delta);
                sprite_selector.play();
            } else {
                sprite_selector.stop();
            }
        }
        sprite_selectors.set_event_emission(true);
    }
}

/*#[derive(Debug)]
pub struct CameraBuffer(pub wgpu::Buffer);

impl Default for CameraBuffer {
    fn default() -> Self {
        todo!()
    }
}

#[derive(Debug, Default, Component)]
#[storage(DefaultVecStorage)]
pub struct CameraOffset(pub wgpu::BufferAddress);

impl CameraOffset {
    pub fn new(offset: wgpu::BufferAddress) -> Self {
        Self { 0: offset }
    }
}
*/
