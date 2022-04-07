use palette::rgb::Rgba;
use specs::{
    prelude::ComponentEvent, shred::DynamicSystemData, shrev::EventIterator, BitSet, Component,
    DenseVecStorage, FlaggedStorage, Read, ReadStorage, ReaderId, System, VecStorage, World,
    WorldExt, WriteStorage,
};

use crate::{
    actor::Queue,
    camera::{Color, Position, View},
    Isometry, Point,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    //pub proj_matrix: [[f32; 4]; 4],
    pub view_matrix: [[f32; 3]; 3],
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub intensity: f32,
    pub spread: f32,
    pub specular: f32, // specular_lobe_factor
    pub bias: f32,     // acne_bias
}

impl Component for LightUniform {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl LightUniform {
    fn get_raw(
        //projection: &Perspective,
        view: &Isometry,
        position: &Point,
        color: &Rgba,
    ) -> ([[f32; 3]; 3], [f32; 3], [f32; 4]) {
        //let proj_matrix = *projection.as_matrix().as_ref();
        let view_matrix = view.to_matrix().into();
        let position = position.to_homogeneous().into();
        let color = Color::to_uniform_rgba(color);

        (view_matrix, position, color)
    }

    pub fn new(
        //projection: &Perspective,
        view: &Isometry,
        position: &Point,
        color: &Rgba,
        intensity: f32,
        spread: f32,
        specular: f32,
        bias: f32,
    ) -> Self {
        let (view_matrix, position, color) = Self::get_raw(view, position, color);

        Self {
            //proj_matrix,
            view_matrix,
            position,
            color,
            intensity,
            spread,
            specular,
            bias,
        }
    }

    pub fn update(
        &mut self,
        //projection: &Perspective,
        view: &Isometry,
        position: &Point,
        color: &Rgba,
    ) {
        let (view_matrix, position, color) = Self::get_raw(view, position, color);

        //self.proj_matrix = proj_matrix;
        self.view_matrix = view_matrix;
        self.position = position;
        self.color = color;
    }
}

#[derive(Debug)]
pub struct LightBuffer(pub wgpu::Buffer);

impl Default for LightBuffer {
    fn default() -> Self {
        todo!()
    }
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct LightOffset(pub wgpu::BufferAddress);

impl LightOffset {
    pub fn new(offset: wgpu::BufferAddress) -> Self {
        Self { 0: offset }
    }
}

#[derive(Default)]
pub struct LightUniformUpdate {
    pub dirty: BitSet,
    pub reader_id_pos: Option<ReaderId<ComponentEvent>>,
    pub reader_id_view: Option<ReaderId<ComponentEvent>>,
    pub reader_id_col: Option<ReaderId<ComponentEvent>>,
    pub reader_id_light: Option<ReaderId<ComponentEvent>>,
}

impl LightUniformUpdate {
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

impl<'a> System<'a> for LightUniformUpdate {
    type SystemData = (
        WriteStorage<'a, LightUniform>,
        ReadStorage<'a, LightOffset>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, View>,
        ReadStorage<'a, Color>,
        Read<'a, LightBuffer>,
        Read<'a, Queue>,
    );

    fn run(
        &mut self,
        (mut light, light_offset, position, view, color, buffer, queue): Self::SystemData,
    ) {
        use specs::Join;

        self.dirty.clear();

        let events_pos = position
            .channel()
            .read(self.reader_id_pos.as_mut().unwrap());
        let events_view = view.channel().read(self.reader_id_view.as_mut().unwrap());
        let events_col = color.channel().read(self.reader_id_col.as_mut().unwrap());

        self.event_update(events_pos);
        self.event_update(events_view);
        self.event_update(events_col);

        light.set_event_emission(false);

        for (light, position, view, color, _) in
            (&mut light, &position, &view, &color, &self.dirty).join()
        {
            light.update(&view.0, &position.0, &color.0);
        }

        light.set_event_emission(true);

        let events_light = light.channel().read(self.reader_id_light.as_mut().unwrap());

        self.event_update(events_light);

        for (light, light_offset, _) in (&mut light, &light_offset, &self.dirty).join() {
            queue
                .0
                .write_buffer(&buffer.0, light_offset.0, bytemuck::cast_slice(&[*light]));
        }
    }

    fn setup(&mut self, world: &mut World) {
        Self::SystemData::setup(&self.accessor(), world);
        self.reader_id_pos = Some(world.write_component::<Position>().register_reader());
        self.reader_id_view = Some(world.write_component::<View>().register_reader());
        self.reader_id_col = Some(world.write_component::<Color>().register_reader());
        self.reader_id_light = Some(world.write_component::<LightUniform>().register_reader());
    }
}
