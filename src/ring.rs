use std::mem;

use wgpu::util::DeviceExt;

use crate::{
    camera, hdr, instance, pipeline, sun,
    texture::{self, SetTextureContainer},
    vertex::Vertex,
};

pub struct Ring {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_elements: u32,
    render_pipeline: wgpu::RenderPipeline,
    texture_container: texture::TextureContainer,
    instance_buffer: wgpu::Buffer,
}

impl Ring {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        hdr: &hdr::HdrPipeline,
        camera_container: &camera::CameraContainer,
        sun: &sun::Sun,
    ) -> Self {
        let (vertices, indices) = Self::generate_ring_vertices(1.2, 2.5, 128);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sphere_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sphere_index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let texture_bytes = include_bytes!("../assets/textures/saturn_ring.png");
        let texture =
            texture::Texture::from_bytes(device, queue, texture_bytes, "saturn ring texture")
                .unwrap();
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let texture_bind_group: wgpu::BindGroup =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: Some("diffuse_bind_group"),
            });
        let texture_container =
            texture::TextureContainer::new(texture, texture_bind_group, texture_bind_group_layout);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_container.bind_group_layout,
                    &camera_container.camera_bind_group_layout,
                    &sun.light().bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let shader = wgpu::include_wgsl!("../shaders/ring.wgsl");
        let render_pipeline = pipeline::create_render_pipeline_without_culling(
            device,
            &render_pipeline_layout,
            hdr.format(),
            Some(texture::Texture::DEPTH_FORMAT),
            &[RingVertex::desc(), instance::InstanceRaw::desc()],
            wgpu::PrimitiveTopology::TriangleList,
            shader,
            Some("render_pipeline_ring"),
        );

        let instance = instance::Instance::default();

        let instance_data = vec![instance::InstanceRaw::from(&instance)];
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ring Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Ring {
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as _,
            texture_container,
            render_pipeline,
            instance_buffer,
        }
    }

    pub fn update_instance(&self, instance: &instance::Instance, queue: &wgpu::Queue) {
        let instance_data = vec![instance::InstanceRaw::from(instance)];
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );
    }

    fn generate_ring_vertices(
        inner_radius: f32,
        outer_radius: f32,
        segments: usize,
    ) -> (Vec<RingVertex>, Vec<u16>) {
        let mut vertices = Vec::with_capacity(segments * 2);
        let mut indices = Vec::with_capacity(segments * 6);

        let normal = [0.0, 1.0, 0.0];

        for i in 0..=segments {
            let theta = (i as f32) / (segments as f32) * std::f32::consts::TAU;
            let (sin, cos) = theta.sin_cos();

            // Inner vertex
            let inner_pos = [inner_radius * cos, 0.0, inner_radius * sin];
            let inner_uv = [0.0, i as f32 / segments as f32];
            vertices.push(RingVertex {
                position: inner_pos,
                normal,
                tex_coords: inner_uv,
            });

            // Outer vertex
            let outer_pos = [outer_radius * cos, 0.0, outer_radius * sin];
            let outer_uv = [1.0, i as f32 / segments as f32];
            vertices.push(RingVertex {
                position: outer_pos,
                normal,
                tex_coords: outer_uv,
            });
        }

        for i in 0..segments {
            let i0 = (i * 2) as u16;
            let i1 = i0 + 1;
            let i2 = i0 + 2;
            let i3 = i0 + 3;

            // First triangle
            indices.push(i0);
            indices.push(i2);
            indices.push(i1);

            // Second triangle
            indices.push(i1);
            indices.push(i2);
            indices.push(i3);
        }

        (vertices, indices)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RingVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for RingVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<RingVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
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
            ],
        }
    }
}

pub trait DrawRing<'a> {
    fn draw_ring(
        &mut self,
        ring: &'a Ring,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawRing<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_ring(
        &mut self,
        ring: &'b Ring,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_pipeline(&ring.render_pipeline);
        self.set_texture_container(&ring.texture_container);
        self.set_vertex_buffer(0, ring.vertex_buffer.slice(..));
        self.set_vertex_buffer(1, ring.instance_buffer.slice(..));
        self.set_index_buffer(ring.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, light_bind_group, &[]);
        self.draw_indexed(0..ring.num_elements, 0, 0..1);
    }
}
