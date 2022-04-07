use palette::rgb::Rgba;
use specs::{
    prelude::ComponentEvent, shred::DynamicSystemData, shrev::EventIterator, BitSet, ReadStorage,
    ReaderId, System, World, WorldExt, Write, WriteStorage,
};

use crate::{
    buffer_update::{ArcDataIndex, DataManager},
    camera::{Color, Projection, View},
    Isometry, Perspective,
};

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct CameraUniform {
    pub proj_matrix: [[f32; 4]; 4],
    pub view_matrix: [[f32; 4]; 4],
    //pub position: [f32; 4],
    pub ambient: [f32; 3],
    _pading: [u32; 4],
}

impl CameraUniform {
    fn get_raw(
        projection: &Perspective,
        view: &Isometry,
        ambient: &Rgba,
    ) -> ([[f32; 4]; 4], [[f32; 4]; 4], [f32; 3]) {
        let proj_matrix = *projection.as_matrix().as_ref();
        let view_matrix = view.to_matrix().into();
        let ambient = Color::to_uniform_rgb(ambient);

        (proj_matrix, view_matrix, ambient)
    }

    pub fn new(projection: &Perspective, view: &Isometry, ambient: &Rgba) -> Self {
        let (proj_matrix, view_matrix, ambient) = Self::get_raw(projection, view, ambient);

        Self {
            proj_matrix,
            view_matrix,
            ambient,
            _pading: [0; 4],
        }
    }

    pub fn update(
        &mut self,
        projection: &Perspective,
        view: &Isometry,
        ambient: &Rgba,
        //num_lights: u32,
    ) {
        let (proj_matrix, view_matrix, ambient) = Self::get_raw(projection, view, ambient);

        self.proj_matrix = proj_matrix;
        self.view_matrix = view_matrix;
        self.ambient = ambient;
        //self.num_lights = num_lights;
    }
}

#[derive(Default)]
pub struct CameraUniformUpdate {
    pub dirty: BitSet,
    pub reader_id_proj: Option<ReaderId<ComponentEvent>>,
    pub reader_id_view: Option<ReaderId<ComponentEvent>>,
    pub reader_id_col: Option<ReaderId<ComponentEvent>>,
}

impl CameraUniformUpdate {
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

impl<'a> System<'a> for CameraUniformUpdate {
    type SystemData = (
        Write<'a, DataManager<CameraUniform>>,
        WriteStorage<'a, ArcDataIndex<CameraUniform>>,
        ReadStorage<'a, Projection>,
        ReadStorage<'a, View>,
        ReadStorage<'a, Color>,
    );

    fn run(&mut self, (mut data, mut indices, projection, view, ambient): Self::SystemData) {
        use specs::Join;

        self.dirty.clear();

        let events_proj = projection
            .channel()
            .read(self.reader_id_proj.as_mut().unwrap());
        let events_view = view.channel().read(self.reader_id_view.as_mut().unwrap());
        let events_col = ambient.channel().read(self.reader_id_col.as_mut().unwrap());

        self.event_update(events_proj);
        self.event_update(events_view);
        self.event_update(events_col);

        for (index, projection, view, ambient, _) in
            (&mut indices, &projection, &view, &ambient, &self.dirty).join()
        {
            data.get_mut_index(&index.0.lock().unwrap())
                .update(&projection.0, &view.0, &ambient.0);
        }
    }

    fn setup(&mut self, world: &mut World) {
        Self::SystemData::setup(&self.accessor(), world);
        self.reader_id_proj = Some(world.write_component::<Projection>().register_reader());
        self.reader_id_view = Some(world.write_component::<View>().register_reader());
        self.reader_id_col = Some(world.write_component::<Color>().register_reader());
    }
}
