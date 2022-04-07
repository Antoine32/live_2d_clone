use crate::{
    buffer_update::{ArcDataIndex, DataManager},
    camera::Scale,
    sprite_selector::SpriteSelector,
    type_def::*,
};
use anyhow::Result;
use dashmap::DashMap;
use image::GenericImageView;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use specs::{
    prelude::ComponentEvent, shred::DynamicSystemData, shrev::EventIterator, BitSet, Component,
    ReadStorage, ReaderId, System, VecStorage, World, WorldExt, Write, WriteStorage,
};
use std::{
    ffi::{OsStr, OsString},
    mem,
    path::Path,
    sync::Arc,
};

use crate::texture::{self};

pub const DEFAULT_MATERIAL: &str = "assets/default/materials/default.mtl";

pub const DEFAULT_DIFFUSE: &str = "assets/default/textures/default_diffuse.png";
pub const DEFAULT_NORMAL: &str = "assets/default/textures/default_normal.png";
pub const DEFAULT_SPECULAR: &str = "assets/default/textures/default_specular.png";
pub const DEFAULT_AMBIENT: &str = "assets/default/textures/default_ambient.png";

pub trait Vertex {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute];
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

/*pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        camera_bind_group: &'a wgpu::BindGroup,
        //light_bind_group: &'a wgpu::BindGroup,
        //shadow_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        //light_bind_group: &'a wgpu::BindGroup,
        //shadow_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        //light_bind_group: &'a wgpu::BindGroup,
        //shadow_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        //light_bind_group: &'a wgpu::BindGroup,
        //shadow_bind_group: &'a wgpu::BindGroup,
    );
}*/

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct ModelVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    //pub tangent: [f32; 2],
    //pub bitangent: [f32; 2],
}

impl Vertex for ModelVertex {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x3]; // , 3 => Float32x2, 4 => Float32x2

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
            /*attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],*/
        }
    }
}

impl ModelVertex {
    fn new(
        scale: &Vector,
        sprite_selector: &SpriteSelector,
        material: &Material,
        //indices: &[Indices],
    ) -> Vec<Self> {
        let dimensions = material.diffuse_texture.img.as_ref().unwrap().dimensions();

        let scale_x = scale.x / 250.0;
        let scale_y = scale.y / 250.0;

        let mut size_x = dimensions.0 as f32 * sprite_selector.width;
        let mut size_y = dimensions.1 as f32 * sprite_selector.height;

        let mid_w = material
            .mat
            .unknown_param
            .get("midW")
            .unwrap_or(&String::new())
            .parse::<f32>()
            .unwrap_or(0.0)
            .clamp(0.0, size_x)
            * scale_x;

        let mid_h = material
            .mat
            .unknown_param
            .get("midH")
            .unwrap_or(&String::new())
            .parse::<f32>()
            .unwrap_or(0.0)
            .clamp(0.0, size_y)
            * scale_y;

        size_x *= scale_x;
        size_y *= scale_y;

        let n_half_x = mid_w - size_x;
        let n_half_y = mid_h - size_y;

        let half_x = size_x + n_half_x;
        let half_y = size_y + n_half_y;

        let positions: [[f32; 2]; 4] = [
            [n_half_x, n_half_y],
            [n_half_x, half_y],
            [half_x, half_y],
            [half_x, n_half_y],
        ];
        let tex_coords: [[f32; 2]; 4] = sprite_selector.get_current();

        let vertices = positions
            .par_iter()
            .zip(tex_coords.par_iter())
            .map(|(p, t)| {
                ModelVertex {
                    position: *p,
                    tex_coords: *t,
                    normal: [0.0, 0.0, 1.0],
                    // We'll calculate these later
                    //tangent: [0.0; 2],
                    //bitangent: [0.0; 2],
                }
            })
            .collect::<Vec<_>>();

        /*let mut triangles_included = (0..vertices.len()).collect::<Vec<_>>();

        // Calculate tangents and bitangets. We're going to
        // use the triangles, so we need to loop through the
        // indices in chunks of 3
        for c in indices.chunks(3) {
            let v0 = vertices[c[0] as usize];
            let v1 = vertices[c[1] as usize];
            let v2 = vertices[c[2] as usize];

            let pos0: Vector = v0.position.into();
            let pos1: Vector = v1.position.into();
            let pos2: Vector = v2.position.into();

            let uv0: Vector = v0.tex_coords.into();
            let uv1: Vector = v1.tex_coords.into();
            let uv2: Vector = v2.tex_coords.into();

            // Calculate the edges of the triangle
            let delta_pos1: Vector = pos1 - pos0;
            let delta_pos2: Vector = pos2 - pos0;

            // This will give us a direction to calculate the
            // tangent and bitangent
            let delta_uv1: Vector = uv1 - uv0;
            let delta_uv2: Vector = uv2 - uv0;

            // Solving the following system of equations will
            // give us the tangent and bitangent.
            //     delta_pos1 = delta_uv1.x * T + delta_u.y * B
            //     delta_pos2 = delta_uv2.x * T + delta_uv2.y * B
            // Luckily, the place I found this equation provided
            // the solution!
            let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
            let tangent: Vector = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
            let bitangent: Vector = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * r;

            // We'll use the same tangent/bitangent for each vertex in the triangle
            vertices[c[0] as usize].tangent =
                (tangent + Vector::from(vertices[c[0] as usize].tangent)).into();
            vertices[c[1] as usize].tangent =
                (tangent + Vector::from(vertices[c[1] as usize].tangent)).into();
            vertices[c[2] as usize].tangent =
                (tangent + Vector::from(vertices[c[2] as usize].tangent)).into();
            vertices[c[0] as usize].bitangent =
                (bitangent + Vector::from(vertices[c[0] as usize].bitangent)).into();
            vertices[c[1] as usize].bitangent =
                (bitangent + Vector::from(vertices[c[1] as usize].bitangent)).into();
            vertices[c[2] as usize].bitangent =
                (bitangent + Vector::from(vertices[c[2] as usize].bitangent)).into();

            // Used to average the tangents/bitangents
            triangles_included[c[0] as usize] += 1;
            triangles_included[c[1] as usize] += 1;
            triangles_included[c[2] as usize] += 1;
        }

        // Average the tangents/bitangents
        for (i, n) in triangles_included.into_iter().enumerate() {
            let denom = 1.0 / n as f32;
            let mut v = &mut vertices[i];
            v.tangent = (Vector::from(v.tangent) * denom).normalize().into();
            v.bitangent = (Vector::from(v.bitangent) * denom).normalize().into();
        }*/

        vertices
    }

