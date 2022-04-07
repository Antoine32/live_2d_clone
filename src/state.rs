use std::{
    ffi::OsString,
    path::PathBuf,
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

use crate::{
    actor::*,
    buffer_update::{ArcDataIndex, DataBuffer, DataBufferUpdater, DataIndex, DataManager},
    camera::*,
    camera_controller::*,
    camera_uniform::{CameraUniform, CameraUniformUpdate},
    collider::ColliderHandle,
    deg, fs,
    instance_uniform::{InstanceUniform, InstanceUniformUpdate},
    model::{self, *},
    rigid_body::RigidBodyHandle,
    sprite_selector::*,
    texture::{self},
};
use dashmap::DashMap;
use rapier2d::prelude::{ColliderBuilder, ColliderSet, RigidBodyBuilder, RigidBodySet};
use smaa::SmaaMode;
use specs::{Builder, Dispatcher, DispatcherBuilder, World, WorldExt};
use wgpu::util::DeviceExt;
use winit::{
    event::{DeviceEvent, KeyboardInput},
    window::Window,
};

pub struct AssetsDir(pub PathBuf);

pub struct State {
    pub world: World,
    pub dispatcher: Dispatcher<'static, 'static>,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let mut world = World::new();

        world.register::<Model>();
        world.register::<Pipeline>();
        world.register::<SpriteSelector>();
        world.register::<ArcDataIndex<Indices>>();
        world.register::<ArcDataIndex<ModelVertex>>();
        world.register::<ArcDataIndex<InstanceUniform>>();
        world.register::<ArcDataIndex<CameraUniform>>();
        //world.register::<CameraOffset>();
        //world.register::<CameraUniform>();
        world.register::<CameraController>();
        // world.register::<LightOffset>();
        // world.register::<LightUniform>();
        world.register::<Position>();
        world.register::<Rotation>();
        world.register::<Scale>();
        world.register::<View>();
        world.register::<Color>();
        world.register::<Translation>();
        //world.register::<TextureView>();
        world.register::<Projection>();

        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        /*let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();*/
        let adapter = instance
            .enumerate_adapters(wgpu::Backends::all())
            .find(|adapter| {
                let info = adapter.get_info();
                info.device_type == wgpu::DeviceType::DiscreteGpu
                    && info.backend == wgpu::Backend::Vulkan
            })
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::DEPTH_CLIP_CONTROL | wgpu::Features::PUSH_CONSTANTS, // PUSH_CONSTANTS only work on Vulkan not on Dx12 or others
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let supports_storage_resources = adapter
            .get_downlevel_properties()
            .flags
            .contains(wgpu::DownlevelFlags::VERTEX_STORAGE)
            && device.limits().max_storage_buffers_per_shader_stage > 0;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(), // wgpu::TextureFormat::Rgba8UnormSrgb
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Immediate, // Fifo
        };
        surface.configure(&device, &config);

        let smaa_target = smaa::SmaaTarget::new(
            &device,
            &queue,
            config.width,
            config.height,
            config.format,
            SmaaMode::Smaa1X,
        );

        let depth_texture = texture::Texture::create_depth_texture(
            &device,
            Some("Depth Texture"),
            (config.width, config.height),
        );

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            // SamplerBindingType::Comparison is only for TextureSampleType::Depth
                            // SamplerBindingType::Filtering if the sample_type of the texture is:
                            // TextureSampleType::Float { filterable: true }
                            // Otherwise you'll get an error.
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let mut cameras_appended = 0;
        let mut cameras_data: Vec<CameraUniform> = Vec::new();
        let mut cameras_indices: Vec<Weak<Mutex<DataIndex<CameraUniform>>>> = Vec::new();

        {
            let position = Position::new(0.0, 0.0);
            let rotation = Rotation::new(deg(0.0));

            let projection = Projection::new(
                config.width as f32 / config.height as f32,
                deg(45.0),
                0.1,
                100.0,
            );

            let view = View::new(&position.0, &rotation.0);

            let ambient = Color::new_rgb(1.0, 1.0, 1.0);

            let camera_index = ArcDataIndex::<CameraUniform>::new(
                cameras_appended,
                cameras_data.len(),
                1,
                Duration::from_millis(1000),
            );
            //CameraOffset::new((camera_vec.len() * std::mem::size_of::<CameraUniform>()) as u64);

            let camera_uniform = CameraUniform::new(&projection.0, &view.0, &ambient.0);

            let camera_controller = CameraController::new(0.5, 1.0);

            cameras_appended += 1;
            cameras_data.push(camera_uniform.clone());
            cameras_indices.push(Arc::downgrade(&camera_index.0));

            world
                .create_entity()
                .with(camera_index)
                .with(position)
                .with(rotation)
                .with(view)
                .with(camera_controller)
                .with(ambient)
                .with(projection)
                .build();
        }

        let cameras_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&cameras_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cameras_buffer.as_entire_binding(),
            }],
        });

        world.insert(DataBuffer::<CameraUniform>::new(cameras_buffer));
        world.insert(DataManager::new(
            cameras_data,
            cameras_indices,
            cameras_appended,
        ));

        /*let shadow_texture =
            texture::Texture::create_shadow_texture(&device, Some("Shadow Texture"));

        let light_uniform_size = MAX_LIGHTS as wgpu::BufferAddress
            * std::mem::size_of::<LightUniform>() as wgpu::BufferAddress;

        let light_storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light Storage Buffer"),
            size: light_uniform_size,
            usage: if supports_storage_resources {
                wgpu::BufferUsages::STORAGE
            } else {
                wgpu::BufferUsages::UNIFORM
            } | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let mut lights_vec = Vec::new();

        let rot = vec![(Position::new(2.0, 0.0), Rotation::new(deg(0.0)))];

        for (position, rotation) in rot.into_iter() {
            let view = View::new(&position.0, &rotation.0);

            /*let projection = Projection::new(
                SHADOW_SIZE as f32 / SHADOW_SIZE as f32,
                deg(90.0),
                0.1,
                20.0,
            );*/

            let color = Color::new_rgb(1.0, 1.0, 1.0);

            let light_uniform =
                LightUniform::new(&view.0, &position.0, &color.0, 50.0, 1.5, 8.0, 0.02);

            let light_offset =
                LightOffset::new((lights_vec.len() * std::mem::size_of::<LightUniform>()) as u64);

            let texture_view = TextureView::new(
                &shadow_texture.texture,
                &wgpu::TextureViewDescriptor {
                    label: Some("shadow"),
                    format: None,
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: lights_vec.len() as u32,
                    array_layer_count: NonZeroU32::new(1),
                },
            );

            lights_vec.push(light_uniform);

            world
                .create_entity()
                .with(light_uniform)
                .with(position)
                .with(rotation)
                .with(view)
                .with(color)
                .with(light_offset)
                .with(texture_view)
                .build();
        }

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: if supports_storage_resources {
                                wgpu::BufferBindingType::Storage { read_only: true }
                            } else {
                                wgpu::BufferBindingType::Uniform
                            },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(light_uniform_size),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_storage_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_texture.sampler),
                },
            ],
            label: None,
        });

        let shadow_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light Storage Buffer"),
            size: std::mem::size_of::<LightUniform>() as _,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<LightUniform>() as _,
                        ),
                    },
                    count: None,
                }],
            });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &shadow_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_buffer.as_entire_binding(),
            }],
            label: None,
        });

        world.insert(LightBuffer {
            0: light_storage_buffer,
        });*/

        ////////////////////////////////////////////////////////////////////////////////////////////////////////

        let assets_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");

        /*let shadow_pipeline = {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[&shadow_bind_group_layout],
                push_constant_ranges: &[],
            });
            let shader = {
                let contents = fs::load_file(assets_dir.join("shaders/shadow.wgsl")).unwrap();
                device.create_shader_module(&wgpu::ShaderModuleDescriptor {
                    label: Some("Shadow Shader"),
                    source: wgpu::ShaderSource::Wgsl(contents.into()),
                })
            };

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Shadow Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_bake",
                    buffers: &[model::ModelVertex::desc(), InstanceUniform::desc()], // , LightUniform::desc()
                },
                fragment: None,
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: device
                        .features()
                        .contains(wgpu::Features::DEPTH_CLIP_CONTROL),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::Texture::SHADOW_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2, // corresponds to bilinear filtering
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            })
        };*/

        let render_pipeline = Arc::new({
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                    //&light_bind_group_layout,
                    //&shadow_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
            let shader = {
                let contents = fs::load_file(assets_dir.join("shaders/test.wgsl")).unwrap();
                device.create_shader_module(&wgpu::ShaderModuleDescriptor {
                    label: Some("View Shader"),
                    source: wgpu::ShaderSource::Wgsl(contents.into()),
                })
            };

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[model::ModelVertex::desc(), InstanceUniform::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: if supports_storage_resources {
                        "fs_main"
                    } else {
                        "fs_main_without_storage"
                    },
                    targets: &[wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::Zero, // OneMinusSrcAlpha
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: device
                        .features()
                        .contains(wgpu::Features::DEPTH_CLIP_CONTROL),
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: true, // true
                },
                multiview: None,
            })
        });

        let render_pipelines = vec![Pipeline(render_pipeline)];

        let mut textures_map: DashMap<OsString, Arc<texture::Texture>> = DashMap::new();
        let mut models_map: DashMap<
            OsString,
            (Model, Vec<Indices>, Vec<ModelVertex>, SpriteSelector),
        > = DashMap::new();

        let models: Vec<(Model, Vec<Indices>, Vec<ModelVertex>, SpriteSelector)> = vec![
            {
                let (material, indices, vertices, sprite_selector) = model::Model::load(
                    &device,
                    &queue,
                    &texture_bind_group_layout,
                    None,
                    assets_dir.clone(),
                    &mut textures_map,
                    &mut models_map,
                )
                .unwrap();

                (material, indices, vertices, sprite_selector)
            },
            {
                let (material, indices, vertices, sprite_selector) = model::Model::load(
                    &device,
                    &queue,
                    &texture_bind_group_layout,
                    None,
                    assets_dir.clone(),
                    &mut textures_map,
                    &mut models_map,
                )
                .unwrap();

                (material, indices, vertices, sprite_selector)
            },
            /*{
                let (material, indices, vertices, sprite_selector) = model::Model::load(
                    &device,
                    &queue,
                    &texture_bind_group_layout,
                    Some(assets_dir.join("default/materials/default.mtl")),
                    assets_dir.clone(),
                    &mut textures_map,
                    &mut models_map,
                )
                .unwrap();

                (material, indices, vertices, sprite_selector)
            },*/
        ];

        {
            let mut indices_appended = 0;
            let mut indices_data: Vec<Indices> = Vec::new();
            let mut indices_indices: Vec<Weak<Mutex<DataIndex<Indices>>>> = Vec::new();

            let mut vertices_appended = 0;
            let mut vertices_data: Vec<ModelVertex> = Vec::new();
            let mut vertices_indices: Vec<Weak<Mutex<DataIndex<ModelVertex>>>> = Vec::new();

            let mut instances_appended = 0;
            let mut instances_data: Vec<InstanceUniform> = Vec::new();
            let mut instances_indices: Vec<Weak<Mutex<DataIndex<InstanceUniform>>>> = Vec::new();

            let mut rb_set = RigidBodySet::new();
            let mut coll_set = ColliderSet::new();

            for x in -2..=2 as i32 {
                for y in -2..=2 as i32 {
                    let position = Position::new(x as f32 * 0.25, y as f32 * 0.25);
                    let rotation = Rotation::new(deg(0.0));
                    let scale = Scale::new(1.0, 1.0);

                    let (model, mut indices, mut vertices, sprite_selector) =
                        models[(y + x + 4) as usize % 2].clone();

                    let indices_index: ArcDataIndex<Indices> = ArcDataIndex::new(
                        indices_appended,
                        indices_data.len(),
                        indices.len(),
                        Duration::from_millis(1000),
                    );

                    indices_appended += 1;
                    indices_data.append(&mut indices);
                    indices_indices.push(Arc::downgrade(&indices_index.0));

                    let vertices_index: ArcDataIndex<ModelVertex> = ArcDataIndex::new(
                        vertices_appended,
                        vertices_data.len(),
                        vertices.len(),
                        Duration::from_millis(1000),
                    );

                    vertices_appended += 1;
                    vertices_data.append(&mut vertices);
                    vertices_indices.push(Arc::downgrade(&vertices_index.0));

                    let instance_uniform = InstanceUniform::new(&rotation.0, &position.0);
                    let instance_index: ArcDataIndex<InstanceUniform> = ArcDataIndex::new(
                        instances_appended,
                        instances_data.len(),
                        1,
                        Duration::from_millis(1000),
                    );

                    instances_appended += 1;
                    instances_data.push(instance_uniform);
                    instances_indices.push(Arc::downgrade(&instance_index.0));

                    world
                        .create_entity()
                        .with(model)
                        .with(render_pipelines[0].clone())
                        .with(position)
                        .with(rotation)
                        .with(scale)
                        .with(sprite_selector)
                        .with(indices_index)
                        .with(vertices_index)
                        .with(instance_index)
                        .build();
                }
                for y in -2..=2 as i32 {
                    let position = Position::new(x as f32 * 0.25, y as f32 * 0.25);
                    let rotation = Rotation::new(deg(0.0));
                    let scale = Scale::new(1.0, 1.0);

                    let (model, mut indices, mut vertices, sprite_selector) =
                        models[(y + x + 4) as usize % 2].clone();

                    let indices_index: ArcDataIndex<Indices> = ArcDataIndex::new(
                        indices_appended,
                        indices_data.len(),
                        indices.len(),
                        Duration::from_millis(1000),
                    );

                    indices_appended += 1;
                    indices_data.append(&mut indices);
                    indices_indices.push(Arc::downgrade(&indices_index.0));

                    let vertices_index: ArcDataIndex<ModelVertex> = ArcDataIndex::new(
                        vertices_appended,
                        vertices_data.len(),
                        vertices.len(),
                        Duration::from_millis(1000),
                    );

                    vertices_appended += 1;
                    vertices_data.append(&mut vertices);
                    vertices_indices.push(Arc::downgrade(&vertices_index.0));

                    let instance_uniform = InstanceUniform::new(&rotation.0, &position.0);
                    let instance_index: ArcDataIndex<InstanceUniform> = ArcDataIndex::new(
                        instances_appended,
                        instances_data.len(),
                        1,
                        Duration::from_millis(1000),
                    );

                    instances_appended += 1;
                    instances_data.push(instance_uniform);
                    instances_indices.push(Arc::downgrade(&instance_index.0));

                    /*let rb = RigidBodyBuilder::new_dynamic()
                        .translation(position.0.coords)
                        .rotation(rotation.0.angle())
                        .build();

                    let coll = ColliderBuilder::cuboid(
                        (vertices[2].position[0] - vertices[0].position[0]).abs() * scale.0.x,
                        (vertices[1].position[1] - vertices[0].position[1]).abs() * scale.0.y,
                    )
                    .build();

                    let rb_handle = rb_set.insert(rb);
                    let coll_handle = coll_set.insert_with_parent(coll, rb_handle, &mut rb_set);*/

                    world
                        .create_entity()
                        .with(model)
                        .with(render_pipelines[0].clone())
                        .with(position)
                        .with(rotation)
                        .with(scale)
                        //.with(RigidBodyHandle(rb_handle))
                        //.with(ColliderHandle(coll_handle))
                        .with(sprite_selector)
                        .with(indices_index)
                        .with(vertices_index)
                        .with(instance_index)
                        .with(Translation::new(0.0, 0.0))
                        .build();
                }
            }

            /*{
                let position = Position::new(0.0, 5.0);
                let rotation = Rotation::new(deg(0.0));
                let scale = Scale::new(100.0, 1.0);
                let collider = Collider::new(&ColliderBuilder::cuboid(0.5, 0.5));

                let (model, mut indices, mut vertices, sprite_selector) = models[2].clone();

                let indices_index: ArcDataIndex<Indices> = ArcDataIndex::new(
                    indices_appended,
                    indices_data.len(),
                    indices.len(),
                    Duration::from_millis(1000),
                );

                indices_appended += 1;
                indices_data.append(&mut indices);
                indices_indices.push(Arc::downgrade(&indices_index.0));

                let vertices_index: ArcDataIndex<ModelVertex> = ArcDataIndex::new(
                    vertices_appended,
                    vertices_data.len(),
                    vertices.len(),
                    Duration::from_millis(1000),
                );

                vertices_appended += 1;
                vertices_data.append(&mut vertices);
                vertices_indices.push(Arc::downgrade(&vertices_index.0));

                let instance_uniform = InstanceUniform::new(&rotation.0, &position.0);
                let instance_index: ArcDataIndex<InstanceUniform> = ArcDataIndex::new(
                    instances_appended,
                    instances_data.len(),
                    1,
                    Duration::from_millis(1000),
                );

                instances_appended += 1;
                instances_data.push(instance_uniform);
                instances_indices.push(Arc::downgrade(&instance_index.0));

                world
                    .create_entity()
                    .with(model)
                    .with(render_pipelines[0].clone())
                    .with(position)
                    .with(rotation)
                    .with(scale)
                    .with(collider)
                    .with(sprite_selector)
                    .with(indices_index)
                    .with(vertices_index)
                    .with(instance_index)
                    .build();
            }*/

            let indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Indices Buffer"),
                contents: bytemuck::cast_slice(&indices_data),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

            let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertices Buffer"),
                contents: bytemuck::cast_slice(&vertices_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            let instances_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instances Buffer"),
                contents: bytemuck::cast_slice(&instances_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            world.insert(DataBuffer::<Indices>::new(indices_buffer));
            world.insert(DataBuffer::<ModelVertex>::new(vertices_buffer));
            world.insert(DataBuffer::<InstanceUniform>::new(instances_buffer));

            world.insert(DataManager::new(
                indices_data,
                indices_indices,
                indices_appended,
            ));
            world.insert(DataManager::new(
                vertices_data,
                vertices_indices,
                vertices_appended,
            ));
            world.insert(DataManager::new(
                instances_data,
                instances_indices,
                instances_appended,
            ));
        }

        /*world.insert(Collections {
            render_pipelines,
            models: models.into_iter().unzip().0,
        });*/

        world.insert(AssetsDir(assets_dir));

        world.insert(RenderThings {
            surface,
            config,
            size,
            depth_texture,
            camera_bind_group,
            // light_bind_group,
            // shadow_pipeline,
            // shadow_bind_group,
            // shadow_buffer,
        });

        world.insert(SmaaTarget { 0: smaa_target });

        world.insert(Device { 0: device });

        world.insert(Queue { 0: queue });

        world.insert(ResizeValue { 0: None });

        world.insert(ControlFlow { 0: None });

        world.insert(Time {
            delta: std::time::Instant::now().elapsed(),
            speed: 1.0,
            on: false,
        });

        world.insert(EventInput {
            events: Vec::new(),
            info: DeviceInfo {
                ..Default::default()
            },
        });

        let mut dispatcher = DispatcherBuilder::new()
            .with(UpdateSize, "UpdateSize", &[])
            .with(ActorUpdate, "ActorUpdate", &[])
            .with(ProcessEvents, "ProcessEvents", &["UpdateSize"])
            .with(
                CameraControllerSys::default(),
                "CameraControllerSys",
                &["ProcessEvents"],
            )
            .with(
                ViewUpdate::default(),
                "ViewUpdate",
                &["CameraControllerSys"],
            )
            .with(
                DataBufferUpdater::<CameraUniform>::default(),
                "DataBufferUpdater<CameraUniform>",
                &["ViewUpdate"],
            )
            .with(
                SpriteSelectorUpdate,
                "SpriteSelectorUpdate",
                &["CameraControllerSys"],
            )
            .with(
                DataBufferUpdater::<Indices>::default(),
                "DataBufferUpdater<Indices>",
                &["ProcessEvents"],
            )
            .with(
                ModelVertexUpdate::default(),
                "ModelVertexUpdate",
                &["SpriteSelectorUpdate"],
            )
            .with(
                DataBufferUpdater::<ModelVertex>::default(),
                "DataBufferUpdater<ModelVertex>",
                &["ModelVertexUpdate"],
            )
            .with(
                InstanceUniformUpdate::default(),
                "InstanceUniformUpdate",
                &["ProcessEvents"],
            )
            .with(
                DataBufferUpdater::<InstanceUniform>::default(),
                "DataBufferUpdater<InstanceUniform>",
                &["InstanceUniformUpdate"],
            )
            /*.with(
                LightUniformUpdate::default(),
                "LightUniformUpdate",
                &["ViewUpdate"],
            )*/
            .with(
                CameraUniformUpdate::default(),
                "CameraUniformUpdate",
                &["ViewUpdate"],
            )
            .with_thread_local(Rendering)
            .build();

        dispatcher.setup(&mut world);

        Self { world, dispatcher }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.world.write_resource::<ResizeValue>().0 = Some(new_size)
        }
    }

    pub fn input(&mut self, event: &DeviceEvent) {
        let mut event_input = self.world.write_resource::<EventInput>();

        match event {
            DeviceEvent::Key(KeyboardInput {
                scancode,
                state,
                virtual_keycode: Some(key),
                ..
            }) => {
                event_input.info.key_map.insert(*key, *state);
                event_input.info.keycode_map.insert(*scancode, *state);
            }
            DeviceEvent::MouseWheel { delta } => {
                event_input.info.scroll_delta = Some(*delta);
            }
            DeviceEvent::Button { button, state } => {
                event_input.info.mouse_map.insert(*button, *state);
            }
            DeviceEvent::MouseMotion { delta } => {
                event_input.info.motion_delta = *delta;
            }
            _ => {}
        }

        event_input.events.push(event.clone());
    }

    pub fn reset_input(&mut self) {
        let mut event_input = self.world.write_resource::<EventInput>();
        event_input.events.clear();
    }

    pub fn update(&mut self, dt: Duration) {
        let dt =
            Duration::from_secs_f64(dt.as_secs_f64() * self.world.read_resource::<Time>().speed);

        self.world.write_resource::<Time>().delta = dt;
        self.dispatcher.dispatch(&mut self.world);
        self.world.maintain();
    }
}
