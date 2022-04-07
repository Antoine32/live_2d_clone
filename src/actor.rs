use crate::{
    buffer_update::{ArcDataIndex, DataBuffer, DataManager},
    camera::Projection,
    camera_controller::CameraController,
    camera_uniform::CameraUniform,
    instance_uniform::InstanceUniform,
    model::*,
    texture::{self},
    type_def::*,
};
use dashmap::DashMap;
use specs::{
    Component, DenseVecStorage, Entity, FlaggedStorage, Read, ReadStorage, System, Write,
    WriteStorage,
};
use specs_hierarchy::Parent as HParent; // Hierarchy, HierarchySystem,
use std::{sync::Arc, time::Duration};
use winit::event::{DeviceEvent, ElementState, MouseScrollDelta, VirtualKeyCode};

#[derive(Debug, Component, Clone)]
pub struct Pipeline(pub Arc<wgpu::RenderPipeline>);

struct Parent {
    entity: Entity,
}

impl Component for Parent {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl HParent for Parent {
    fn parent_entity(&self) -> Entity {
        self.entity
    }
}

// #[derive(Debug, Default)]
// pub struct InstanceData(pub Vec<InstanceRaw>);

#[derive(Debug)]
pub struct Queue(pub wgpu::Queue);

impl Default for Queue {
    fn default() -> Self {
        todo!()
    }
}

#[derive(Debug)]
pub struct Device(pub wgpu::Device);

impl Default for Device {
    fn default() -> Self {
        todo!()
    }
}

pub struct SmaaTarget(pub smaa::SmaaTarget);

impl Default for SmaaTarget {
    fn default() -> Self {
        todo!()
    }
}

#[derive(Debug, Default)]
pub struct Collections {
    pub render_pipelines: Vec<Arc<wgpu::RenderPipeline>>,
    pub models: Vec<Arc<Model>>,
}

#[derive(Debug, Default)]
pub struct DeviceInfo {
    pub key_map: DashMap<VirtualKeyCode, ElementState>,
    pub keycode_map: DashMap<u32, ElementState>,
    pub mouse_map: DashMap<u32, ElementState>,
    pub scroll_delta: Option<MouseScrollDelta>,
    pub motion_delta: (f64, f64),
}

#[derive(Debug, Default)]
pub struct EventInput {
    pub events: Vec<DeviceEvent>,
    pub info: DeviceInfo,
}

#[derive(Debug, Default)]
pub struct ResizeValue(pub Option<winit::dpi::PhysicalSize<u32>>);

#[derive(Debug, Default)]
pub struct ControlFlow(pub Option<winit::event_loop::ControlFlow>);

#[derive(Debug, Default)]
pub struct Time {
    pub delta: Duration,
    pub speed: f64,
    pub on: bool, // remove this shit
}

// #[derive(Component, Debug)]
// #[storage(VecStorage)]
pub struct RenderThings {
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub depth_texture: texture::Texture,
    pub camera_bind_group: wgpu::BindGroup,
    //pub light_bind_group: wgpu::BindGroup,
    //pub shadow_pipeline: wgpu::RenderPipeline,
    //pub shadow_bind_group: wgpu::BindGroup,
    //pub shadow_buffer: wgpu::Buffer,
}

impl Default for RenderThings {
    fn default() -> Self {
        todo!()
    }
}

pub struct ProcessEvents;

impl<'a> System<'a> for ProcessEvents {
    type SystemData = (Write<'a, EventInput>, WriteStorage<'a, CameraController>);

    fn run(&mut self, (mut event_input, mut camera_controllers): Self::SystemData) {
        use specs::Join;

        for camera_controller in (&mut camera_controllers).join() {
            for i in 0..event_input.events.len() {
                while i < event_input.events.len()
                    && CameraController::process_event(
                        camera_controller,
                        &event_input.events[i],
                        &event_input.info,
                    )
                {
                    event_input.events.remove(i);
                }
            }
        }
    }
}

pub struct UpdateSize;

impl<'a> System<'a> for UpdateSize {
    type SystemData = (
        Write<'a, ResizeValue>,
        Write<'a, RenderThings>,
        Write<'a, SmaaTarget>,
        Read<'a, Device>,
        WriteStorage<'a, Projection>,
        ReadStorage<'a, ArcDataIndex<CameraUniform>>,
    );

    fn run(
        &mut self,
        (
            mut resize_value,
            mut render_things,
            mut smaa_target,
            device,
            mut projection,
            camera_indices,
        ): Self::SystemData,
    ) {
        use specs::Join;

        match resize_value.0.take() {
            Some(new_size) => {
                render_things.size = new_size;
                render_things.config.width = new_size.width;
                render_things.config.height = new_size.height;
                render_things
                    .surface
                    .configure(&device.0, &render_things.config);

                smaa_target.0.resize(
                    &device.0,
                    render_things.config.width,
                    render_things.config.height,
                );

                render_things.depth_texture = texture::Texture::create_depth_texture(
                    &device.0,
                    Some("Depth Texture"),
                    (render_things.config.width, render_things.config.height),
                );

                for (projection, _camera) in (&mut projection, &camera_indices).join() {
                    projection.0.set_aspect(
                        render_things.config.width as f32 / render_things.config.height as f32,
                    );
                }
            }
            None => {}
        }
    }
}

pub struct Rendering;

impl<'a> System<'a> for Rendering {
    type SystemData = (
        ReadStorage<'a, Model>,
        ReadStorage<'a, Pipeline>,
        //
        //ReadStorage<'a, LightUniform>,
        //ReadStorage<'a, TextureView>,
        //ReadStorage<'a, LightOffset>,
        //Read<'a, LightBuffer>,
        //
        ReadStorage<'a, ArcDataIndex<Indices>>,
        Read<'a, DataBuffer<Indices>>,
        Read<'a, DataManager<Indices>>,
        //
        ReadStorage<'a, ArcDataIndex<ModelVertex>>,
        Read<'a, DataBuffer<ModelVertex>>,
        Read<'a, DataManager<ModelVertex>>,
        //
        ReadStorage<'a, ArcDataIndex<InstanceUniform>>,
        Read<'a, DataBuffer<InstanceUniform>>,
        Read<'a, DataManager<InstanceUniform>>,
        //
        Read<'a, RenderThings>,
        Read<'a, Queue>,
        Read<'a, Device>,
        Write<'a, ResizeValue>,
        Write<'a, ControlFlow>,
        Write<'a, SmaaTarget>,
    );