    fn update(&mut self, other: &Self) {
        *self = *other;
    }
}

#[derive(Default)]
pub struct ModelVertexUpdate {
    pub dirty: BitSet,
    pub reader_id_scale: Option<ReaderId<ComponentEvent>>,
    pub reader_id_sprite_selector: Option<ReaderId<ComponentEvent>>,
}

impl ModelVertexUpdate {
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

impl<'a> System<'a> for ModelVertexUpdate {
    type SystemData = (
        Write<'a, DataManager<ModelVertex>>,
        WriteStorage<'a, ArcDataIndex<ModelVertex>>, // mutable to flag a modification in the flagstorage in order to update its buffer
        ReadStorage<'a, Scale>,
        ReadStorage<'a, SpriteSelector>,
        ReadStorage<'a, Model>,
    );

    fn run(
        &mut self,
        (
            mut vertices_data,
            mut vertices_indices, // mutable to flag a modification in the flagstorage in order to update its buffer
            scales,
            sprite_selectors,
            materials,
        ): Self::SystemData,
    ) {
        use specs::Join;

        self.dirty.clear();

        let events_scales = scales
            .channel()
            .read(self.reader_id_scale.as_mut().unwrap());
        let events_sprite_selector = sprite_selectors
            .channel()
            .read(self.reader_id_sprite_selector.as_mut().unwrap());

        self.event_update(events_scales);
        self.event_update(events_sprite_selector);

        (
            &mut vertices_indices,
            &scales,
            &sprite_selectors,
            &materials,
            &self.dirty,
        )
            .join()
            .for_each(|(vertex_index, scale, sprite_selector, material, _)| {
                let vertex_index = vertex_index.0.lock().unwrap();
                let old_vertices = vertices_data.get_mut_range(&vertex_index);

                let new_vertices = ModelVertex::new(&scale.0, sprite_selector, &material.0);

                old_vertices
                    .par_iter_mut()
                    .zip(new_vertices.par_iter())
                    .for_each(|(old, new)| old.update(new));
            });
    }

