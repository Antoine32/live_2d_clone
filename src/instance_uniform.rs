pub use crate::type_def::*;
use crate::{
    buffer_update::{ArcDataIndex, DataManager},
    camera::{Position, Rotation},
    model::{self},
};
use rapier::na::Matrix3;
use rapier2d::na::{self as nalgebra, vector};
use specs::{
    prelude::ComponentEvent, shred::DynamicSystemData, shrev::EventIterator, BitSet, ReadStorage,
    ReaderId, System, World, WorldExt, Write, WriteStorage,
};
use std::mem;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, Default)]
pub struct InstanceUniform {
    pub model_0: [f32; 4],
    pub model_1: [f32; 4],
    pub model_2: [f32; 4],
    pub model_3: [f32; 4],
    pub normal_0: [f32; 3],
    pub normal_1: [f32; 3],
    pub normal_2: [f32; 3],
}

impl model::Vertex for InstanceUniform {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x3, 8 => Float32x3, 9 => Float32x3];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceUniform>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl InstanceUniform {
    fn get_raw(rotation: &Rotator, position: &Point) -> ([[f32; 4]; 4], [[f32; 3]; 3]) {
        let model = Isometry::new(
            vector![position.coords[0], position.coords[1], 1.0],
            vector![0.0, 0.0, rotation.angle()],
        )
        .to_matrix();

        let model: [[f32; 4]; 4] = *(model).as_ref();
        let normal: [[f32; 3]; 3] = Matrix3::from(*rotation).into();

        (model, normal)
    }

    pub fn new(rotation: &Rotator, position: &Point) -> Self {
        let (model, normal) = Self::get_raw(rotation, position);

        Self {
            model_0: model[0],
            model_1: model[1],
            model_2: model[2],
            model_3: model[3],
            normal_0: normal[0],
            normal_1: normal[1],
            normal_2: normal[2],
        }
    }

    fn update(&mut self, rotation: &Rotator, position: &Point) {
        let (model, normal) = Self::get_raw(rotation, position);

        self.model_0 = model[0];
        self.model_1 = model[1];
        self.model_2 = model[2];
        self.model_3 = model[3];
        self.normal_0 = normal[0];
        self.normal_1 = normal[1];
        self.normal_2 = normal[2];
    }
}

#[derive(Default)]
pub struct InstanceUniformUpdate {
    pub dirty: BitSet,
    pub reader_id_pos: Option<ReaderId<ComponentEvent>>,
    pub reader_id_rot: Option<ReaderId<ComponentEvent>>,
}

impl InstanceUniformUpdate {
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

impl<'a> System<'a> for InstanceUniformUpdate {
    type SystemData = (
        Write<'a, DataManager<InstanceUniform>>,
        WriteStorage<'a, ArcDataIndex<InstanceUniform>>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Rotation>,
    );

    fn run(&mut self, (mut data, mut indices, positions, rotations): Self::SystemData) {
        use specs::Join;

        self.dirty.clear();

        let events_pos = positions
            .channel()
            .read(self.reader_id_pos.as_mut().unwrap());
        let events_rot = rotations
            .channel()
            .read(self.reader_id_rot.as_mut().unwrap());

        self.event_update(events_pos);
        self.event_update(events_rot);

        (&mut indices, &positions, &rotations, &self.dirty)
            .join()
            .for_each(|(index, position, rotation, _)| {
                let index = index.0.lock().unwrap();
                let instance_uniform = data.get_mut_index(&index);
                instance_uniform.update(&rotation.0, &position.0);
            });
    }

    fn setup(&mut self, world: &mut World) {
        Self::SystemData::setup(&self.accessor(), world);
        self.reader_id_pos = Some(world.write_component::<Position>().register_reader());
        self.reader_id_rot = Some(world.write_component::<Rotation>().register_reader());
    }
}
