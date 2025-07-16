use std::{mem, ops::Range};

use wgpu::util::DeviceExt;

pub struct Sphere {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_elements: u32,
}

impl Sphere {
    pub fn new(device: &wgpu::Device) -> Self {
        let (vertices, indices) = Self::generate_sphere_vertices(128, 128);

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

        Sphere {
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as _,
        }
    }

    fn generate_sphere_vertices(
        longitude_segments: u16,
        latitude_segments: u16,
    ) -> (Vec<SphereVertex>, Vec<u16>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for y in 0..=latitude_segments {
            let v = y as f32 / latitude_segments as f32;
            let theta = v * std::f32::consts::PI;

            for x in 0..=longitude_segments {
                let u = x as f32 / longitude_segments as f32;
                let phi = u * std::f32::consts::TAU;

                let position = [
                    theta.sin() * phi.cos(),
                    theta.cos(),
                    theta.sin() * phi.sin(),
                ];

                let tex_coords = [u, v];

                // For a unit sphere, the normal is the same as the position
                let normal = position;

                vertices.push(SphereVertex {
                    position,
                    tex_coords,
                    normal,
                });
            }
        }

        for y in 0..latitude_segments {
            for x in 0..longitude_segments {
                let i = y * (longitude_segments + 1) + x;
                let next = i + longitude_segments + 1;

                indices.push(i);
                indices.push(i + 1);
                indices.push(next);

                indices.push(i + 1);
                indices.push(next + 1);
                indices.push(next);
            }
        }

        (vertices, indices)
    }
}

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for SphereVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SphereVertex>() as wgpu::BufferAddress,
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

pub trait DrawSphere<'a> {
    fn draw_sphere_instanced(
        &mut self,
        sphere: &'a Sphere,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawSphere<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_sphere_instanced(
        &mut self,
        sphere: &'b Sphere,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, sphere.vertex_buffer.slice(..));
        self.set_index_buffer(sphere.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, light_bind_group, &[]);
        self.draw_indexed(0..sphere.num_elements, 0, instances);
    }
}