    fn setup(&mut self, world: &mut World) {
        Self::SystemData::setup(&self.accessor(), world);
        self.reader_id_scale = Some(world.write_component::<Scale>().register_reader());
        self.reader_id_sprite_selector =
            Some(world.write_component::<SpriteSelector>().register_reader());
    }
}

#[derive(Component, Debug, Clone)]
#[storage(VecStorage)]
pub struct Model(pub Arc<Material>);

impl Model {
    pub fn load<P: AsRef<Path> + std::fmt::Debug + Clone>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        path_mtl: Option<P>,
        path_assets: P,
        textures_map: &mut DashMap<OsString, Arc<texture::Texture>>,
        models_map: &mut DashMap<OsString, (Self, Vec<Indices>, Vec<ModelVertex>, SpriteSelector)>,
    ) -> Result<(Self, Vec<Indices>, Vec<ModelVertex>, SpriteSelector)> {
        let path_assets = path_assets.as_ref();

        let path_mtl = match path_mtl {
            Some(path_mtl) => path_mtl.as_ref().as_os_str().to_os_string(),
            None => OsString::from(DEFAULT_MATERIAL),
        };

        match models_map.get(&path_mtl) {
            Some(model) => Ok(model.clone()),
            None => {
                let (mut obj_materials, _) = tobj::load_mtl(path_mtl.clone())?;
                let mat = obj_materials.pop().unwrap(); // I don't know why it's a Vec, when is there more than one material inside it?

                let material = {
                    let mut textures = [
                        (mat.diffuse_texture.as_str(), DEFAULT_DIFFUSE, false),
                        (mat.normal_texture.as_str(), DEFAULT_NORMAL, true),
                        (mat.specular_texture.as_str(), DEFAULT_SPECULAR, true),
                        (mat.ambient_texture.as_str(), DEFAULT_AMBIENT, true),
                    ]
                    .par_iter()
                    .map(|(texture_path, default, is_normal_map)| {
                        let entry = if texture_path.len() == 0 {
                            OsString::from(default)
                        } else {
                            path_assets.join(texture_path).as_os_str().to_os_string()
                        };

                        match textures_map.get(&entry) {
                            Some(texture) => texture.clone(),
                            None => {
                                let texture =
                                    texture::Texture::load(device, queue, &entry, *is_normal_map);

                                match texture {
                                    Ok(texture) => {
                                        textures_map.insert(entry, texture.clone());

                                        texture
                                    }
                                    Err(e) => {
                                        eprintln!(
                                        "Error : {}. fall back on default texture. |{}|, |{:?}| ",
                                        e, texture_path, entry
                                    );
                                        textures_map.get(OsStr::new(default)).unwrap().clone()
                                    }
                                }
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                    let ambient_texture = textures.pop().unwrap();
                    let specular_texture = textures.pop().unwrap();
                    let normal_texture = textures.pop().unwrap();
                    let diffuse_texture = textures.pop().unwrap();

                    Arc::new(Material::new(
                        device,
                        mat,
                        diffuse_texture,
                        normal_texture,
                        specular_texture,
                        ambient_texture,
                        layout,
                    ))
                };

                let sprite_selector = SpriteSelector::from_mat(&material.mat.unknown_param);

                let indices = vec![0, 1, 2, 0, 2, 3]; // 0, 1, 2, 0, 2, 3 // 0, 2, 1, 0, 3, 2
                                                      //let indices = vec![0, 2, 1, 0, 3, 2];
                let vertices =
                    ModelVertex::new(&Vector::new(1.0, 1.0), &sprite_selector, &material);

                let result = (Self(material), indices, vertices, sprite_selector);
                models_map.insert(path_mtl, result.clone());

                Ok(result)
            }
        }
    }
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Material {
    pub mat: tobj::Material,
    pub diffuse_texture: Arc<texture::Texture>,
    pub normal_texture: Arc<texture::Texture>,
    pub specular_texture: Arc<texture::Texture>,
    pub ambient_texture: Arc<texture::Texture>,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn new(
        device: &wgpu::Device,
        mat: tobj::Material,
        diffuse_texture: Arc<texture::Texture>,
        normal_texture: Arc<texture::Texture>,
        specular_texture: Arc<texture::Texture>,
        ambient_texture: Arc<texture::Texture>,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&specular_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&specular_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&ambient_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&ambient_texture.sampler),
                },
            ],
            label: Some(&mat.name),
        });

        Self {
            mat,
            diffuse_texture,
            normal_texture,
            specular_texture,
            ambient_texture,
            bind_group,
        }
    }
}

/*#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Mesh {
    pub name: String,
    //pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    //pub vertices: Vec<ModelVertex>,
    pub indices: Vec<u32>,
    pub num_elements: u32,
}*/

/*impl Mesh {
    pub fn clone(&self, device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", self.name)),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", self.name)),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            name: self.name.clone(),
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            vertices: self.vertices.clone(),
            indices: self.indices.clone(),
            num_elements: self.num_elements,
            material: self.material,
        }
    }
}*/

/*impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        camera_bind_group: &'b wgpu::BindGroup,
        //light_bind_group: &'b wgpu::BindGroup,
        //shadow_bind_group: &'a wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(
            mesh,
            material,
            0..1,
            camera_bind_group,
            //light_bind_group,
            //shadow_bind_group,
        );
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        //light_bind_group: &'b wgpu::BindGroup,
        //shadow_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        //self.set_bind_group(2, light_bind_group, &[]);
        //self.set_bind_group(3, shadow_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(
        &mut self,
        model: &'b Model,
        camera_bind_group: &'b wgpu::BindGroup,
        //light_bind_group: &'b wgpu::BindGroup,
        //shadow_bind_group: &'a wgpu::BindGroup,
    ) {
        self.draw_model_instanced(
            model,
            0..1,
            camera_bind_group,
            //light_bind_group,
            //shadow_bind_group,
        );
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        //light_bind_group: &'b wgpu::BindGroup,
        //shadow_bind_group: &'a wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(
            &model.mesh,
            &model.material,
            instances.clone(),
            camera_bind_group,
            //light_bind_group,
            //shadow_bind_group,
        );
    }
}*/