    fn run(
        &mut self,
        (
            materials,
            pipelines,
            //
            //_lights,
            //_texture_views,
            //_light_offsets,
            //_light_buffer,
            //
            indices_indices,
            indices_buffer,
            indices_data,
            //
            vertices_indices,
            vertices_buffer,
            vertices_data,
            //
            instances_indices,
            instances_buffer,
            instances_data,
            //
            render_things,
            queue,
            device,
            mut resize_value,
            mut control_flow,
            mut smaa_target,
        ): Self::SystemData,
    ) {
        use specs::Join;

        match render_things.surface.get_current_texture() {
            Ok(output) => {
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let view = smaa_target.0.start_frame(&device.0, &queue.0, &view);

                let mut encoder =
                    device
                        .0
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });

                /*let mut actors_indices = (&actors, &instance_indices).join().collect::<Vec<_>>();

                actors_indices.sort_by(|&(_actor_a, index_a), &(_actor_b, index_b)| {
                    let a = &index_a.0.lock().unwrap();
                    let b = &index_b.0.lock().unwrap();
                    (a.index
                        + if a.active {
                            instance_data.idle_data.len()
                        } else {
                            0
                        })
                    .cmp(
                        &(b.index
                            + if b.active {
                                instance_data.idle_data.len()
                            } else {
                                0
                            }),
                    )
                });

                let (actors, _): (Vec<&Actor>, Vec<&ArcDataIndex<_>>) =
                    actors_indices.into_iter().unzip();*/

                /*let mut lights = (&lights, &texture_views, &light_offsets)
                    .join()
                    .collect::<Vec<_>>();
                lights.sort_by(|&a, &b| a.2 .0.cmp(&b.2 .0));

                for (_light, texture_view, light_offset) in lights.iter() {
                    {
                        encoder.copy_buffer_to_buffer(
                            &light_buffer.0,
                            light_offset.0,
                            &render_things.shadow_buffer,
                            0,
                            128,
                        );

                        let mut render_pass =
                            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Shadow Pass"),
                                color_attachments: &[],
                                depth_stencil_attachment: Some(
                                    wgpu::RenderPassDepthStencilAttachment {
                                        view: &texture_view.0,
                                        depth_ops: Some(wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(1.0),
                                            store: true,
                                        }),
                                        stencil_ops: None,
                                    },
                                ),
                            });

                        render_pass.set_index_buffer(
                            indices_buffer.buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );

                        render_pass.set_vertex_buffer(0, vertices_buffer.buffer.slice(..));
                        render_pass.set_vertex_buffer(1, instances_buffer.buffer.slice(..));

                        render_pass.set_bind_group(0, &render_things.shadow_bind_group, &[]);

                        render_pass.set_pipeline(&render_things.shadow_pipeline);

                        for (index_index, vertex_index, instance_index) in
                            (&indices_indices, &vertices_indices, &instances_indices).join()
                        {
                            let i = index_index.get_range::<u32>(&indices_data, true);
                            let j = vertex_index.get_array_index::<i32>(&vertices_data, true);
                            let k = instance_index.get_range::<u32>(&instances_data, true);

                            render_pass.draw_indexed(i, j, k);
                        }
                    }
                }*/

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.5,
                                    g: 0.5,
                                    b: 0.5,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &render_things.depth_texture.view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: true,
                            }),
                            stencil_ops: None,
                        }),
                    });

                    render_pass.set_index_buffer(
                        indices_buffer.buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );

                    render_pass.set_vertex_buffer(0, vertices_buffer.buffer.slice(..));
                    render_pass.set_vertex_buffer(1, instances_buffer.buffer.slice(..));

                    render_pass.set_bind_group(1, &render_things.camera_bind_group, &[]);
                    //render_pass.set_bind_group(2, &render_things.light_bind_group, &[]);

                    for (material, pipeline, index_index, vertex_index, instance_index) in (
                        &materials,
                        &pipelines,
                        &indices_indices,
                        &vertices_indices,
                        &instances_indices,
                    )
                        .join()
                    {
                        let i = index_index.get_range::<u32>(&indices_data, true);
                        let j = vertex_index.get_array_index::<i32>(&vertices_data, true);
                        let k = instance_index.get_range::<u32>(&instances_data, true);

                        render_pass.set_pipeline(&pipeline.0);
                        render_pass.set_bind_group(0, &material.0.bind_group, &[]);

                        render_pass.draw_indexed(i, j, k);
                    }
                }

                queue.0.submit(std::iter::once(encoder.finish()));

                view.resolve();
                output.present();
            }
            // Reconfigure the surface if lost
            Err(wgpu::SurfaceError::Lost) => resize_value.0 = Some(render_things.size),
            // The system is out of memory, we should probably quit
            Err(wgpu::SurfaceError::OutOfMemory) => {
                control_flow.0 = Some(winit::event_loop::ControlFlow::Exit)
            }
            // All other errors (Outdated, Timeout) should be resolved by the next frame
            Err(e) => eprintln!("{:?}", e),
        }
    }
}
